use chrono::{Days, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// A recurring rule's schedule (DATA_MODEL §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub frequency: Frequency,
    /// Every *N* days, weeks, months or years: 1 or more.
    pub interval: u32,
    pub start: NaiveDate,
    /// The last day an occurrence may fall on, included.
    pub end: Option<NaiveDate>,
    /// How many dates the rule processes in its life, counted by `next`.
    pub max_occurrences: Option<u32>,
}

impl Schedule {
    /// The `n`-th date (0 is `start`), always counted from `start` so a 31st never drifts to the
    /// 28th (REC-2). `None` after `end`, past chrono's date range, or for an interval of 0, which
    /// would repeat `start` forever.
    pub fn occurrence(&self, n: u32) -> Option<NaiveDate> {
        if self.interval == 0 {
            return None;
        }
        let steps = n.checked_mul(self.interval)?;
        // checked_add_months puts a day past the month's end on its last day, so Feb 29 + 1 year
        // is Feb 28.
        let date = match self.frequency {
            Frequency::Daily => self.start.checked_add_days(Days::new(steps.into())),
            Frequency::Weekly => self.start.checked_add_days(Days::new(u64::from(steps) * 7)),
            Frequency::Monthly => self.start.checked_add_months(Months::new(steps)),
            Frequency::Yearly => self
                .start
                .checked_add_months(Months::new(steps.checked_mul(12)?)),
        }?;
        self.end.is_none_or(|end| date <= end).then_some(date)
    }

    /// The date at `index`, unless the rule has already processed `done` dates out of
    /// `max_occurrences`. The count is of processed dates, not indexes, so it holds when a
    /// schedule edit starts the index again and when a resume skips the paused dates (§4.1).
    pub fn next(&self, index: u32, done: u32) -> Option<NaiveDate> {
        if self.max_occurrences.is_some_and(|max| done >= max) {
            return None;
        }
        self.occurrence(index)
    }

    /// The first index from `index` on whose date is on or after `day`, or the index where the
    /// dates run out. Resuming a paused rule moves to it, so the paused dates are never created.
    pub fn first_on_or_after(&self, mut index: u32, day: NaiveDate) -> u32 {
        while self.occurrence(index).is_some_and(|date| date < day) {
            index += 1;
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn every(frequency: Frequency, interval: u32, start: NaiveDate) -> Schedule {
        Schedule {
            frequency,
            interval,
            start,
            end: None,
            max_occurrences: None,
        }
    }

    fn first(schedule: &Schedule, count: u32) -> Vec<NaiveDate> {
        (0..count).map_while(|n| schedule.occurrence(n)).collect()
    }

    #[test]
    fn a_monthly_31st_falls_on_each_month_end_and_never_drifts() {
        let rule = every(Frequency::Monthly, 1, d(2026, 1, 31));
        assert_eq!(
            first(&rule, 5),
            [
                d(2026, 1, 31),
                d(2026, 2, 28),
                d(2026, 3, 31),
                d(2026, 4, 30),
                d(2026, 5, 31)
            ]
        );
    }

    #[test]
    fn a_monthly_29th_and_30th_use_february_s_last_day() {
        let feb = |start| every(Frequency::Monthly, 1, start).occurrence(1);
        assert_eq!(feb(d(2026, 1, 29)), Some(d(2026, 2, 28)));
        assert_eq!(feb(d(2026, 1, 30)), Some(d(2026, 2, 28)));
        assert_eq!(feb(d(2028, 1, 30)), Some(d(2028, 2, 29)), "a leap year");
        assert_eq!(feb(d(2028, 1, 31)), Some(d(2028, 2, 29)));
    }

    #[test]
    fn every_3_months_steps_from_the_start() {
        let rule = every(Frequency::Monthly, 3, d(2026, 11, 30));
        assert_eq!(
            first(&rule, 4),
            [
                d(2026, 11, 30),
                d(2027, 2, 28),
                d(2027, 5, 30),
                d(2027, 8, 30)
            ]
        );
    }

    #[test]
    fn a_yearly_feb_29_is_feb_28_until_the_next_leap_year() {
        let rule = every(Frequency::Yearly, 1, d(2028, 2, 29));
        assert_eq!(
            first(&rule, 5),
            [
                d(2028, 2, 29),
                d(2029, 2, 28),
                d(2030, 2, 28),
                d(2031, 2, 28),
                d(2032, 2, 29)
            ]
        );
        let every_2 = every(Frequency::Yearly, 2, d(2028, 2, 29));
        assert_eq!(every_2.occurrence(1), Some(d(2030, 2, 28)));
        assert_eq!(every_2.occurrence(2), Some(d(2032, 2, 29)));
    }

    #[test]
    fn daily_and_weekly_rules_cross_month_and_year_ends() {
        let daily = every(Frequency::Daily, 2, d(2026, 12, 29));
        assert_eq!(
            first(&daily, 3),
            [d(2026, 12, 29), d(2026, 12, 31), d(2027, 1, 2)]
        );
        let weekly = every(Frequency::Weekly, 2, d(2026, 12, 21));
        assert_eq!(
            first(&weekly, 3),
            [d(2026, 12, 21), d(2027, 1, 4), d(2027, 1, 18)]
        );
        let leap = every(Frequency::Daily, 1, d(2028, 2, 28));
        assert_eq!(leap.occurrence(1), Some(d(2028, 2, 29)));
    }

    #[test]
    fn a_rule_ends_on_its_end_date_included() {
        let rule = Schedule {
            end: Some(d(2026, 3, 31)),
            ..every(Frequency::Monthly, 1, d(2026, 1, 31))
        };
        assert_eq!(
            first(&rule, 10),
            [d(2026, 1, 31), d(2026, 2, 28), d(2026, 3, 31)]
        );
        assert_eq!(rule.occurrence(3), None);
    }

    #[test]
    fn a_rule_ends_once_it_has_processed_its_number_of_times() {
        let rule = Schedule {
            max_occurrences: Some(2),
            ..every(Frequency::Weekly, 1, d(2026, 11, 2))
        };
        assert_eq!(rule.next(0, 0), Some(d(2026, 11, 2)));
        assert_eq!(rule.next(1, 1), Some(d(2026, 11, 9)));
        assert_eq!(rule.next(2, 2), None);
        // Started again from a later date after one was processed: one more to go.
        assert_eq!(rule.next(0, 1), Some(d(2026, 11, 2)));
        assert_eq!(rule.next(1, 2), None);
        assert_eq!(
            every(Frequency::Weekly, 1, d(2026, 11, 2)).next(500, 500),
            d(2026, 11, 2).checked_add_days(Days::new(3500)),
            "no number of times never ends"
        );
    }

    #[test]
    fn resuming_moves_to_the_first_date_on_or_after_the_day() {
        let rule = every(Frequency::Monthly, 1, d(2026, 1, 31));
        assert_eq!(rule.first_on_or_after(1, d(2026, 6, 15)), 5, "Jun 30");
        assert_eq!(
            rule.first_on_or_after(1, d(2026, 6, 30)),
            5,
            "the day itself"
        );
        assert_eq!(rule.first_on_or_after(5, d(2026, 2, 1)), 5, "never back");
        let ended = Schedule {
            end: Some(d(2026, 3, 31)),
            ..rule
        };
        assert_eq!(ended.first_on_or_after(1, d(2026, 6, 15)), 3);
        assert_eq!(ended.occurrence(3), None, "nothing left");
    }

    #[test]
    fn an_end_before_the_start_or_an_interval_of_0_gives_no_dates() {
        let ended = Schedule {
            end: Some(d(2026, 1, 30)),
            ..every(Frequency::Monthly, 1, d(2026, 1, 31))
        };
        assert_eq!(ended.occurrence(0), None);
        assert_eq!(
            every(Frequency::Daily, 0, d(2026, 1, 31)).occurrence(0),
            None
        );
    }

    #[test]
    fn a_date_past_chrono_s_range_is_none_instead_of_a_panic() {
        assert_eq!(
            every(Frequency::Yearly, 1, d(2026, 1, 1)).occurrence(u32::MAX),
            None
        );
        assert_eq!(
            every(Frequency::Daily, u32::MAX, d(2026, 1, 1)).occurrence(u32::MAX),
            None
        );
        assert_eq!(
            every(Frequency::Weekly, 1, NaiveDate::MAX).occurrence(1),
            None
        );
    }
}
