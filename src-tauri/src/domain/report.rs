use chrono::{Datelike, Months, NaiveDate};
use serde::Serialize;
use ts_rs::TS;

use crate::domain::money::Money;
use crate::domain::period::{Period, PeriodKind, PeriodSettings};

/// How much time each bar of the income-vs-expense chart covers (RPT-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Grain {
    Day,
    Week,
    Month,
}

/// The chart's bars for `period`, oldest first, covering it exactly (DATA_MODEL §5.9). Weeks follow
/// `week_start` and months are calendar months, so the first and last bar can be cut short by the
/// range. `None` past chrono's date range.
pub fn bars(
    period: &Period,
    settings: &PeriodSettings,
) -> Option<(Grain, Vec<(NaiveDate, NaiveDate)>)> {
    let (start, end) = (period.start(), period.end());
    let grain = match period.kind() {
        PeriodKind::Day | PeriodKind::Week | PeriodKind::Month => Grain::Day,
        PeriodKind::Year => Grain::Month,
        // Up to 31 days, counting both ends.
        PeriodKind::Custom if (end - start).num_days() < 31 => Grain::Day,
        PeriodKind::Custom if end < start.checked_add_months(Months::new(6))? => Grain::Week,
        PeriodKind::Custom => Grain::Month,
    };
    let mut bars = Vec::new();
    let mut from = start;
    loop {
        let to = match grain {
            Grain::Day => from,
            Grain::Week => Period::containing(PeriodKind::Week, from, settings)?.end(),
            Grain::Month => from.with_day(from.num_days_in_month().into())?,
        }
        .min(end);
        bars.push((from, to));
        if to == end {
            return Some((grain, bars));
        }
        from = to.succ_opt()?;
    }
}

/// How many categories the breakdown's donut shows before the rest become one Other slice
/// (DESIGN_SYSTEM §3.4).
pub const SLICES: usize = 8;

/// `part` as tenths of a percent of `total`, rounded half up: 1 of 3 is 333 (33.3%). 0 when `total`
/// is 0, so there's never a division by zero.
pub fn share(part: Money, total: Money) -> i64 {
    if total == Money::ZERO {
        return 0;
    }
    // i128, so part × 2000 can't overflow.
    let (part, total) = (i128::from(part.0), i128::from(total.0));
    i64::try_from((part * 2000 + total) / (2 * total)).unwrap_or(i64::MAX)
}

/// The donut's slices from categories ranked largest first: the first `SLICES`, then the rest
/// added up as one slice with no category. `None` if the rest is too large to add up.
pub fn slices(ranked: &[(i64, Money)]) -> Option<Vec<(Option<i64>, Money)>> {
    let (largest, rest) = ranked.split_at(ranked.len().min(SLICES));
    let mut slices: Vec<_> = largest
        .iter()
        .map(|&(id, amount)| (Some(id), amount))
        .collect();
    if !rest.is_empty() {
        let other = rest
            .iter()
            .try_fold(Money::ZERO, |sum, &(_, amount)| sum.checked_add(amount))?;
        slices.push((None, other));
    }
    Some(slices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn settings(week_start: Weekday, month_start_day: u32) -> PeriodSettings {
        PeriodSettings {
            week_start,
            month_start_day,
        }
    }

    const MONDAY_1: PeriodSettings = PeriodSettings {
        week_start: Weekday::Mon,
        month_start_day: 1,
    };

    fn of(
        kind: PeriodKind,
        anchor: NaiveDate,
        s: &PeriodSettings,
    ) -> (Grain, Vec<(NaiveDate, NaiveDate)>) {
        bars(&Period::containing(kind, anchor, s).unwrap(), s).unwrap()
    }

    fn custom(
        start: NaiveDate,
        end: NaiveDate,
        s: &PeriodSettings,
    ) -> (Grain, Vec<(NaiveDate, NaiveDate)>) {
        bars(&Period::custom(start, end).unwrap(), s).unwrap()
    }

    /// One bar per day from `start` to `end`.
    fn daily(start: NaiveDate, end: NaiveDate) -> Vec<(NaiveDate, NaiveDate)> {
        start
            .iter_days()
            .take_while(|day| *day <= end)
            .map(|day| (day, day))
            .collect()
    }

    #[test]
    fn a_day_is_one_bar() {
        let day = d(2026, 10, 26);
        assert_eq!(
            of(PeriodKind::Day, day, &MONDAY_1),
            (Grain::Day, vec![(day, day)])
        );
    }

    #[test]
    fn a_monday_week_is_seven_days() {
        let (grain, bars) = of(PeriodKind::Week, d(2026, 10, 28), &MONDAY_1);
        assert_eq!(grain, Grain::Day);
        assert_eq!(bars, daily(d(2026, 10, 26), d(2026, 11, 1)));
    }

    #[test]
    fn a_month_is_each_day_even_when_it_starts_on_the_25th() {
        let s25 = settings(Weekday::Mon, 25);
        let (grain, bars) = of(PeriodKind::Month, d(2026, 10, 10), &s25);
        assert_eq!(grain, Grain::Day);
        assert_eq!(bars, daily(d(2026, 9, 25), d(2026, 10, 24)));
        assert_eq!(bars.len(), 30);
    }

    #[test]
    fn february_has_a_bar_for_each_of_its_days() {
        assert_eq!(of(PeriodKind::Month, d(2026, 2, 10), &MONDAY_1).1.len(), 28);
        let leap = of(PeriodKind::Month, d(2028, 2, 10), &MONDAY_1).1;
        assert_eq!(leap.last(), Some(&(d(2028, 2, 29), d(2028, 2, 29))));
    }

    #[test]
    fn a_year_is_twelve_calendar_months_whatever_the_month_start_day() {
        for s in [MONDAY_1, settings(Weekday::Sun, 25)] {
            let (grain, bars) = of(PeriodKind::Year, d(2028, 6, 15), &s);
            assert_eq!(grain, Grain::Month);
            assert_eq!(bars.len(), 12);
            assert_eq!(bars[0], (d(2028, 1, 1), d(2028, 1, 31)));
            assert_eq!(bars[1], (d(2028, 2, 1), d(2028, 2, 29)));
            assert_eq!(bars[11], (d(2028, 12, 1), d(2028, 12, 31)));
        }
    }

    #[test]
    fn a_custom_range_up_to_31_days_is_by_day() {
        let (grain, bars) = custom(d(2026, 10, 1), d(2026, 10, 31), &MONDAY_1);
        assert_eq!(grain, Grain::Day);
        assert_eq!(bars, daily(d(2026, 10, 1), d(2026, 10, 31)));
        let one = d(2026, 10, 5);
        assert_eq!(custom(one, one, &MONDAY_1), (Grain::Day, vec![(one, one)]));
    }

    #[test]
    fn a_custom_range_of_32_days_is_by_monday_week_cut_to_the_range() {
        // Oct 1, 2026 is a Thursday.
        let (grain, bars) = custom(d(2026, 10, 1), d(2026, 11, 1), &MONDAY_1);
        assert_eq!(grain, Grain::Week);
        assert_eq!(
            bars,
            [
                (d(2026, 10, 1), d(2026, 10, 4)),
                (d(2026, 10, 5), d(2026, 10, 11)),
                (d(2026, 10, 12), d(2026, 10, 18)),
                (d(2026, 10, 19), d(2026, 10, 25)),
                (d(2026, 10, 26), d(2026, 11, 1)),
            ]
        );
    }

    #[test]
    fn custom_weeks_follow_the_week_start() {
        let sunday = settings(Weekday::Sun, 1);
        let bars = custom(d(2026, 10, 1), d(2026, 11, 2), &sunday).1;
        assert_eq!(bars[0], (d(2026, 10, 1), d(2026, 10, 3)));
        assert_eq!(bars[1], (d(2026, 10, 4), d(2026, 10, 10)));
        assert_eq!(bars.last(), Some(&(d(2026, 11, 1), d(2026, 11, 2))));
    }

    #[test]
    fn six_months_is_up_to_the_day_before_the_same_date_six_months_later() {
        assert_eq!(
            custom(d(2026, 1, 1), d(2026, 6, 30), &MONDAY_1).0,
            Grain::Week
        );
        assert_eq!(
            custom(d(2026, 1, 1), d(2026, 7, 1), &MONDAY_1).0,
            Grain::Month
        );
        // Six months after Aug 31 is Feb 28, the last day of that February.
        assert_eq!(
            custom(d(2026, 8, 31), d(2027, 2, 27), &MONDAY_1).0,
            Grain::Week
        );
        assert_eq!(
            custom(d(2026, 8, 31), d(2027, 2, 28), &MONDAY_1).0,
            Grain::Month
        );
    }

    #[test]
    fn a_longer_custom_range_is_by_calendar_month_cut_to_the_range() {
        let s25 = settings(Weekday::Mon, 25);
        let (grain, bars) = custom(d(2027, 10, 15), d(2028, 6, 10), &s25);
        assert_eq!(grain, Grain::Month);
        assert_eq!(bars.len(), 9);
        assert_eq!(bars[0], (d(2027, 10, 15), d(2027, 10, 31)));
        assert_eq!(bars[2], (d(2027, 12, 1), d(2027, 12, 31)));
        assert_eq!(bars[3], (d(2028, 1, 1), d(2028, 1, 31)));
        assert_eq!(bars[4], (d(2028, 2, 1), d(2028, 2, 29)));
        assert_eq!(bars[8], (d(2028, 6, 1), d(2028, 6, 10)));
    }

    #[test]
    fn a_share_is_in_tenths_of_a_percent_rounded_half_up() {
        assert_eq!(share(Money(1), Money(3)), 333);
        assert_eq!(share(Money(2), Money(3)), 667);
        assert_eq!(share(Money(1), Money(2000)), 1);
        assert_eq!(share(Money(1), Money(2001)), 0);
        assert_eq!(share(Money(5), Money(5)), 1000);
    }

    #[test]
    fn a_share_of_nothing_is_0() {
        assert_eq!(share(Money(0), Money(0)), 0);
        assert_eq!(share(Money(0), Money(7)), 0);
    }

    #[test]
    fn a_share_of_the_largest_amounts_does_not_overflow() {
        let total = Money(i64::MAX);
        assert_eq!(share(Money(i64::MAX / 2), total), 500);
    }

    fn ranked(n: i64) -> Vec<(i64, Money)> {
        (1..=n).map(|id| (id, Money(100 - id))).collect()
    }

    #[test]
    fn eight_categories_or_fewer_are_each_a_slice() {
        let eight = ranked(8);
        let slices = slices(&eight).unwrap();
        assert_eq!(slices.len(), 8);
        assert!(
            slices
                .iter()
                .zip(&eight)
                .all(|(s, r)| *s == (Some(r.0), r.1))
        );
        assert_eq!(super::slices(&[]).unwrap(), []);
    }

    #[test]
    fn past_eight_the_rest_add_up_to_one_other_slice() {
        let slices = slices(&ranked(11)).unwrap();
        assert_eq!(slices.len(), 9);
        assert_eq!(slices[7], (Some(8), Money(92)));
        // Categories 9, 10 and 11: 91 + 90 + 89.
        assert_eq!(slices[8], (None, Money(270)));
        assert_eq!(super::slices(&ranked(9)).unwrap()[8], (None, Money(91)));
    }

    #[test]
    fn bars_reach_the_last_supported_date_without_panicking() {
        let start = NaiveDate::MAX
            .checked_sub_days(chrono::Days::new(3))
            .unwrap();
        let bars = custom(start, NaiveDate::MAX, &MONDAY_1).1;
        assert_eq!(bars.last(), Some(&(NaiveDate::MAX, NaiveDate::MAX)));
    }
}
