use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;
use crate::repo;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum CurrencyPosition {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum WeekStart {
    Monday,
    Sunday,
}

/// The Phase 1 settings (DATA_MODEL §3.5). A key missing from the table takes its default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Settings {
    pub currency_symbol: String,
    pub currency_position: CurrencyPosition,
    #[ts(type = "0 | 2")]
    pub currency_decimals: u8,
    pub week_start: WeekStart,
    pub month_start_day: u8,
    pub show_payment_method: bool,
    pub onboarding_completed: bool,
    pub auto_update_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            currency_symbol: "\u{20B1}".into(), // ₱
            currency_position: CurrencyPosition::Before,
            currency_decimals: 2,
            week_start: WeekStart::Monday,
            month_start_day: 1,
            show_payment_method: true,
            onboarding_completed: false,
            auto_update_enabled: true,
        }
    }
}

/// What `get_settings` and `update_settings` return.
#[derive(Debug, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsView {
    #[serde(flatten)]
    pub settings: Settings,
    /// `currency_decimals` can't change once any amount is stored (DATA_MODEL §1).
    pub decimals_locked: bool,
}

pub fn get(conn: &Connection) -> Result<SettingsView, AppError> {
    Ok(SettingsView {
        settings: load(conn, |_| true)?,
        decimals_locked: decimals_locked(conn)?,
    })
}

/// Changing the decimals would change the meaning of every stored amount (DATA_MODEL §1).
fn decimals_locked(conn: &Connection) -> Result<bool, AppError> {
    Ok(repo::transactions::any(conn)? || repo::accounts::any_with_amount(conn)?)
}

/// The stored rows whose key `wanted` picks, over the defaults.
fn load(conn: &Connection, wanted: impl Fn(&str) -> bool) -> Result<Settings, AppError> {
    let mut settings = Settings::default();
    for (key, value) in repo::settings::all(conn)? {
        if wanted(&key) && read(&mut settings, &key, &value).is_none() {
            // Never the value itself: Database errors go to the log (ARCHITECTURE §4.4).
            return Err(AppError::Database {
                message: format!("The setting {key} has an invalid value"),
            });
        }
    }
    Ok(settings)
}

/// Saves every key at once: the form sends the whole set each time a field changes.
pub fn update(conn: &mut Connection, mut settings: Settings) -> Result<SettingsView, AppError> {
    settings.currency_symbol = settings.currency_symbol.trim().to_owned();
    if !symbol_ok(&settings.currency_symbol) {
        return Err(invalid(
            "currencySymbol",
            "Enter a symbol of 1 to 5 characters",
        ));
    }
    if !decimals_ok(settings.currency_decimals) {
        return Err(invalid("currencyDecimals", "Choose 0 or 2 decimal places"));
    }
    if !month_start_day_ok(settings.month_start_day) {
        return Err(invalid("monthStartDay", "Choose a day from 1 to 28"));
    }
    let tx = conn.transaction()?;
    let decimals_locked = decimals_locked(&tx)?;
    // Reads only the row the lock needs, so saving replaces any other bad row.
    if decimals_locked
        && settings.currency_decimals
            != load(&tx, |key| key == "currency_decimals")?.currency_decimals
    {
        return Err(invalid(
            "currencyDecimals",
            "Decimal places can't change once you've saved an amount",
        ));
    }
    for (key, value) in [
        ("currency_symbol", settings.currency_symbol.clone()),
        (
            "currency_position",
            position_text(settings.currency_position).into(),
        ),
        ("currency_decimals", settings.currency_decimals.to_string()),
        ("week_start", week_start_text(settings.week_start).into()),
        ("month_start_day", settings.month_start_day.to_string()),
        (
            "show_payment_method",
            settings.show_payment_method.to_string(),
        ),
        (
            "onboarding_completed",
            settings.onboarding_completed.to_string(),
        ),
        (
            "auto_update_enabled",
            settings.auto_update_enabled.to_string(),
        ),
    ] {
        repo::settings::set(&tx, key, &value)?;
    }
    tx.commit()?;
    Ok(SettingsView {
        settings,
        decimals_locked,
    })
}

/// Reads one stored row into `settings`. `None` when the value isn't valid for its key.
fn read(settings: &mut Settings, key: &str, value: &str) -> Option<()> {
    let flag = || match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };
    match key {
        "currency_symbol" => {
            settings.currency_symbol = Some(value.to_owned()).filter(|s| symbol_ok(s))?
        }
        "currency_position" => {
            settings.currency_position = [CurrencyPosition::Before, CurrencyPosition::After]
                .into_iter()
                .find(|p| position_text(*p) == value)?
        }
        "currency_decimals" => {
            settings.currency_decimals = value.parse().ok().filter(|d| decimals_ok(*d))?
        }
        "week_start" => {
            settings.week_start = [WeekStart::Monday, WeekStart::Sunday]
                .into_iter()
                .find(|w| week_start_text(*w) == value)?
        }
        "month_start_day" => {
            settings.month_start_day = value.parse().ok().filter(|d| month_start_day_ok(*d))?
        }
        "show_payment_method" => settings.show_payment_method = flag()?,
        "onboarding_completed" => settings.onboarding_completed = flag()?,
        "auto_update_enabled" => settings.auto_update_enabled = flag()?,
        // Later phases' keys, and last_used_account_id, which transactions own.
        _ => {}
    }
    Some(())
}

fn position_text(position: CurrencyPosition) -> &'static str {
    match position {
        CurrencyPosition::Before => "before",
        CurrencyPosition::After => "after",
    }
}

fn week_start_text(week_start: WeekStart) -> &'static str {
    match week_start {
        WeekStart::Monday => "monday",
        WeekStart::Sunday => "sunday",
    }
}

fn symbol_ok(symbol: &str) -> bool {
    (1..=5).contains(&symbol.chars().count())
}

fn decimals_ok(decimals: u8) -> bool {
    matches!(decimals, 0 | 2)
}

/// Capped at 28 so every month has that day (DATA_MODEL §3.5).
fn month_start_day_ok(day: u8) -> bool {
    (1..=28).contains(&day)
}

fn invalid(field: &str, message: &str) -> AppError {
    AppError::Validation {
        field: field.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::db;

    fn field_of(result: Result<SettingsView, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn store(conn: &Connection, key: &str, value: &str) {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            [key, value],
        )
        .unwrap();
    }

    #[test]
    fn a_new_database_returns_the_defaults() {
        let defaults = Settings {
            currency_symbol: "\u{20B1}".into(),
            currency_position: CurrencyPosition::Before,
            currency_decimals: 2,
            week_start: WeekStart::Monday,
            month_start_day: 1,
            show_payment_method: true,
            onboarding_completed: false,
            auto_update_enabled: true,
        };
        assert_eq!(Settings::default(), defaults);
        assert_eq!(
            get(&db()).unwrap(),
            SettingsView {
                settings: defaults,
                decimals_locked: false
            }
        );
    }

    #[test]
    fn update_saves_every_key_and_get_reads_them_back() {
        let mut conn = db();
        let changed = Settings {
            currency_symbol: "US$".into(),
            currency_position: CurrencyPosition::After,
            currency_decimals: 0,
            week_start: WeekStart::Sunday,
            month_start_day: 25,
            show_payment_method: false,
            onboarding_completed: true,
            auto_update_enabled: false,
        };
        let saved = update(&mut conn, changed.clone()).unwrap();
        assert_eq!(saved.settings, changed);
        assert_eq!(get(&conn).unwrap().settings, changed);
    }

    #[test]
    fn the_symbol_is_trimmed_and_must_be_1_to_5_characters() {
        let mut conn = db();
        let with = |symbol: &str| Settings {
            currency_symbol: symbol.into(),
            ..Settings::default()
        };
        let saved = update(&mut conn, with(" $ ")).unwrap();
        assert_eq!(saved.settings.currency_symbol, "$");
        // Five characters, but 15 bytes.
        assert!(update(&mut conn, with("\u{20B1}\u{20B1}\u{20B1}\u{20B1}\u{20B1}")).is_ok());
        for bad in ["", "   ", "ABCDEF"] {
            assert_eq!(
                field_of(update(&mut conn, with(bad))),
                "currencySymbol",
                "{bad:?}"
            );
        }
    }

    #[test]
    fn decimals_must_be_0_or_2() {
        let mut conn = db();
        let with = |currency_decimals| Settings {
            currency_decimals,
            ..Settings::default()
        };
        for bad in [1, 3] {
            assert_eq!(field_of(update(&mut conn, with(bad))), "currencyDecimals");
        }
        assert!(update(&mut conn, with(0)).is_ok());
    }

    #[test]
    fn month_start_day_must_be_1_to_28() {
        let mut conn = db();
        let with = |month_start_day| Settings {
            month_start_day,
            ..Settings::default()
        };
        for bad in [0, 29] {
            assert_eq!(field_of(update(&mut conn, with(bad))), "monthStartDay");
        }
        for good in [1, 28] {
            assert!(update(&mut conn, with(good)).is_ok());
        }
    }

    #[test]
    fn decimals_lock_once_any_transaction_exists() {
        let mut conn = db();
        // The seed holds account 1 (Cash) and category 1 (Salary).
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, category_id, created_at, updated_at) VALUES ('income', 100, '2026-11-03', 1, 1, 'x', 'x')",
            [],
        )
        .unwrap();
        assert!(get(&conn).unwrap().decimals_locked);

        let zero = Settings {
            currency_decimals: 0,
            ..Settings::default()
        };
        assert_eq!(field_of(update(&mut conn, zero)), "currencyDecimals");
        assert_eq!(get(&conn).unwrap().settings.currency_decimals, 2);

        let same_decimals = Settings {
            week_start: WeekStart::Sunday,
            ..Settings::default()
        };
        let saved = update(&mut conn, same_decimals).unwrap();
        assert_eq!(saved.settings.week_start, WeekStart::Sunday);
        assert!(saved.decimals_locked);
    }

    #[test]
    fn decimals_lock_once_an_account_holds_an_amount() {
        let add = |conn: &Connection, values: &str| {
            conn.execute(
                &format!("INSERT INTO accounts (name, kind, opening_balance, credit_limit, statement_day, due_day, created_at, updated_at) VALUES {values}"),
                [],
            )
            .unwrap();
        };
        let conn = db();
        assert!(
            !get(&conn).unwrap().decimals_locked,
            "the seeded Cash holds 0"
        );
        add(
            &conn,
            "('Bank', 'bank', 100000, NULL, NULL, NULL, 'x', 'x')",
        );
        assert!(get(&conn).unwrap().decimals_locked);

        let mut conn = db();
        add(
            &conn,
            "('Visa', 'credit_card', 0, 5000000, 15, 5, 'x', 'x')",
        );
        let zero = Settings {
            currency_decimals: 0,
            ..Settings::default()
        };
        assert_eq!(field_of(update(&mut conn, zero)), "currencyDecimals");
    }

    #[test]
    fn a_rejected_update_changes_nothing() {
        let mut conn = db();
        let bad = Settings {
            currency_symbol: "$".into(),
            month_start_day: 29,
            ..Settings::default()
        };
        assert!(update(&mut conn, bad).is_err());
        assert_eq!(get(&conn).unwrap().settings, Settings::default());
    }

    #[test]
    fn an_invalid_stored_value_is_a_database_error() {
        let conn = db();
        store(&conn, "month_start_day", "40");
        match get(&conn) {
            Err(AppError::Database { message }) => {
                assert!(message.contains("month_start_day"), "{message}");
                assert!(
                    !message.contains("40"),
                    "the value must not be shown: {message}"
                );
            }
            other => panic!("expected a database error, got {other:?}"),
        }
    }

    #[test]
    fn saving_repairs_an_invalid_stored_value() {
        let mut conn = db();
        store(&conn, "month_start_day", "40");
        update(&mut conn, Settings::default()).unwrap();
        assert_eq!(get(&conn).unwrap().settings, Settings::default());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let conn = db();
        store(&conn, "last_used_account_id", "1");
        assert_eq!(get(&conn).unwrap().settings, Settings::default());
    }
}
