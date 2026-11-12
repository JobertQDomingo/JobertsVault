use chrono::{Datelike, Months, NaiveDate};

use crate::domain::money::Money;

/// Calendar months from `today` to `target`, a part month counting as a whole one: Nov 12 →
/// Dec 12 is 1, Dec 13 is 2. 0 once the target date is today or has passed (DATA_MODEL §5.7).
pub fn months_left(today: NaiveDate, target: NaiveDate) -> i64 {
    if target <= today {
        return 0;
    }
    let whole = i64::from(target.year() - today.year()) * 12 + i64::from(target.month0())
        - i64::from(today.month0());
    // Adding months clamps to the month's end, so Jan 31 + 1 month is Feb 28.
    let reached = u32::try_from(whole)
        .ok()
        .and_then(|n| today.checked_add_months(Months::new(n)))
        .is_some_and(|date| date >= target);
    if reached { whole } else { whole + 1 }
}

/// What must be saved each month to cover `remaining` in `months`, rounded up to the minor
/// unit so saving it every month always reaches the target. None when nothing is left to save
/// or no month is (GOL-3).
pub fn monthly_needed(remaining: Money, months: i64) -> Option<Money> {
    // Divides first, so no amount can overflow on the way up.
    (remaining > Money::ZERO && months > 0)
        .then(|| Money(remaining.0 / months + i64::from(remaining.0 % months != 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn left(target: NaiveDate) -> i64 {
        months_left(d(2026, 11, 12), target)
    }

    #[test]
    fn a_part_month_counts_as_a_whole_one() {
        assert_eq!(left(d(2026, 11, 13)), 1);
        assert_eq!(left(d(2026, 11, 30)), 1);
        assert_eq!(left(d(2026, 12, 12)), 1);
        assert_eq!(left(d(2026, 12, 13)), 2);
        assert_eq!(left(d(2027, 5, 12)), 6);
        assert_eq!(left(d(2027, 5, 13)), 7);
    }

    #[test]
    fn a_target_today_or_past_leaves_no_months() {
        assert_eq!(left(d(2026, 11, 12)), 0);
        assert_eq!(left(d(2026, 11, 11)), 0);
        assert_eq!(left(d(2025, 1, 1)), 0);
    }

    #[test]
    fn month_ends_clamp_and_leap_years_count() {
        // Jan 31 + 1 month is Feb 28, so Feb 28 is reached in one month and Mar 1 needs two.
        assert_eq!(months_left(d(2026, 1, 31), d(2026, 2, 28)), 1);
        assert_eq!(months_left(d(2026, 1, 31), d(2026, 3, 1)), 2);
        assert_eq!(months_left(d(2028, 1, 31), d(2028, 2, 29)), 1);
        assert_eq!(months_left(d(2026, 11, 30), d(2026, 12, 1)), 1);
        assert_eq!(months_left(d(2026, 12, 31), d(2027, 1, 1)), 1);
    }

    #[test]
    fn the_monthly_amount_is_rounded_up() {
        // 2,600.00 over 6 months is 433.333…, so 433.34 a month (UI_FLOWS §8.4's example).
        assert_eq!(monthly_needed(Money(260_000), 6), Some(Money(43_334)));
        assert_eq!(monthly_needed(Money(260_000), 2), Some(Money(130_000)));
        assert_eq!(monthly_needed(Money(1), 12), Some(Money(1)));
        assert_eq!(monthly_needed(Money::MAX, 1), Some(Money::MAX));
        assert_eq!(
            monthly_needed(Money(i64::MAX), 2),
            Some(Money(i64::MAX / 2 + 1))
        );
    }

    #[test]
    fn nothing_left_or_no_months_has_no_monthly_amount() {
        assert_eq!(monthly_needed(Money::ZERO, 6), None);
        assert_eq!(monthly_needed(Money(500), 0), None);
    }
}
