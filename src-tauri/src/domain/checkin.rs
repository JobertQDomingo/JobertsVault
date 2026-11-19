//! Which check-in periods are due (CHK-1, CHK-5, DATA_MODEL §5.19).

use chrono::{Datelike, NaiveDate};

use super::period::{Period, PeriodKind, PeriodSettings};

/// The periods a check-in finished today would settle. Each is `None` when it isn't due.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DuePeriods {
    pub week: Option<Period>,
    pub month: Option<Period>,
    /// Only with the week or the month, and only in January.
    pub year: Option<Period>,
}

impl DuePeriods {
    /// The period the check-in shows: the month when it's due, else the week. `None` means no
    /// check-in is due.
    pub fn shown(&self) -> Option<Period> {
        self.month.or(self.week)
    }
}

/// Only the newest completed week, month and (in January) year can be due, each when it has no
/// row in `done` (kind and period start) and some transaction is dated on or before its end.
/// Older periods never reviewed are skipped.
pub fn due_periods(
    today: NaiveDate,
    settings: &PeriodSettings,
    done: &[(PeriodKind, NaiveDate)],
    earliest_transaction: Option<NaiveDate>,
) -> DuePeriods {
    let due = |kind| {
        Period::containing(kind, today, settings)
            .and_then(|current| current.previous(settings))
            .filter(|p| !done.contains(&(kind, p.start())))
            .filter(|p| earliest_transaction.is_some_and(|first| first <= p.end()))
    };
    let week = due(PeriodKind::Week);
    let month = due(PeriodKind::Month);
    // The year is never due by itself: it rides on a January check-in.
    let year = (today.month() == 1 && (week.is_some() || month.is_some()))
        .then(|| due(PeriodKind::Year))
        .flatten();
    DuePeriods { week, month, year }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn settings(week_start: Weekday, month_start_day: u32) -> PeriodSettings {
        PeriodSettings {
            week_start,
            month_start_day,
        }
    }

    const MONDAY_1ST: PeriodSettings = PeriodSettings {
        week_start: Weekday::Mon,
        month_start_day: 1,
    };

    fn span(p: Option<Period>) -> Option<(NaiveDate, NaiveDate)> {
        p.map(|p| (p.start(), p.end()))
    }

    const LONG_AGO: Option<NaiveDate> = NaiveDate::from_ymd_opt(2025, 1, 1);

    #[test]
    fn the_week_that_ended_most_recently_is_due_with_a_monday_start() {
        // Thu Nov 19: the last full week is Mon Nov 9 to Sun Nov 15.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 9), d(2026, 11, 15))));
        assert_eq!(span(due.month), Some((d(2026, 10, 1), d(2026, 10, 31))));
        assert_eq!(due.year, None);
    }

    #[test]
    fn on_the_week_start_the_week_that_ended_yesterday_is_due() {
        let due = due_periods(d(2026, 11, 16), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 9), d(2026, 11, 15))));
    }

    #[test]
    fn a_sunday_week_start_moves_the_week() {
        let due = due_periods(d(2026, 11, 19), &settings(Weekday::Sun, 1), &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 8), d(2026, 11, 14))));
    }

    #[test]
    fn a_month_starting_on_the_25th_ends_on_the_24th() {
        let s = settings(Weekday::Mon, 25);
        // Nov 19 is in Oct 25 to Nov 24, so Sep 25 to Oct 24 is the latest completed month.
        let due = due_periods(d(2026, 11, 19), &s, &[], LONG_AGO);
        assert_eq!(span(due.month), Some((d(2026, 9, 25), d(2026, 10, 24))));
        // On the 25th, the month that ended yesterday is due.
        let due = due_periods(d(2026, 11, 25), &s, &[], LONG_AGO);
        assert_eq!(span(due.month), Some((d(2026, 10, 25), d(2026, 11, 24))));
    }

    #[test]
    fn a_week_and_a_month_due_together_show_the_month() {
        // Mon Nov 2: the week Oct 26 to Nov 1 and October both just ended.
        let due = due_periods(d(2026, 11, 2), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 10, 26), d(2026, 11, 1))));
        assert_eq!(span(due.month), Some((d(2026, 10, 1), d(2026, 10, 31))));
        assert_eq!(span(due.shown()), span(due.month));
    }

    #[test]
    fn only_the_week_due_shows_the_week() {
        let done = [(PeriodKind::Month, d(2026, 10, 1))];
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(due.month, None);
        assert_eq!(span(due.shown()), Some((d(2026, 11, 9), d(2026, 11, 15))));
    }

    #[test]
    fn the_year_is_due_only_in_january() {
        let due = due_periods(d(2027, 1, 4), &MONDAY_1ST, &[], LONG_AGO);
        assert_eq!(span(due.year), Some((d(2026, 1, 1), d(2026, 12, 31))));
        assert_eq!(span(due.month), Some((d(2026, 12, 1), d(2026, 12, 31))));
        // Jan 31 still counts; Feb 1 doesn't.
        assert!(
            due_periods(d(2027, 1, 31), &MONDAY_1ST, &[], LONG_AGO)
                .year
                .is_some()
        );
        assert_eq!(
            due_periods(d(2027, 2, 1), &MONDAY_1ST, &[], LONG_AGO).year,
            None
        );
        assert_eq!(
            due_periods(d(2026, 12, 31), &MONDAY_1ST, &[], LONG_AGO).year,
            None
        );
    }

    #[test]
    fn the_year_is_never_due_by_itself() {
        // Wed Jan 13: the week of Jan 4 and December are done, the year isn't.
        let done = [
            (PeriodKind::Week, d(2027, 1, 4)),
            (PeriodKind::Month, d(2026, 12, 1)),
        ];
        let due = due_periods(d(2027, 1, 13), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(
            due,
            DuePeriods {
                week: None,
                month: None,
                year: None
            }
        );
        assert_eq!(due.shown(), None);
    }

    #[test]
    fn the_year_is_calendar_even_with_a_shifted_month() {
        // Jan 10 with months from the 25th: Nov 25 to Dec 24 is the latest month, 2026 the year.
        let due = due_periods(d(2027, 1, 10), &settings(Weekday::Mon, 25), &[], LONG_AGO);
        assert_eq!(span(due.month), Some((d(2026, 11, 25), d(2026, 12, 24))));
        assert_eq!(span(due.year), Some((d(2026, 1, 1), d(2026, 12, 31))));
    }

    #[test]
    fn a_year_already_checked_in_is_not_due_again() {
        let done = [(PeriodKind::Year, d(2026, 1, 1))];
        let due = due_periods(d(2027, 1, 13), &MONDAY_1ST, &done, LONG_AGO);
        assert!(due.week.is_some());
        assert_eq!(due.year, None);
    }

    #[test]
    fn three_weeks_unopened_leave_only_the_newest_due() {
        // The week of Oct 19 was the last one checked in; Oct 26, Nov 2 and Nov 9 never were.
        let done = [(PeriodKind::Week, d(2026, 10, 19))];
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(span(due.week), Some((d(2026, 11, 9), d(2026, 11, 15))));
    }

    #[test]
    fn a_period_that_already_has_a_row_is_not_due() {
        let done = [(PeriodKind::Week, d(2026, 11, 9))];
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &done, LONG_AGO);
        assert_eq!(due.week, None);
        assert!(due.month.is_some(), "each kind has its own row");
    }

    #[test]
    fn nothing_is_due_without_a_transaction() {
        let due = due_periods(d(2027, 1, 4), &MONDAY_1ST, &[], None);
        assert_eq!(
            due,
            DuePeriods {
                week: None,
                month: None,
                year: None
            }
        );
    }

    #[test]
    fn a_period_that_ended_before_the_first_transaction_is_not_due() {
        // The first transaction is Wed Nov 11: the week of Nov 9 is due, October isn't.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], Some(d(2026, 11, 11)));
        assert!(due.week.is_some());
        assert_eq!(due.month, None);
        // Dated on the period's last day still counts.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], Some(d(2026, 11, 15)));
        assert!(due.week.is_some());
        // Only a transaction this week: nothing is due yet.
        let due = due_periods(d(2026, 11, 19), &MONDAY_1ST, &[], Some(d(2026, 11, 16)));
        assert_eq!(due.shown(), None);
    }
}
