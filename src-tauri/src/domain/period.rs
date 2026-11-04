use chrono::{Datelike, Days, NaiveDate, Weekday};

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

/// A closed date range that includes both ends (DATA_MODEL §5.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Period {
    pub kind: PeriodKind,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

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
                let start =
                    anchor - Days::new(anchor.weekday().days_since(settings.week_start).into());
                (start, start + Days::new(6))
            }
            PeriodKind::Month => month(anchor, settings.month_start_day),
            PeriodKind::Year => year(anchor),
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

    pub fn previous(&self, settings: &PeriodSettings) -> Period {
        self.step(false, settings)
    }

    /// `None` when the next period would start after `today` (DATA_MODEL §5.3).
    pub fn next(&self, settings: &PeriodSettings, today: NaiveDate) -> Option<Period> {
        let next = self.step(true, settings);
        (next.start <= today).then_some(next)
    }

    fn step(&self, forward: bool, settings: &PeriodSettings) -> Period {
        let outside = if forward {
            self.end + Days::new(1)
        } else {
            self.start - Days::new(1)
        };
        let (start, end) = match self.kind {
            // Months and years vary in length, so take the one around the next or previous day.
            PeriodKind::Month => month(outside, settings.month_start_day),
            PeriodKind::Year => year(outside),
            // Days, weeks and custom ranges move by their own length, counting both ends.
            PeriodKind::Day | PeriodKind::Week | PeriodKind::Custom => {
                let length = Days::new((self.end - self.start).num_days().unsigned_abs() + 1);
                if forward {
                    (self.start + length, self.end + length)
                } else {
                    (self.start - length, self.end - length)
                }
            }
        };
        Period {
            kind: self.kind,
            start,
            end,
        }
    }
}

/// The month that starts on day `start_day`: from that day in `d`'s month, or the month before
/// when `d` comes earlier, to the day before the next one starts.
fn month(d: NaiveDate, start_day: u32) -> (NaiveDate, NaiveDate) {
    let s = start_day.clamp(1, 28);
    let start = if d.day() >= s {
        d - Days::new((d.day() - s).into())
    } else {
        let last_of_previous = d - Days::new(d.day().into());
        last_of_previous - Days::new((last_of_previous.day() - s).into())
    };
    (
        start,
        start + Days::new(u64::from(start.num_days_in_month()) - 1),
    )
}

/// Always the calendar year, whatever `month_start_day` is.
fn year(d: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = d - Days::new(d.ordinal0().into());
    (
        start,
        start + Days::new(if start.leap_year() { 365 } else { 364 }),
    )
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
            span(day.previous(&MONDAY_1)),
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
            span(week.previous(&MONDAY_1)),
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
        assert_eq!(span(mar.previous(&MONDAY_1)), span(feb));
        assert_eq!(
            span(jan.previous(&MONDAY_1)),
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
            span(month.previous(&s25)),
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
            span(p.previous(&MONDAY_1)),
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
            span(p.previous(&MONDAY_1)),
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
            span(p.previous(&MONDAY_1)),
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
        let september = october.previous(&MONDAY_1);
        assert_eq!(september.next(&MONDAY_1, today), Some(october));
    }
}
