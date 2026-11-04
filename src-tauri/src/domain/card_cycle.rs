use chrono::{Datelike, Months, NaiveDate};

/// A credit card's current statement cycle and next payment due date (DATA_MODEL §5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardCycle {
    /// The day after the previous statement closing date.
    pub start: NaiveDate,
    /// The statement closing date: today or later.
    pub end: NaiveDate,
    /// The first due date on or after today.
    pub due: NaiveDate,
}

/// `statement_day` and `due_day` are 1–31; a shorter month uses its last day. `None` only for a
/// `today` within a month of either end of chrono's date range.
pub fn card_cycle(statement_day: u8, due_day: u8, today: NaiveDate) -> Option<CardCycle> {
    let end = on_or_after(today, statement_day)?;
    let previous_month = end.with_day(1)?.checked_sub_months(Months::new(1))?;
    Some(CardCycle {
        start: on_day(previous_month, statement_day)?.succ_opt()?,
        end,
        due: on_or_after(today, due_day)?,
    })
}

/// That day of `d`'s month, or the month's last day when it's shorter.
fn on_day(d: NaiveDate, day: u8) -> Option<NaiveDate> {
    // The accounts table only allows 1–31 (DATA_MODEL §3.1); clamping keeps a 0 from panicking.
    d.with_day(u32::from(day).clamp(1, d.num_days_in_month().into()))
}

/// The first date on or after `today` that falls on `day`, using the last day of a shorter month.
fn on_or_after(today: NaiveDate, day: u8) -> Option<NaiveDate> {
    let this_month = on_day(today, day)?;
    if this_month >= today {
        return Some(this_month);
    }
    on_day(today.with_day(1)?.checked_add_months(Months::new(1))?, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn cycle(statement: u8, due: u8, today: NaiveDate) -> (NaiveDate, NaiveDate, NaiveDate) {
        let c = card_cycle(statement, due, today).expect("a date inside chrono's range");
        (c.start, c.end, c.due)
    }

    #[test]
    fn the_cycle_runs_from_the_day_after_the_last_closing_to_the_next() {
        // UI_FLOWS card detail: statement closes Nov 20, payment due Nov 5.
        assert_eq!(
            cycle(20, 5, d(2026, 10, 26)),
            (d(2026, 10, 21), d(2026, 11, 20), d(2026, 11, 5))
        );
    }

    #[test]
    fn the_closing_day_itself_is_still_in_the_cycle() {
        assert_eq!(
            cycle(20, 5, d(2026, 10, 20)),
            (d(2026, 9, 21), d(2026, 10, 20), d(2026, 11, 5))
        );
    }

    #[test]
    fn the_due_day_itself_is_the_next_due_date() {
        assert_eq!(
            cycle(20, 5, d(2026, 11, 5)),
            (d(2026, 10, 21), d(2026, 11, 20), d(2026, 11, 5))
        );
    }

    #[test]
    fn a_cycle_crosses_the_year_end() {
        assert_eq!(
            cycle(20, 5, d(2026, 12, 26)),
            (d(2026, 12, 21), d(2027, 1, 20), d(2027, 1, 5))
        );
    }

    #[test]
    fn a_31st_statement_closes_on_the_last_day_of_short_months() {
        assert_eq!(
            cycle(31, 15, d(2026, 2, 10)),
            (d(2026, 2, 1), d(2026, 2, 28), d(2026, 2, 15))
        );
        assert_eq!(
            cycle(31, 15, d(2028, 2, 10)),
            (d(2028, 2, 1), d(2028, 2, 29), d(2028, 2, 15))
        );
        assert_eq!(
            cycle(31, 15, d(2026, 3, 1)),
            (d(2026, 3, 1), d(2026, 3, 31), d(2026, 3, 15))
        );
    }

    #[test]
    fn a_30th_statement_closes_on_feb_28_then_the_30th() {
        let (start, end, _) = cycle(30, 15, d(2026, 2, 15));
        assert_eq!((start, end), (d(2026, 1, 31), d(2026, 2, 28)));
        let (start, end, _) = cycle(30, 15, d(2026, 3, 1));
        assert_eq!((start, end), (d(2026, 3, 1), d(2026, 3, 30)));
    }

    #[test]
    fn a_due_day_past_the_month_end_falls_on_its_last_day() {
        assert_eq!(cycle(10, 31, d(2026, 4, 15)).2, d(2026, 4, 30));
        assert_eq!(cycle(10, 30, d(2026, 2, 1)).2, d(2026, 2, 28));
        assert_eq!(cycle(10, 30, d(2028, 2, 1)).2, d(2028, 2, 29));
        assert_eq!(cycle(10, 29, d(2026, 2, 1)).2, d(2026, 2, 28));
        assert_eq!(cycle(10, 31, d(2026, 5, 1)).2, d(2026, 5, 31));
    }

    #[test]
    fn a_passed_due_day_moves_to_next_month_even_into_a_short_one() {
        assert_eq!(cycle(10, 31, d(2026, 1, 31)).2, d(2026, 1, 31));
        assert_eq!(cycle(10, 30, d(2026, 1, 31)).2, d(2026, 2, 28));
    }

    #[test]
    fn a_date_at_the_calendar_limits_has_no_cycle_instead_of_panicking() {
        assert_eq!(card_cycle(20, 5, NaiveDate::MAX), None);
        assert_eq!(card_cycle(20, 5, NaiveDate::MIN), None);
    }
}
