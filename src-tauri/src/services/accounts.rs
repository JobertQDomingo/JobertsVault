use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::invalid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AccountKind {
    Cash,
    Bank,
    Ewallet,
    CreditCard,
}

/// What the new and edit account forms send. The kind can't change once created (UI_FLOWS §5.2).
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountFields {
    pub name: String,
    /// 0 or more, as typed in the form. For a credit card it's the amount owed (DATA_MODEL §5.1).
    pub opening_balance: Money,
    /// Credit card only, like the two days (CC-1).
    pub credit_limit: Option<Money>,
    /// 1–31; a shorter month uses its last day.
    pub statement_day: Option<u8>,
    pub due_day: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NewAccount {
    pub kind: AccountKind,
    #[serde(flatten)]
    pub fields: AccountFields,
}

/// An account as the screens show it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Account {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub kind: AccountKind,
    /// As the form shows it: 0 or more, and the amount owed for a credit card.
    pub opening_balance: Money,
    pub credit_limit: Option<Money>,
    pub statement_day: Option<u8>,
    pub due_day: Option<u8>,
    #[ts(type = "number")]
    pub sort_order: i64,
    pub archived: bool,
}

/// Every account, archived included, in display order.
pub fn list(conn: &Connection) -> Result<Vec<Account>, AppError> {
    repo::accounts::all(conn)?.into_iter().map(view).collect()
}

pub fn get(conn: &Connection, id: i64) -> Result<Account, AppError> {
    view(repo::accounts::find(conn, id)?.ok_or_else(|| not_found(id))?)
}

pub fn create(
    conn: &mut Connection,
    mut input: NewAccount,
    now: DateTime<Utc>,
) -> Result<Account, AppError> {
    validate(input.kind, &mut input.fields)?;
    let tx = conn.transaction()?;
    check_name_free(&tx, &input.fields.name, None)?;
    let id = repo::accounts::insert(
        &tx,
        kind_text(input.kind),
        &values(input.kind, &input.fields),
        &stamp(now),
    )?;
    let account = get(&tx, id)?;
    tx.commit()?;
    Ok(account)
}

pub fn update(
    conn: &mut Connection,
    id: i64,
    mut input: AccountFields,
    now: DateTime<Utc>,
) -> Result<Account, AppError> {
    let tx = conn.transaction()?;
    let kind = get(&tx, id)?.kind;
    validate(kind, &mut input)?;
    check_name_free(&tx, &input.name, Some(id))?;
    repo::accounts::update(&tx, id, &values(kind, &input), &stamp(now))?;
    let account = get(&tx, id)?;
    tx.commit()?;
    Ok(account)
}

/// Gives each account in `ids` its position as `sort_order` (UI_FLOWS §5.1).
pub fn reorder(conn: &mut Connection, ids: Vec<i64>, now: DateTime<Utc>) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    if !ids.iter().all(|id| seen.insert(id)) {
        return Err(invalid("ids", "Each account can appear only once"));
    }
    let tx = conn.transaction()?;
    let now = stamp(now);
    for (order, id) in (0_i64..).zip(ids) {
        if repo::accounts::set_sort_order(&tx, id, order, &now)? == 0 {
            return Err(not_found(id));
        }
    }
    tx.commit()?;
    Ok(())
}

/// Trims the name and checks every field (DATA_MODEL §6).
fn validate(kind: AccountKind, fields: &mut AccountFields) -> Result<(), AppError> {
    fields.name = fields.name.trim().to_owned();
    if !(1..=50).contains(&fields.name.chars().count()) {
        return Err(invalid("name", "Enter a name of 1 to 50 characters"));
    }
    if fields.opening_balance < Money::ZERO {
        return Err(invalid("openingBalance", "Enter an amount of 0 or more"));
    }
    if fields.opening_balance > Money::MAX {
        return Err(invalid("openingBalance", "That amount is too large"));
    }
    let card = kind == AccountKind::CreditCard;
    // Present if and only if the account is a credit card (DATA_MODEL §6).
    for (field, present, missing) in [
        (
            "creditLimit",
            fields.credit_limit.is_some(),
            "Enter a credit limit",
        ),
        (
            "statementDay",
            fields.statement_day.is_some(),
            "Choose a day from 1 to 31",
        ),
        (
            "dueDay",
            fields.due_day.is_some(),
            "Choose a day from 1 to 31",
        ),
    ] {
        if card && !present {
            return Err(invalid(field, missing));
        }
        if !card && present {
            return Err(invalid(field, "Only a credit card has this field"));
        }
    }
    if let Some(limit) = fields.credit_limit {
        if limit <= Money::ZERO {
            return Err(invalid("creditLimit", "Enter a limit above 0"));
        }
        if limit > Money::MAX {
            return Err(invalid("creditLimit", "That amount is too large"));
        }
    }
    for (field, day) in [
        ("statementDay", fields.statement_day),
        ("dueDay", fields.due_day),
    ] {
        if day.is_some_and(|day| !(1..=31).contains(&day)) {
            return Err(invalid(field, "Choose a day from 1 to 31"));
        }
    }
    Ok(())
}

/// The row to store: a card keeps its amount owed as a negative opening balance (DATA_MODEL §3.1).
fn values(kind: AccountKind, fields: &AccountFields) -> repo::accounts::Values<'_> {
    let opening = fields.opening_balance.0;
    repo::accounts::Values {
        name: &fields.name,
        opening_balance: if kind == AccountKind::CreditCard {
            -opening
        } else {
            opening
        },
        credit_limit: fields.credit_limit.map(|limit| limit.0),
        statement_day: fields.statement_day,
        due_day: fields.due_day,
    }
}

fn check_name_free(conn: &Connection, name: &str, except: Option<i64>) -> Result<(), AppError> {
    if repo::accounts::name_taken(conn, name, except)? {
        return Err(invalid("name", "An account with this name already exists"));
    }
    Ok(())
}

fn view(row: repo::accounts::Row) -> Result<Account, AppError> {
    let kind = [
        AccountKind::Cash,
        AccountKind::Bank,
        AccountKind::Ewallet,
        AccountKind::CreditCard,
    ]
    .into_iter()
    .find(|k| kind_text(*k) == row.kind)
    .ok_or_else(|| AppError::Database {
        message: format!("Account {} has an unknown kind", row.id),
    })?;
    let opening_balance = if kind == AccountKind::CreditCard {
        // Stored negative; i64::MIN can only come from a hand-edited database.
        row.opening_balance
            .checked_neg()
            .ok_or_else(|| AppError::Database {
                message: format!("Account {} has an invalid opening balance", row.id),
            })?
    } else {
        row.opening_balance
    };
    Ok(Account {
        id: row.id,
        name: row.name,
        kind,
        opening_balance: Money(opening_balance),
        credit_limit: row.credit_limit.map(Money),
        statement_day: row.statement_day,
        due_day: row.due_day,
        sort_order: row.sort_order,
        archived: row.archived,
    })
}

fn kind_text(kind: AccountKind) -> &'static str {
    match kind {
        AccountKind::Cash => "cash",
        AccountKind::Bank => "bank",
        AccountKind::Ewallet => "ewallet",
        AccountKind::CreditCard => "credit_card",
    }
}

fn not_found(id: i64) -> AppError {
    AppError::NotFound {
        entity: "account".into(),
        id,
    }
}

/// `created_at` / `updated_at` text (DATA_MODEL §1).
fn stamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{db, now};

    type FieldEdit = fn(&mut AccountFields);

    fn field_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn fields(name: &str, opening_balance: i64) -> AccountFields {
        AccountFields {
            name: name.into(),
            opening_balance: Money(opening_balance),
            credit_limit: None,
            statement_day: None,
            due_day: None,
        }
    }

    /// A card owing `owed`, with a 10,000.00 limit, closing on the 20th and due on the 5th.
    fn card(name: &str, owed: i64) -> NewAccount {
        NewAccount {
            kind: AccountKind::CreditCard,
            fields: AccountFields {
                credit_limit: Some(Money(1_000_000)),
                statement_day: Some(20),
                due_day: Some(5),
                ..fields(name, owed)
            },
        }
    }

    fn new(name: &str, kind: AccountKind, opening_balance: i64) -> NewAccount {
        NewAccount {
            kind,
            fields: fields(name, opening_balance),
        }
    }

    fn add(conn: &mut Connection, name: &str) -> Account {
        create(conn, new(name, AccountKind::Bank, 0), now().to_utc()).unwrap()
    }

    #[test]
    fn create_saves_and_lists_after_the_seeded_cash() {
        let mut conn = db();
        let bank = create(
            &mut conn,
            new("Bank", AccountKind::Bank, 100_000),
            now().to_utc(),
        )
        .unwrap();
        assert_eq!(
            bank,
            Account {
                id: bank.id,
                name: "Bank".into(),
                kind: AccountKind::Bank,
                opening_balance: Money(100_000),
                credit_limit: None,
                statement_day: None,
                due_day: None,
                sort_order: 1,
                archived: false,
            }
        );
        let listed = list(&conn).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|a| (a.name.as_str(), a.sort_order))
                .collect::<Vec<_>>(),
            [("Cash", 0), ("Bank", 1)]
        );
        assert_eq!(get(&conn, bank.id).unwrap(), bank);
    }

    #[test]
    fn cash_bank_and_ewallet_accounts_can_be_created() {
        let mut conn = db();
        for (name, kind) in [
            ("Wallet", AccountKind::Cash),
            ("Savings", AccountKind::Bank),
            ("GCash", AccountKind::Ewallet),
        ] {
            assert_eq!(
                create(&mut conn, new(name, kind, 0), now().to_utc())
                    .unwrap()
                    .kind,
                kind
            );
        }
    }

    #[test]
    fn names_are_trimmed_and_1_to_50_characters() {
        let mut conn = db();
        let saved = create(
            &mut conn,
            new("  Bank ", AccountKind::Bank, 0),
            now().to_utc(),
        )
        .unwrap();
        assert_eq!(saved.name, "Bank");
        // 50 characters, but 100 bytes.
        let long = "ñ".repeat(50);
        assert!(create(&mut conn, new(&long, AccountKind::Bank, 0), now().to_utc()).is_ok());
        for bad in ["".to_string(), "   ".into(), "a".repeat(51)] {
            assert_eq!(
                field_of(create(
                    &mut conn,
                    new(&bad, AccountKind::Bank, 0),
                    now().to_utc()
                )),
                "name",
                "{bad:?}"
            );
        }
    }

    #[test]
    fn names_are_unique_ignoring_case_and_archived_ones_count() {
        let mut conn = db();
        assert_eq!(
            field_of(create(
                &mut conn,
                new(" cash ", AccountKind::Cash, 0),
                now().to_utc()
            )),
            "name"
        );
        let old = add(&mut conn, "Old Wallet");
        conn.execute("UPDATE accounts SET archived = 1 WHERE id = ?1", [old.id])
            .unwrap();
        assert_eq!(
            field_of(create(
                &mut conn,
                new("OLD WALLET", AccountKind::Cash, 0),
                now().to_utc()
            )),
            "name"
        );
        assert_eq!(
            field_of(update(
                &mut conn,
                1,
                fields("old wallet", 0),
                now().to_utc()
            )),
            "name"
        );
        // Its own name in a different case is fine.
        let renamed = update(&mut conn, 1, fields("CASH", 0), now().to_utc()).unwrap();
        assert_eq!(renamed.name, "CASH");
    }

    #[test]
    fn opening_balance_is_0_to_the_largest_amount() {
        let mut conn = db();
        for bad in [-1, Money::MAX.0 + 1] {
            assert_eq!(
                field_of(create(
                    &mut conn,
                    new("Bank", AccountKind::Bank, bad),
                    now().to_utc()
                )),
                "openingBalance",
                "{bad}"
            );
            assert_eq!(
                field_of(update(&mut conn, 1, fields("Cash", bad), now().to_utc())),
                "openingBalance"
            );
        }
        assert!(create(&mut conn, new("Zero", AccountKind::Bank, 0), now().to_utc()).is_ok());
        assert!(
            create(
                &mut conn,
                new("Max", AccountKind::Bank, Money::MAX.0),
                now().to_utc()
            )
            .is_ok()
        );
    }

    #[test]
    fn update_changes_the_name_and_opening_balance() {
        let mut conn = db();
        let later = now().to_utc() + chrono::Duration::days(1);
        let saved = update(&mut conn, 1, fields(" Wallet ", 32_000), later).unwrap();
        assert_eq!(
            (saved.name.as_str(), saved.opening_balance),
            ("Wallet", Money(32_000))
        );
        assert_eq!(get(&conn, 1).unwrap(), saved);
        let (created, updated): (String, String) = conn
            .query_row(
                "SELECT created_at, updated_at FROM accounts WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(updated, later.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        assert_ne!(created, updated);
    }

    #[test]
    fn a_missing_account_is_not_found() {
        let mut conn = db();
        let not_found = |result: Result<Account, AppError>| {
            assert!(
                matches!(result, Err(AppError::NotFound { ref entity, id: 99 }) if entity == "account"),
                "{result:?}"
            );
        };
        not_found(get(&conn, 99));
        not_found(update(&mut conn, 99, fields("Gone", 0), now().to_utc()));
    }

    #[test]
    fn reorder_sets_the_order_given() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        let wallet = add(&mut conn, "Wallet");
        reorder(&mut conn, vec![wallet.id, 1, bank.id], now().to_utc()).unwrap();
        let names: Vec<String> = list(&conn).unwrap().into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Wallet", "Cash", "Bank"]);

        assert!(matches!(
            reorder(&mut conn, vec![bank.id, 99], now().to_utc()),
            Err(AppError::NotFound { id: 99, .. })
        ));
        assert_eq!(
            field_of(reorder(&mut conn, vec![bank.id, bank.id], now().to_utc())),
            "ids"
        );
        let unchanged: Vec<String> = list(&conn).unwrap().into_iter().map(|a| a.name).collect();
        assert_eq!(
            unchanged,
            ["Wallet", "Cash", "Bank"],
            "a rejected reorder changes nothing"
        );
    }

    #[test]
    fn a_card_stores_its_amount_owed_as_negative() {
        let mut conn = db();
        let visa = create(&mut conn, card("Visa", 64_000), now().to_utc()).unwrap();
        let stored: i64 = conn
            .query_row(
                "SELECT opening_balance FROM accounts WHERE id = ?1",
                [visa.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored, -64_000);
        assert_eq!(
            visa.opening_balance,
            Money(64_000),
            "the form shows it owed"
        );
        assert_eq!(
            (visa.credit_limit, visa.statement_day, visa.due_day),
            (Some(Money(1_000_000)), Some(20), Some(5))
        );

        let edited = update(
            &mut conn,
            visa.id,
            card("Visa", 1_000).fields,
            now().to_utc(),
        )
        .unwrap();
        assert_eq!(edited.opening_balance, Money(1_000));
    }

    #[test]
    fn a_card_needs_all_three_fields() {
        let mut conn = db();
        let without = |strip: FieldEdit| {
            let mut input = card("Visa", 0);
            strip(&mut input.fields);
            input
        };
        let cases: [(FieldEdit, &str); 3] = [
            (|f| f.credit_limit = None, "creditLimit"),
            (|f| f.statement_day = None, "statementDay"),
            (|f| f.due_day = None, "dueDay"),
        ];
        for (strip, field) in cases {
            assert_eq!(
                field_of(create(&mut conn, without(strip), now().to_utc())),
                field
            );
        }
    }

    #[test]
    fn card_fields_are_only_for_cards() {
        let mut conn = db();
        let cases: [(FieldEdit, &str); 3] = [
            (|f| f.credit_limit = Some(Money(100)), "creditLimit"),
            (|f| f.statement_day = Some(1), "statementDay"),
            (|f| f.due_day = Some(1), "dueDay"),
        ];
        for (add_field, field) in cases {
            let mut input = new("Bank", AccountKind::Bank, 0);
            add_field(&mut input.fields);
            assert_eq!(
                field_of(create(&mut conn, input.clone(), now().to_utc())),
                field
            );
            assert_eq!(
                field_of(update(&mut conn, 1, input.fields, now().to_utc())),
                field,
                "editing Cash"
            );
        }
    }

    #[test]
    fn a_limit_is_1_to_the_largest_amount_and_days_are_1_to_31() {
        let mut conn = db();
        let with = |limit: i64, statement: u8, due: u8| {
            let mut input = card("Visa", 0);
            input.fields.credit_limit = Some(Money(limit));
            input.fields.statement_day = Some(statement);
            input.fields.due_day = Some(due);
            input
        };
        let mut create_with =
            |limit, statement, due| create(&mut conn, with(limit, statement, due), now().to_utc());
        assert_eq!(field_of(create_with(0, 20, 5)), "creditLimit");
        assert_eq!(
            field_of(create_with(Money::MAX.0 + 1, 20, 5)),
            "creditLimit"
        );
        assert_eq!(field_of(create_with(100, 0, 5)), "statementDay");
        assert_eq!(field_of(create_with(100, 32, 5)), "statementDay");
        assert_eq!(field_of(create_with(100, 20, 0)), "dueDay");
        assert_eq!(field_of(create_with(100, 20, 32)), "dueDay");
        assert!(create_with(Money::MAX.0, 31, 31).is_ok());
    }

    #[test]
    fn any_number_of_cards_can_be_added() {
        let mut conn = db();
        for name in ["Visa", "Mastercard", "Amex"] {
            create(&mut conn, card(name, 0), now().to_utc()).unwrap();
        }
        let cards = list(&conn)
            .unwrap()
            .into_iter()
            .filter(|a| a.kind == AccountKind::CreditCard)
            .count();
        assert_eq!(cards, 3);
    }

    #[test]
    fn update_can_change_the_card_fields() {
        let mut conn = db();
        let visa = create(&mut conn, card("Visa", 0), now().to_utc()).unwrap();
        let mut changed = card("Visa", 0).fields;
        changed.credit_limit = Some(Money(2_000_000));
        changed.statement_day = Some(31);
        changed.due_day = Some(15);
        let saved = update(&mut conn, visa.id, changed, now().to_utc()).unwrap();
        assert_eq!(
            (saved.credit_limit, saved.statement_day, saved.due_day),
            (Some(Money(2_000_000)), Some(31), Some(15))
        );
        // A card still needs its fields when edited.
        let mut stripped = card("Visa", 0).fields;
        stripped.due_day = None;
        assert_eq!(
            field_of(update(&mut conn, visa.id, stripped, now().to_utc())),
            "dueDay"
        );
    }
}
