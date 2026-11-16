use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// An amount in minor units (centavos), never a float (DATA_MODEL §1, ADR 0002). Serde sends a
/// one-field tuple struct as its bare value, so it crosses IPC as a plain number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Money(#[ts(type = "number")] pub i64);

/// How many decimal places the currency shows: `currency_decimals` is 0 or 2 (DATA_MODEL §3.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decimals {
    Zero,
    Two,
}

impl Decimals {
    /// From the `currency_decimals` setting, 0 or 2 (DATA_MODEL §3.5).
    pub fn of(currency_decimals: u8) -> Self {
        if currency_decimals == 0 {
            Self::Zero
        } else {
            Self::Two
        }
    }

    fn places(self) -> usize {
        match self {
            Self::Zero => 0,
            Self::Two => 2,
        }
    }
}

impl Money {
    pub const ZERO: Money = Money(0);
    /// The largest amount any field accepts (DATA_MODEL §6).
    pub const MAX: Money = Money(999_999_999_999);

    pub fn checked_add(self, other: Money) -> Option<Money> {
        self.0.checked_add(other.0).map(Money)
    }

    pub fn checked_sub(self, other: Money) -> Option<Money> {
        self.0.checked_sub(other.0).map(Money)
    }

    /// Reads typed or pasted text: digits, one `.`, and commas as thousands separators
    /// (DESIGN_SYSTEM §7). Anything else, or more than `MAX`, is `None`. Never rounds.
    pub fn parse(text: &str, decimals: Decimals) -> Option<Money> {
        let text = text.trim();
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        let whole = whole_digits(whole)?;
        let width = decimals.places();
        let fraction_ok = fraction.len() <= width && fraction.chars().all(|c| c.is_ascii_digit());
        if !fraction_ok || (whole.is_empty() && fraction.is_empty()) {
            return None;
        }
        // Only digits are left, so this can't read a sign, and it reports overflow itself.
        let minor: i64 = format!("{whole}{fraction:0<width$}").parse().ok()?;
        (minor <= Money::MAX.0).then_some(Money(minor))
    }

    /// Plain text for files: no symbol or separators, and `-` when negative (DATA_MODEL §8).
    pub fn to_plain(self, decimals: Decimals) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        let width = decimals.places();
        if width == 0 {
            return format!("{sign}{abs}");
        }
        let unit = 10u64.pow(width as u32);
        format!("{sign}{}.{:0width$}", abs / unit, abs % unit)
    }

    /// An amount of 0 or more as `formatMoney` shows it (src/lib/money.ts), for messages:
    /// "₱1,234.50", or "1,234.50 ₱" (a no-break space) with the symbol after.
    pub fn shown(self, decimals: Decimals, symbol: &str, before: bool) -> String {
        let plain = self.to_plain(decimals);
        let (whole, fraction) = plain
            .split_once('.')
            .map_or((plain.as_str(), None), |(w, f)| (w, Some(f)));
        let mut grouped = String::new();
        for (i, c) in whole.chars().enumerate() {
            if i > 0 && (whole.len() - i) % 3 == 0 {
                grouped.push(',');
            }
            grouped.push(c);
        }
        if let Some(f) = fraction {
            grouped = format!("{grouped}.{f}");
        }
        if before {
            format!("{symbol}{grouped}")
        } else {
            format!("{grouped}\u{a0}{symbol}")
        }
    }
}

/// The digits of the whole part: plain, or grouped by commas in threes, as in "12,345".
fn whole_digits(whole: &str) -> Option<String> {
    let groups: Vec<&str> = whole.split(',').collect();
    let well_grouped = groups.len() == 1
        || groups.iter().enumerate().all(|(i, group)| match i {
            0 => (1..=3).contains(&group.len()),
            _ => group.len() == 3,
        });
    let digits = groups.concat();
    (well_grouped && digits.chars().all(|c| c.is_ascii_digit())).then_some(digits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str, decimals: Decimals) -> Option<i64> {
        Money::parse(text, decimals).map(|money| money.0)
    }

    #[test]
    fn parses_typed_and_pasted_amounts() {
        assert_eq!(parsed("1,234.5", Decimals::Two), Some(123_450));
        assert_eq!(parsed("1234.50", Decimals::Two), Some(123_450));
        assert_eq!(parsed(" 12 ", Decimals::Two), Some(1_200));
        assert_eq!(parsed(".5", Decimals::Two), Some(50));
        assert_eq!(parsed("12.", Decimals::Two), Some(1_200));
        assert_eq!(parsed("0", Decimals::Two), Some(0));
        assert_eq!(parsed("1,234", Decimals::Zero), Some(1_234));
    }

    #[test]
    fn parses_up_to_the_largest_amount() {
        assert_eq!(
            parsed("999,999,999,999", Decimals::Zero),
            Some(Money::MAX.0)
        );
        assert_eq!(
            parsed("9,999,999,999.99", Decimals::Two),
            Some(Money::MAX.0)
        );
        assert_eq!(parsed("10,000,000,000", Decimals::Two), None);
        assert_eq!(parsed("99999999999999999999", Decimals::Zero), None);
    }

    #[test]
    fn rejects_anything_else_instead_of_guessing() {
        for text in ["", ".", "-5", "+5", "1.2.3", "₱5", "1e3", "12.3,4"] {
            assert_eq!(parsed(text, Decimals::Two), None, "{text:?}");
        }
        // A comma is only a thousands separator. "12,50" isn't ₱1,250.00.
        for text in ["12,50", "1,2345", "1,2,3", ",5", "1234,567", "1,"] {
            assert_eq!(parsed(text, Decimals::Two), None, "{text:?}");
        }
        assert_eq!(parsed("12,345,678.90", Decimals::Two), Some(1_234_567_890));
        // Never rounded: more fraction digits than the currency has is a typo.
        assert_eq!(parsed("12.5", Decimals::Zero), None);
        assert_eq!(parsed("1.234", Decimals::Two), None);
    }

    #[test]
    fn formats_plain_amounts_for_csv() {
        assert_eq!(Money(123_450).to_plain(Decimals::Two), "1234.50");
        assert_eq!(Money(-123_450).to_plain(Decimals::Two), "-1234.50");
        assert_eq!(Money(5).to_plain(Decimals::Two), "0.05");
        assert_eq!(Money(0).to_plain(Decimals::Two), "0.00");
        assert_eq!(Money(1_234).to_plain(Decimals::Zero), "1234");
        assert!(Money(i64::MIN).to_plain(Decimals::Two).starts_with('-'));
    }

    #[test]
    fn arithmetic_is_checked() {
        assert_eq!(Money(i64::MAX).checked_add(Money(1)), None);
        assert_eq!(Money(5).checked_sub(Money(7)), Some(Money(-2)));
        assert_eq!(Money(i64::MIN).checked_sub(Money(1)), None);
    }

    #[test]
    fn shown_formats_like_the_ui() {
        assert_eq!(Money(43_334).shown(Decimals::Two, "₱", true), "₱433.34");
        assert_eq!(
            Money(123_456_789).shown(Decimals::Two, "₱", true),
            "₱1,234,567.89"
        );
        assert_eq!(
            Money(1_234_567).shown(Decimals::Zero, "$", false),
            "1,234,567\u{a0}$"
        );
        assert_eq!(Money(5).shown(Decimals::Two, "₱", true), "₱0.05");
        assert_eq!(Money(999).shown(Decimals::Zero, "₱", true), "₱999");
    }

    #[test]
    fn money_crosses_ipc_as_a_number() {
        assert_eq!(
            Money::decl(&ts_rs::Config::default()),
            "type Money = number;"
        );
    }
}
