use chrono::{Datelike, Days, NaiveDate, TimeDelta, Weekday};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodKind {
    Day,
    Week,
    Month,
    Year,
    Custom,
}

/// The settings that decide where weeks and months start (DATA_MODEL §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeriodSettings {
    pub week_start: Weekday,
    /// 1–28, so every month has that day.
    pub month_start_day: u32,
}

/// A closed date range that includes both ends (DATA_MODEL §5.3). The fields are private so
/// `start <= end` always holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Period {
    kind: PeriodKind,
    start: NaiveDate,
    end: NaiveDate,
}

/// Every function here is `None` instead of panicking for a date past chrono's range.
impl Period {
    /// The period of `kind` that contains `anchor`. Custom has no anchor, so it's `None`.
    pub fn containing(
        kind: PeriodKind,
        anchor: NaiveDate,
        settings: &PeriodSettings,
    ) -> Option<Period> {
        let (start, end) = match kind {
            PeriodKind::Day => (anchor, anchor),
            PeriodKind::Week => {
                let back = anchor.weekday().days_since(settings.week_start);
                let start = anchor.checked_sub_days(Days::new(back.into()))?;
                (start, start.checked_add_days(Days::new(6))?)
            }
            PeriodKind::Month => month(anchor, settings.month_start_day)?,
            PeriodKind::Year => year(anchor)?,
            PeriodKind::Custom => return None,
        };
        Some(Period { kind, start, end })
    }

    /// `None` when `end` is before `start`.
    pub fn custom(start: NaiveDate, end: NaiveDate) -> Option<Period> {
        (start <= end).then_some(Period {
            kind: PeriodKind::Custom,
            start,
            end,
        })
    }

    pub fn kind(&self) -> PeriodKind {
        self.kind
    }

    pub fn start(&self) -> NaiveDate {
        self.start
    }

    pub fn end(&self) -> NaiveDate {
        self.end
    }

    pub fn previous(&self, settings: &PeriodSettings) -> Option<Period> {
        self.step(false, settings)
    }

    /// `None` when the next period would start after `today` (DATA_MODEL §5.3).
    pub fn next(&self, settings: &PeriodSettings, today: NaiveDate) -> Option<Period> {
        self.step(true, settings).filter(|next| next.start <= today)
    }

    /// The period in the same sequence as this one that contains `date`, for the Today button.
    /// A custom range moves by whole range lengths, so it lands where `next` stops.
    pub fn in_sequence(&self, date: NaiveDate, settings: &PeriodSettings) -> Option<Period> {
        if self.kind != PeriodKind::Custom {
            return Period::containing(self.kind, date, settings);
        }
        let length = (self.end - self.start).num_days() + 1;
        let steps = (date - self.start).num_days().div_euclid(length);
        let start = self
            .start
            .checked_add_signed(TimeDelta::try_days(steps.checked_mul(length)?)?)?;
        Period::custom(
            start,
            start.checked_add_days(Days::new(length.unsigned_abs() - 1))?,
        )
    }

    fn step(&self, forward: bool, settings: &PeriodSettings) -> Option<Period> {
        let shift = |d: NaiveDate, days: Days| {
            if forward {
                d.checked_add_days(days)
            } else {
                d.checked_sub_days(days)
            }
        };
        let outside = shift(if forward { self.end } else { self.start }, Days::new(1))?;
        let (start, end) = match self.kind {
            // Months and years vary in length, so take the one around the next or previous day.
            PeriodKind::Month => month(outside, settings.month_start_day)?,
            PeriodKind::Year => year(outside)?,
            // Days, weeks and custom ranges move by their own length, counting both ends.
            PeriodKind::Day | PeriodKind::Week | PeriodKind::Custom => {
                let length = Days::new((self.end - self.start).num_days().unsigned_abs() + 1);
                (shift(self.start, length)?, shift(self.end, length)?)
            }
        };
        Some(Period {
            kind: self.kind,
            start,
            end,
        })
    }
}

/// What a period still in progress is compared with (RPT-4a, SET-7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ComparisonMode {
    /// The same number of days at the start of the previous period.
    ToDate,
    /// The whole previous period.
    Full,
}

/// The dates `current` is compared with (DATA_MODEL §5.4): the previous period, cut to the same
/// number of days when `current` is still in progress and `mode` is `ToDate`, but never past the
/// previous period's end. `None` past chrono's date range.
pub fn comparison_window(
    current: &Period,
    settings: &PeriodSettings,
    today: NaiveDate,
    mode: ComparisonMode,
) -> Option<Period> {
    let previous = current.previous(settings)?;
    let in_progress = (current.start..=current.end).contains(&today);
    if mode == ComparisonMode::Full || !in_progress {
        return Some(previous);
    }
    let elapsed = Days::new((today - current.start).num_days().unsigned_abs());
    let end = previous.start.checked_add_days(elapsed)?.min(previous.end);
    Period::custom(previous.start, end)
}

/// The month that starts on day `start_day`: from that day in `d`'s month, or the month before
/// when `d` comes earlier, to the day before the next one starts.
fn month(d: NaiveDate, start_day: u32) -> Option<(NaiveDate, NaiveDate)> {
    // Settings only allow 1–28 (DATA_MODEL §3.5); clamping keeps a bad value from panicking.
    let s = start_day.clamp(1, 28);
    let start = if d.day() >= s {
        d.with_day(s)?
    } else {
        d.checked_sub_days(Days::new(d.day().into()))?.with_day(s)?
    };
    let end = start.checked_add_days(Days::new(u64::from(start.num_days_in_month()) - 1))?;
    Some((start, end))
}

/// Always the calendar year, whatever `month_start_day` is.
fn year(d: NaiveDate) -> Option<(NaiveDate, NaiveDate)> {
    Some((
        NaiveDate::from_ymd_opt(d.year(), 1, 1)?,
        NaiveDate::from_ymd_opt(d.year(), 12, 31)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A far-future today, so `next` is never cut off unless a test means it to be.
    fn far() -> NaiveDate {
        d(2099, 12, 31)
    }

    fn range(kind: PeriodKind, anchor: NaiveDate, s: &PeriodSettings) -> (NaiveDate, NaiveDate) {
        let p = Period::containing(kind, anchor, s).expect("not custom");
        assert_eq!(p.kind, kind);
        (p.start, p.end)
    }

    fn span(p: Period) -> (NaiveDate, NaiveDate) {
        (p.start, p.end)
    }

    #[test]
    fn a_day_is_its_own_date() {
        let day = Period::containing(PeriodKind::Day, d(2026, 10, 26), &MONDAY_1).unwrap();
        assert_eq!(span(day), (d(2026, 10, 26), d(2026, 10, 26)));
        assert_eq!(
            span(day.previous(&MONDAY_1).unwrap()),
            (d(2026, 10, 25), d(2026, 10, 25))
        );
        let next = day.next(&MONDAY_1, d(2026, 10, 27)).unwrap();
        assert_eq!(span(next), (d(2026, 10, 27), d(2026, 10, 27)));
        assert_eq!(day.next(&MONDAY_1, d(2026, 10, 26)), None);
    }

    #[test]
    fn a_week_starts_on_monday_by_default() {
        let week = (d(2026, 10, 26), d(2026, 11, 1));
        assert_eq!(range(PeriodKind::Week, d(2026, 10, 26), &MONDAY_1), week);
        assert_eq!(range(PeriodKind::Week, d(2026, 11, 1), &MONDAY_1), week);
    }

    #[test]
    fn a_week_can_start_on_other_days() {
        let sunday = settings(Weekday::Sun, 1);
        let saturday = settings(Weekday::Sat, 1);
        assert_eq!(
            range(PeriodKind::Week, d(2026, 10, 26), &sunday),
            (d(2026, 10, 25), d(2026, 10, 31))
        );
        assert_eq!(
            range(PeriodKind::Week, d(2026, 10, 26), &saturday),
            (d(2026, 10, 24), d(2026, 10, 30))
        );
    }

    #[test]
    fn a_week_crosses_the_year_boundary() {
        assert_eq!(
            range(PeriodKind::Week, d(2026, 1, 1), &MONDAY_1),
            (d(2025, 12, 29), d(2026, 1, 4))
        );
    }

    #[test]
    fn weeks_move_by_seven_days() {
        let week = Period::containing(PeriodKind::Week, d(2026, 10, 26), &MONDAY_1).unwrap();
        assert_eq!(
            span(week.previous(&MONDAY_1).unwrap()),
            (d(2026, 10, 19), d(2026, 10, 25))
        );
        assert_eq!(
            span(week.next(&MONDAY_1, far()).unwrap()),
            (d(2026, 11, 2), d(2026, 11, 8))
        );
    }

    #[test]
    fn a_calendar_month_ends_on_its_last_day() {
        assert_eq!(
            range(PeriodKind::Month, d(2026, 2, 15), &MONDAY_1),
            (d(2026, 2, 1), d(2026, 2, 28))
        );
        assert_eq!(
            range(PeriodKind::Month, d(2028, 2, 10), &MONDAY_1),
            (d(2028, 2, 1), d(2028, 2, 29))
        );
    }

    #[test]
    fn calendar_months_move_across_month_ends() {
        let jan = Period::containing(PeriodKind::Month, d(2026, 1, 31), &MONDAY_1).unwrap();
        let feb = jan.next(&MONDAY_1, d(2026, 12, 31)).unwrap();
        assert_eq!(span(feb), (d(2026, 2, 1), d(2026, 2, 28)));
        let mar = feb.next(&MONDAY_1, d(2026, 12, 31)).unwrap();
        assert_eq!(span(mar), (d(2026, 3, 1), d(2026, 3, 31)));
        assert_eq!(span(mar.previous(&MONDAY_1).unwrap()), span(feb));
        assert_eq!(
            span(jan.previous(&MONDAY_1).unwrap()),
            (d(2025, 12, 1), d(2025, 12, 31))
        );
    }

    #[test]
    fn a_month_can_start_on_the_25th() {
        let s25 = settings(Weekday::Mon, 25);
        assert_eq!(
            range(PeriodKind::Month, d(2026, 10, 10), &s25),
            (d(2026, 9, 25), d(2026, 10, 24))
        );
        assert_eq!(
            range(PeriodKind::Month, d(2026, 10, 25), &s25),
            (d(2026, 10, 25), d(2026, 11, 24))
        );
        assert_eq!(
            range(PeriodKind::Month, d(2026, 10, 24), &s25),
            (d(2026, 9, 25), d(2026, 10, 24))
        );
        assert_eq!(
            range(PeriodKind::Month, d(2026, 1, 10), &s25),
            (d(2025, 12, 25), d(2026, 1, 24))
        );
    }

    #[test]
    fn a_month_can_start_on_the_28th() {
        let s28 = settings(Weekday::Mon, 28);
        assert_eq!(
            range(PeriodKind::Month, d(2026, 3, 1), &s28),
            (d(2026, 2, 28), d(2026, 3, 27))
        );
        assert_eq!(
            range(PeriodKind::Month, d(2026, 2, 27), &s28),
            (d(2026, 1, 28), d(2026, 2, 27))
        );
        assert_eq!(
            range(PeriodKind::Month, d(2028, 2, 29), &s28),
            (d(2028, 2, 28), d(2028, 3, 27))
        );
    }

    #[test]
    fn shifted_months_move_by_one_month() {
        let s25 = settings(Weekday::Mon, 25);
        let month = Period::containing(PeriodKind::Month, d(2026, 1, 10), &s25).unwrap();
        assert_eq!(
            span(month.previous(&s25).unwrap()),
            (d(2025, 11, 25), d(2025, 12, 24))
        );
        assert_eq!(
            span(month.next(&s25, far()).unwrap()),
            (d(2026, 1, 25), d(2026, 2, 24))
        );
    }

    #[test]
    fn a_year_is_the_calendar_year() {
        let year = (d(2026, 1, 1), d(2026, 12, 31));
        assert_eq!(range(PeriodKind::Year, d(2026, 6, 15), &MONDAY_1), year);
        assert_eq!(
            range(
                PeriodKind::Year,
                d(2026, 6, 15),
                &settings(Weekday::Sun, 25)
            ),
            year
        );
        let p = Period::containing(PeriodKind::Year, d(2026, 6, 15), &MONDAY_1).unwrap();
        assert_eq!(
            span(p.previous(&MONDAY_1).unwrap()),
            (d(2025, 1, 1), d(2025, 12, 31))
        );
        let leap = p
            .next(&MONDAY_1, far())
            .unwrap()
            .next(&MONDAY_1, far())
            .unwrap();
        assert_eq!(span(leap), (d(2028, 1, 1), d(2028, 12, 31)));
    }

    #[test]
    fn a_custom_range_moves_by_its_length_counting_both_ends() {
        let p = Period::custom(d(2026, 10, 1), d(2026, 10, 10)).unwrap();
        assert_eq!(p.kind, PeriodKind::Custom);
        assert_eq!(
            span(p.previous(&MONDAY_1).unwrap()),
            (d(2026, 9, 21), d(2026, 9, 30))
        );
        let next = p.next(&MONDAY_1, d(2026, 10, 11)).unwrap();
        assert_eq!(span(next), (d(2026, 10, 11), d(2026, 10, 20)));
        assert_eq!(p.next(&MONDAY_1, d(2026, 10, 10)), None);
    }

    #[test]
    fn a_one_day_custom_range_moves_by_one_day() {
        let p = Period::custom(d(2026, 10, 5), d(2026, 10, 5)).unwrap();
        assert_eq!(
            span(p.previous(&MONDAY_1).unwrap()),
            (d(2026, 10, 4), d(2026, 10, 4))
        );
    }

    #[test]
    fn a_custom_range_crosses_a_leap_day() {
        let p = Period::custom(d(2028, 2, 28), d(2028, 3, 1)).unwrap();
        assert_eq!(
            span(p.next(&MONDAY_1, far()).unwrap()),
            (d(2028, 3, 2), d(2028, 3, 4))
        );
    }

    #[test]
    fn a_custom_range_must_not_end_before_it_starts() {
        assert_eq!(Period::custom(d(2026, 10, 10), d(2026, 10, 1)), None);
    }

    #[test]
    fn custom_has_no_anchored_period() {
        assert_eq!(
            Period::containing(PeriodKind::Custom, d(2026, 10, 26), &MONDAY_1),
            None
        );
    }

    #[test]
    fn there_is_no_next_period_after_the_current_one() {
        let today = d(2026, 10, 26);
        let october = Period::containing(PeriodKind::Month, today, &MONDAY_1).unwrap();
        assert_eq!(october.next(&MONDAY_1, today), None);
        let september = october.previous(&MONDAY_1).unwrap();
        assert_eq!(september.next(&MONDAY_1, today), Some(october));
    }

    #[test]
    fn today_finds_the_custom_range_that_contains_it() {
        let p = Period::custom(d(2026, 10, 1), d(2026, 10, 10)).unwrap();
        let at = |date| span(p.in_sequence(date, &MONDAY_1).unwrap());
        assert_eq!(at(d(2026, 10, 26)), (d(2026, 10, 21), d(2026, 10, 30)));
        assert_eq!(at(d(2026, 10, 10)), (d(2026, 10, 1), d(2026, 10, 10)));
        assert_eq!(at(d(2026, 10, 11)), (d(2026, 10, 11), d(2026, 10, 20)));
        assert_eq!(
            p.in_sequence(d(2026, 10, 26), &MONDAY_1).unwrap().kind,
            PeriodKind::Custom
        );
    }

    #[test]
    fn today_moves_a_future_custom_range_back() {
        let p = Period::custom(d(2026, 11, 1), d(2026, 11, 10)).unwrap();
        assert_eq!(
            span(p.in_sequence(d(2026, 10, 26), &MONDAY_1).unwrap()),
            (d(2026, 10, 22), d(2026, 10, 31))
        );
    }

    #[test]
    fn today_is_where_next_stops() {
        let today = d(2026, 10, 26);
        let mut p = Period::custom(d(2026, 10, 1), d(2026, 10, 10)).unwrap();
        while let Some(next) = p.next(&MONDAY_1, today) {
            p = next;
        }
        assert_eq!(p.in_sequence(today, &MONDAY_1), Some(p));
    }

    #[test]
    fn today_for_anchored_kinds_is_the_containing_period() {
        let s25 = settings(Weekday::Mon, 25);
        let old = Period::containing(PeriodKind::Month, d(2025, 3, 1), &s25).unwrap();
        assert_eq!(
            span(old.in_sequence(d(2026, 10, 10), &s25).unwrap()),
            (d(2026, 9, 25), d(2026, 10, 24))
        );
    }

    #[test]
    fn in_sequence_at_the_calendar_limit_is_none() {
        // Oct 1–10's range around MAX ends on MAX exactly; a day later, it would end past it.
        let fits = Period::custom(d(2026, 10, 1), d(2026, 10, 10)).unwrap();
        assert_eq!(
            fits.in_sequence(NaiveDate::MAX, &MONDAY_1).unwrap().end,
            NaiveDate::MAX
        );
        let past = Period::custom(d(2026, 10, 2), d(2026, 10, 11)).unwrap();
        assert_eq!(past.in_sequence(NaiveDate::MAX, &MONDAY_1), None);
    }

    #[test]
    fn a_sunday_start_week_anchored_on_sunday_starts_that_day() {
        let sunday = settings(Weekday::Sun, 1);
        assert_eq!(
            range(PeriodKind::Week, d(2026, 10, 25), &sunday),
            (d(2026, 10, 25), d(2026, 10, 31))
        );
    }

    #[test]
    fn weeks_move_across_the_year_end() {
        let week = Period::containing(PeriodKind::Week, d(2025, 12, 22), &MONDAY_1).unwrap();
        assert_eq!(
            span(week.next(&MONDAY_1, far()).unwrap()),
            (d(2025, 12, 29), d(2026, 1, 4))
        );
    }

    #[test]
    fn a_28th_start_month_moves_across_a_leap_february() {
        let s28 = settings(Weekday::Mon, 28);
        let jan = Period::containing(PeriodKind::Month, d(2028, 2, 10), &s28).unwrap();
        assert_eq!(span(jan), (d(2028, 1, 28), d(2028, 2, 27)));
        let feb = jan.next(&s28, far()).unwrap();
        assert_eq!(span(feb), (d(2028, 2, 28), d(2028, 3, 27)));
        assert_eq!(span(feb.previous(&s28).unwrap()), span(jan));
    }

    #[test]
    fn a_shifted_month_has_no_next_before_its_start_day() {
        let s25 = settings(Weekday::Mon, 25);
        let month = Period::containing(PeriodKind::Month, d(2026, 10, 10), &s25).unwrap();
        assert_eq!(month.next(&s25, d(2026, 10, 24)), None);
        assert!(month.next(&s25, d(2026, 10, 25)).is_some());
    }

    /// The comparison window of the `kind` period containing `today`, or of the one containing
    /// `anchor` when given.
    fn compare(
        kind: PeriodKind,
        anchor: NaiveDate,
        today: NaiveDate,
        s: &PeriodSettings,
        mode: ComparisonMode,
    ) -> (NaiveDate, NaiveDate) {
        let current = Period::containing(kind, anchor, s).unwrap();
        span(comparison_window(&current, s, today, mode).unwrap())
    }

    use ComparisonMode::{Full, ToDate};

    #[test]
    fn a_month_in_progress_is_compared_with_the_same_days_last_month() {
        let today = d(2026, 10, 26);
        assert_eq!(
            compare(PeriodKind::Month, today, today, &MONDAY_1, ToDate),
            (d(2026, 9, 1), d(2026, 9, 26))
        );
        assert_eq!(
            compare(PeriodKind::Month, today, today, &MONDAY_1, Full),
            (d(2026, 9, 1), d(2026, 9, 30))
        );
    }

    #[test]
    fn the_comparison_stops_at_the_end_of_a_shorter_previous_month() {
        let mar31 = d(2026, 3, 31);
        assert_eq!(
            compare(PeriodKind::Month, mar31, mar31, &MONDAY_1, ToDate),
            (d(2026, 2, 1), d(2026, 2, 28))
        );
        let leap = d(2028, 3, 30);
        assert_eq!(
            compare(PeriodKind::Month, leap, leap, &MONDAY_1, ToDate),
            (d(2028, 2, 1), d(2028, 2, 29))
        );
    }

    #[test]
    fn a_month_starting_on_the_25th_compares_the_same_days_from_the_25th() {
        let s25 = settings(Weekday::Mon, 25);
        let today = d(2026, 11, 3);
        // Oct 25 – Nov 24 is 9 days in, so Sep 25 + 9 days.
        assert_eq!(
            compare(PeriodKind::Month, today, today, &s25, ToDate),
            (d(2026, 9, 25), d(2026, 10, 4))
        );
    }

    #[test]
    fn a_monday_week_in_progress_is_compared_with_last_monday_to_the_same_weekday() {
        let wednesday = d(2026, 10, 28);
        assert_eq!(
            compare(PeriodKind::Week, wednesday, wednesday, &MONDAY_1, ToDate),
            (d(2026, 10, 19), d(2026, 10, 21))
        );
        let sunday = settings(Weekday::Sun, 1);
        assert_eq!(
            compare(PeriodKind::Week, wednesday, wednesday, &sunday, ToDate),
            (d(2026, 10, 18), d(2026, 10, 21))
        );
    }

    #[test]
    fn a_year_in_progress_counts_the_same_number_of_days_across_a_leap_day() {
        // Mar 1, 2029 is day 60, and day 60 of 2028 is Feb 29.
        let today = d(2029, 3, 1);
        assert_eq!(
            compare(PeriodKind::Year, today, today, &MONDAY_1, ToDate),
            (d(2028, 1, 1), d(2028, 2, 29))
        );
        assert_eq!(
            compare(PeriodKind::Year, today, today, &MONDAY_1, Full),
            (d(2028, 1, 1), d(2028, 12, 31))
        );
    }

    #[test]
    fn today_is_compared_with_yesterday() {
        let today = d(2026, 11, 3);
        let yesterday = d(2026, 11, 2);
        assert_eq!(
            compare(PeriodKind::Day, today, today, &MONDAY_1, ToDate),
            (yesterday, yesterday)
        );
    }

    #[test]
    fn a_period_that_is_over_is_always_compared_in_full() {
        let today = d(2026, 11, 3);
        assert_eq!(
            compare(PeriodKind::Month, d(2026, 10, 5), today, &MONDAY_1, ToDate),
            (d(2026, 9, 1), d(2026, 9, 30))
        );
    }

    #[test]
    fn the_first_day_of_a_period_compares_one_day() {
        let first = d(2026, 11, 1);
        assert_eq!(
            compare(PeriodKind::Month, first, first, &MONDAY_1, ToDate),
            (d(2026, 10, 1), d(2026, 10, 1))
        );
    }

    #[test]
    fn a_custom_range_is_compared_with_the_same_length_before_it() {
        let today = d(2026, 11, 3);
        let past = Period::custom(d(2026, 10, 1), d(2026, 10, 10)).unwrap();
        let window = comparison_window(&past, &MONDAY_1, today, ToDate).unwrap();
        assert_eq!(span(window), (d(2026, 9, 21), d(2026, 9, 30)));
        // The range holding today counts the same days so far: Oct 31 – Nov 3 is 4 days.
        let current = Period::custom(d(2026, 10, 31), d(2026, 11, 9)).unwrap();
        let window = comparison_window(&current, &MONDAY_1, today, ToDate).unwrap();
        assert_eq!(span(window), (d(2026, 10, 21), d(2026, 10, 24)));
    }

    #[test]
    fn dates_at_the_calendar_limits_have_no_period_instead_of_panicking() {
        let s25 = settings(Weekday::Mon, 25);
        let first = Period::containing(PeriodKind::Day, NaiveDate::MIN, &MONDAY_1).unwrap();
        assert_eq!(first.previous(&MONDAY_1), None);
        let last = Period::containing(PeriodKind::Day, NaiveDate::MAX, &MONDAY_1).unwrap();
        assert_eq!(last.next(&MONDAY_1, NaiveDate::MAX), None);
        assert_eq!(
            Period::containing(PeriodKind::Month, NaiveDate::MIN, &s25),
            None
        );
        let year = Period::containing(PeriodKind::Year, NaiveDate::MAX, &MONDAY_1).unwrap();
        assert_eq!(year.next(&MONDAY_1, NaiveDate::MAX), None);
    }
}
