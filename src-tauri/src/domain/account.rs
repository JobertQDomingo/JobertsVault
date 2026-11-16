use serde::Serialize;
use ts_rs::TS;

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

/// How far a card's spending this statement cycle has gone (CC-10, DATA_MODEL §5.18), in rank
/// order, so a level reached for the first time compares higher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SpendLevel {
    None,
    Soft,
    Mid,
    Hard,
}

/// A card's three levels, `0 < soft < mid < hard`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpendLevels {
    pub soft: Money,
    pub mid: Money,
    pub hard: Money,
}

/// The highest level `spent` has reached: equal counts as reached.
pub fn spend_level(spent: Money, levels: SpendLevels) -> SpendLevel {
    if spent >= levels.hard {
        SpendLevel::Hard
    } else if spent >= levels.mid {
        SpendLevel::Mid
    } else if spent >= levels.soft {
        SpendLevel::Soft
    } else {
        SpendLevel::None
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

    #[test]
    fn a_spending_level_is_reached_at_exactly_its_amount() {
        let levels = SpendLevels {
            soft: Money(300_000),
            mid: Money(400_000),
            hard: Money(500_000),
        };
        let at = |spent| spend_level(Money(spent), levels);
        assert_eq!(at(0), SpendLevel::None);
        assert_eq!(at(299_999), SpendLevel::None);
        assert_eq!(at(300_000), SpendLevel::Soft);
        assert_eq!(at(399_999), SpendLevel::Soft);
        assert_eq!(at(400_000), SpendLevel::Mid);
        assert_eq!(at(499_999), SpendLevel::Mid);
        assert_eq!(at(500_000), SpendLevel::Hard);
        assert_eq!(at(i64::MAX), SpendLevel::Hard);
    }

    #[test]
    fn spending_levels_rank_from_none_to_hard() {
        assert!(SpendLevel::None < SpendLevel::Soft);
        assert!(SpendLevel::Soft < SpendLevel::Mid);
        assert!(SpendLevel::Mid < SpendLevel::Hard);
    }
}
