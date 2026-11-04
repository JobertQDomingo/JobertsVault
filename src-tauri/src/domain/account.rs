use crate::domain::money::Money;

/// What a credit card's balance means for its limit (DATA_MODEL §5.1, CC-6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CardFigures {
    /// `max(0, −balance)`: a positive balance is credit on the card, not a debt.
    pub amount_owed: Money,
    /// `credit_limit − amount_owed`, negative once spending passes the limit.
    pub available_credit: Money,
    /// `amount_owed ÷ credit_limit` as a whole percent, rounded half up. Over 100 past the limit.
    pub utilization_percent: i64,
}

pub fn card_figures(balance: Money, credit_limit: Money) -> CardFigures {
    // In i128, so even a hand-edited i64::MIN balance can't overflow.
    let owed = (-i128::from(balance.0)).max(0);
    let limit = i128::from(credit_limit.0);
    // The table only allows a limit above 0 (DATA_MODEL §3.1); this keeps a 0 from panicking.
    let percent = if limit > 0 {
        (owed * 200 + limit) / (limit * 2)
    } else {
        0
    };
    CardFigures {
        amount_owed: Money(saturate(owed)),
        available_credit: Money(saturate(limit - owed)),
        utilization_percent: saturate(percent),
    }
}

fn saturate(value: i128) -> i64 {
    i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figures(balance: i64, limit: i64) -> (i64, i64, i64) {
        let f = card_figures(Money(balance), Money(limit));
        (f.amount_owed.0, f.available_credit.0, f.utilization_percent)
    }

    #[test]
    fn the_amount_owed_is_the_negative_balance() {
        // UI_FLOWS §5.3: ₱640.00 owed, 6% of a ₱10,000.00 limit, ₱9,360.00 available.
        assert_eq!(figures(-64_000, 1_000_000), (64_000, 936_000, 6));
        assert_eq!(figures(0, 1_000_000), (0, 1_000_000, 0));
    }

    #[test]
    fn an_overpaid_card_owes_nothing() {
        assert_eq!(figures(500, 1_000_000), (0, 1_000_000, 0));
    }

    #[test]
    fn spending_past_the_limit_is_shown_as_it_is() {
        assert_eq!(figures(-1_200_000, 1_000_000), (1_200_000, -200_000, 120));
    }

    #[test]
    fn utilization_rounds_half_up() {
        assert_eq!(figures(-5, 1_000).2, 1);
        assert_eq!(figures(-4, 1_000).2, 0);
        assert_eq!(figures(-995, 1_000).2, 100);
    }

    #[test]
    fn the_largest_values_do_not_overflow() {
        let f = card_figures(Money(i64::MIN + 1), Money(1));
        assert_eq!(f.amount_owed, Money(i64::MAX));
        assert!(f.available_credit < Money::ZERO);
        assert!(f.utilization_percent > 0);
    }
}
