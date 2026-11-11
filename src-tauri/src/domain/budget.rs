use crate::domain::money::Money;

/// How much of `amount` is used: whole percent rounded down, so the bar turns amber only once
/// `spent` is really 80% of it, and red only once it's all spent (BUD-3, DATA_MODEL §5.6).
pub fn used_percent(spent: Money, amount: Money) -> i64 {
    // In i128, so no total can overflow; the store keeps `amount` above 0.
    let percent = i128::from(spent.0) * 100 / i128::from(amount.0.max(1));
    i64::try_from(percent).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pct(spent: i64, amount: i64) -> i64 {
        used_percent(Money(spent), Money(amount))
    }

    #[test]
    fn nothing_spent_is_0() {
        assert_eq!(pct(0, 40_000), 0);
    }

    #[test]
    fn the_thresholds_are_reached_only_when_exactly_met() {
        assert_eq!(pct(31_999, 40_000), 79);
        assert_eq!(pct(32_000, 40_000), 80);
        assert_eq!(pct(39_999, 40_000), 99);
        assert_eq!(pct(40_000, 40_000), 100);
    }

    #[test]
    fn over_the_limit_goes_past_100() {
        assert_eq!(pct(18_000, 15_000), 120);
    }

    #[test]
    fn huge_totals_dont_overflow() {
        assert_eq!(pct(i64::MAX, 1), i64::MAX);
        assert_eq!(pct(Money::MAX.0, Money::MAX.0), 100);
    }
}
