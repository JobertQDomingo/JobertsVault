use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An amount in minor units (centavos), never a float (DATA_MODEL §1, ADR 0002). Serde sends a
/// one-field tuple struct as its bare value, so it crosses IPC as a plain number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Money(#[ts(type = "number")] pub i64);

impl Money {
    /// The largest amount any field accepts (DATA_MODEL §6).
    pub const MAX: Money = Money(999_999_999_999);

    pub fn checked_add(self, other: Money) -> Option<Money> {
        self.0.checked_add(other.0).map(Money)
    }

    pub fn checked_sub(self, other: Money) -> Option<Money> {
        self.0.checked_sub(other.0).map(Money)
    }

    /// Reads typed or pasted text: digits, one `.`, and commas in the whole part
    /// (DESIGN_SYSTEM §7). Anything else, or more than `MAX`, is `None`. Never rounds.
    pub fn parse(text: &str, decimals: u32) -> Option<Money> {
        let text = text.trim();
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let whole: String = whole.chars().filter(|&c| c != ',').collect();
        let digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
        let width = decimals as usize;
        if !digits(&whole) || !digits(fraction) || fraction.len() > width {
            return None;
        }
        if whole.is_empty() && fraction.is_empty() {
            return None;
        }
        let padding = std::iter::repeat_n('0', width - fraction.len());
        let mut minor: i64 = 0;
        for c in whole.chars().chain(fraction.chars()).chain(padding) {
            minor = minor
                .checked_mul(10)?
                .checked_add(i64::from(c.to_digit(10)?))?;
        }
        (minor <= Money::MAX.0).then_some(Money(minor))
    }

    /// Plain text for files: no symbol or separators, and `-` when negative (DATA_MODEL §8).
    pub fn to_plain(self, decimals: u32) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        if decimals == 0 {
            return format!("{sign}{abs}");
        }
        let unit = 10u64.pow(decimals);
        format!(
            "{sign}{}.{:0width$}",
            abs / unit,
            abs % unit,
            width = decimals as usize
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str, decimals: u32) -> Option<i64> {
        Money::parse(text, decimals).map(|money| money.0)
    }

    #[test]
    fn parses_typed_and_pasted_amounts() {
        assert_eq!(parsed("1,234.5", 2), Some(123_450));
        assert_eq!(parsed("1234.50", 2), Some(123_450));
        assert_eq!(parsed(" 12 ", 2), Some(1_200));
        assert_eq!(parsed(".5", 2), Some(50));
        assert_eq!(parsed("12.", 2), Some(1_200));
        assert_eq!(parsed("0", 2), Some(0));
        assert_eq!(parsed("1,234", 0), Some(1_234));
    }

    #[test]
    fn parses_up_to_the_largest_amount() {
        assert_eq!(parsed("999,999,999,999", 0), Some(Money::MAX.0));
        assert_eq!(parsed("9,999,999,999.99", 2), Some(Money::MAX.0));
        assert_eq!(parsed("10,000,000,000", 2), None);
        assert_eq!(parsed("99999999999999999999", 0), None);
    }

    #[test]
    fn rejects_anything_else_instead_of_guessing() {
        for text in ["", ".", "-5", "+5", "1.2.3", "₱5", "1e3", "12.3,4"] {
            assert_eq!(parsed(text, 2), None, "{text:?}");
        }
        // Never rounded: more fraction digits than the currency has is a typo.
        assert_eq!(parsed("12.5", 0), None);
        assert_eq!(parsed("1.234", 2), None);
    }

    #[test]
    fn formats_plain_amounts_for_csv() {
        assert_eq!(Money(123_450).to_plain(2), "1234.50");
        assert_eq!(Money(-123_450).to_plain(2), "-1234.50");
        assert_eq!(Money(5).to_plain(2), "0.05");
        assert_eq!(Money(0).to_plain(2), "0.00");
        assert_eq!(Money(1_234).to_plain(0), "1234");
        assert!(Money(i64::MIN).to_plain(2).starts_with('-'));
    }

    #[test]
    fn arithmetic_is_checked() {
        assert_eq!(Money(i64::MAX).checked_add(Money(1)), None);
        assert_eq!(Money(5).checked_sub(Money(7)), Some(Money(-2)));
        assert_eq!(Money(i64::MIN).checked_sub(Money(1)), None);
    }

    #[test]
    fn money_crosses_ipc_as_a_number() {
        assert_eq!(
            Money::decl(&ts_rs::Config::default()),
            "type Money = number;"
        );
    }
}
