use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::account::card_figures;
use crate::domain::card_cycle::card_cycle;
use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::{invalid, new_order, stamp};

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
    /// 1–31; a shorter month uses its last day. Any integer arrives, so validation can name the
    /// field instead of the argument decoding failing.
    #[ts(type = "number | null")]
    pub statement_day: Option<i64>,
    #[ts(type = "number | null")]
    pub due_day: Option<i64>,
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
    /// Signed and calculated as of today: card debt is negative (DATA_MODEL §5.1).
    pub balance: Money,
    /// Only for a credit card.
    pub card: Option<CardSummary>,
    /// Whether any transaction uses it, in either direction. Only an unused account can be deleted.
    pub in_use: bool,
}

/// A credit card's details view (CC-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardSummary {
    pub amount_owed: Money,
    /// Negative once spending passes the limit.
    pub available_credit: Money,
    /// A whole percent, over 100 past the limit.
    #[ts(type = "number")]
    pub utilization_percent: i64,
    /// The current statement cycle, which ends on the closing date (DATA_MODEL §5.5).
    pub cycle_start: NaiveDate,
    pub cycle_end: NaiveDate,
    /// Expenses on the card from the cycle start through today.
    pub cycle_spending: Money,
    pub due_date: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AccountList {
    pub accounts: Vec<Account>,
    /// Every balance added up, archived accounts included (ACC-5).
    pub net_balance: Money,
}

/// Every account, archived included, in display order, with balances as of `today`.
pub fn list(conn: &Connection, today: NaiveDate) -> Result<AccountList, AppError> {
    let accounts = repo::accounts::all(conn, today)?
        .into_iter()
        .map(|row| view(conn, row, today))
        .collect::<Result<Vec<_>, _>>()?;
    let net_balance = accounts
        .iter()
        .try_fold(Money::ZERO, |sum, a| sum.checked_add(a.balance))
        .ok_or_else(|| AppError::Database {
            message: "The net balance is too large".into(),
        })?;
    Ok(AccountList {
        accounts,
        net_balance,
    })
}

pub fn get(conn: &Connection, id: i64, today: NaiveDate) -> Result<Account, AppError> {
    view(conn, stored(conn, id, today)?, today)
}

/// The stored row, for writes that don't need the card summary.
fn stored(conn: &Connection, id: i64, today: NaiveDate) -> Result<repo::accounts::Row, AppError> {
    repo::accounts::find(conn, id, today)?.ok_or_else(|| not_found(id))
}

pub fn create(
    conn: &mut Connection,
    mut input: NewAccount,
    now: DateTime<Local>,
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
    let account = get(&tx, id, now.date_naive())?;
    tx.commit()?;
    Ok(account)
}

pub fn update(
    conn: &mut Connection,
    id: i64,
    mut input: AccountFields,
    now: DateTime<Local>,
) -> Result<Account, AppError> {
    let tx = conn.transaction()?;
    let kind = kind_of(&stored(&tx, id, now.date_naive())?)?;
    validate(kind, &mut input)?;
    check_name_free(&tx, &input.name, Some(id))?;
    repo::accounts::update(&tx, id, &values(kind, &input), &stamp(now))?;
    let account = get(&tx, id, now.date_naive())?;
    tx.commit()?;
    Ok(account)
}

/// Gives each account in `ids` its position as `sort_order` (UI_FLOWS §5.1).
pub fn reorder(conn: &mut Connection, ids: Vec<i64>, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let existing = repo::accounts::all(&tx, now.date_naive())?
        .into_iter()
        .map(|row| row.id);
    let order = new_order(ids, existing)?;
    let now = stamp(now);
    for (position, id) in (0_i64..).zip(order) {
        if repo::accounts::set_sort_order(&tx, id, position, &now)? == 0 {
            return Err(not_found(id));
        }
    }
    tx.commit()?;
    Ok(())
}

/// Hides the account from pickers. It still counts toward the net balance, so a balance other
/// than 0 needs `acknowledge_balance` after the user confirms the warning (ACC-3, UI_FLOWS F6).
pub fn archive(
    conn: &mut Connection,
    id: i64,
    acknowledge_balance: bool,
    now: DateTime<Local>,
) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let row = stored(&tx, id, now.date_naive())?;
    if row.archived {
        return Ok(());
    }
    check_not_last_active(&tx)?;
    if row.balance != 0 && !acknowledge_balance {
        return Err(AppError::Conflict {
            message: "This account's balance isn't 0. Archived accounts still count toward your net balance.".into(),
        });
    }
    repo::accounts::set_archived(&tx, id, true, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

pub fn unarchive(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if !stored(&tx, id, now.date_naive())?.archived {
        return Ok(());
    }
    repo::accounts::set_archived(&tx, id, false, &stamp(now))?;
    tx.commit()?;
    Ok(())
}

/// Only an account no transaction uses can go; any other is archived instead (ACC-3).
pub fn delete(conn: &mut Connection, id: i64, now: DateTime<Local>) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    let row = stored(&tx, id, now.date_naive())?;
    if row.in_use {
        return Err(AppError::Conflict {
            message: "This account has transactions, so it can only be archived".into(),
        });
    }
    if !row.archived {
        check_not_last_active(&tx)?;
    }
    repo::accounts::delete(&tx, id)?;
    tx.commit()?;
    Ok(())
}

/// Called before an active account is archived or deleted.
fn check_not_last_active(conn: &Connection) -> Result<(), AppError> {
    if repo::accounts::active_count(conn)? <= 1 {
        return Err(AppError::Conflict {
            message: "Keep at least one active account".into(),
        });
    }
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

fn view(
    conn: &Connection,
    row: repo::accounts::Row,
    today: NaiveDate,
) -> Result<Account, AppError> {
    let kind = kind_of(&row)?;
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
    let balance = Money(row.balance);
    let card = match (row.credit_limit, row.statement_day, row.due_day) {
        (Some(limit), Some(statement_day), Some(due_day)) if kind == AccountKind::CreditCard => {
            let terms = CardTerms {
                credit_limit: Money(limit),
                statement_day,
                due_day,
            };
            Some(card_summary(conn, row.id, balance, terms, today)?)
        }
        _ => None,
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
        balance,
        card,
        in_use: row.in_use,
    })
}

fn kind_of(row: &repo::accounts::Row) -> Result<AccountKind, AppError> {
    [
        AccountKind::Cash,
        AccountKind::Bank,
        AccountKind::Ewallet,
        AccountKind::CreditCard,
    ]
    .into_iter()
    .find(|k| kind_text(*k) == row.kind)
    .ok_or_else(|| AppError::Database {
        message: format!("Account {} has an unknown kind", row.id),
    })
}

struct CardTerms {
    credit_limit: Money,
    statement_day: u8,
    due_day: u8,
}

fn card_summary(
    conn: &Connection,
    id: i64,
    balance: Money,
    terms: CardTerms,
    today: NaiveDate,
) -> Result<CardSummary, AppError> {
    let cycle = card_cycle(terms.statement_day, terms.due_day, today).ok_or_else(|| {
        AppError::Conflict {
            message: "Today's date is outside the supported range".into(),
        }
    })?;
    let figures = card_figures(balance, terms.credit_limit);
    // Through today, so a future-dated purchase waits for its date (TXN-8).
    let spent = repo::accounts::expenses_between(conn, id, cycle.start, cycle.end.min(today))?;
    Ok(CardSummary {
        amount_owed: figures.amount_owed,
        available_credit: figures.available_credit,
        utilization_percent: figures.utilization_percent,
        cycle_start: cycle.start,
        cycle_end: cycle.end,
        cycle_spending: Money(spent),
        due_date: cycle.due,
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
    super::not_found("account", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{db, now};
    use chrono::NaiveDate;

    fn today() -> NaiveDate {
        now().date_naive()
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    /// Inserts a transaction directly: the transaction commands come in ROADMAP 1.5.
    fn txn(
        conn: &Connection,
        kind: &str,
        amount: i64,
        date: NaiveDate,
        from: i64,
        to: Option<i64>,
    ) {
        conn.execute(
            "INSERT INTO transactions (kind, amount, date, account_id, to_account_id, category_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, (SELECT id FROM categories WHERE kind = ?1 LIMIT 1), 'x', 'x')",
            rusqlite::params![kind, amount, date.to_string(), from, to],
        )
        .unwrap();
    }

    fn balance_of(conn: &Connection, id: i64, on: NaiveDate) -> i64 {
        get(conn, id, on).unwrap().balance.0
    }

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
        create(conn, new(name, AccountKind::Bank, 0), now()).unwrap()
    }

    #[test]
    fn create_saves_and_lists_after_the_seeded_cash() {
        let mut conn = db();
        let bank = create(&mut conn, new("Bank", AccountKind::Bank, 100_000), now()).unwrap();
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
                balance: Money(100_000),
                card: None,
                in_use: false,
            }
        );
        let listed = list(&conn, today()).unwrap().accounts;
        assert_eq!(
            listed
                .iter()
                .map(|a| (a.name.as_str(), a.sort_order))
                .collect::<Vec<_>>(),
            [("Cash", 0), ("Bank", 1)]
        );
        assert_eq!(get(&conn, bank.id, today()).unwrap(), bank);
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
                create(&mut conn, new(name, kind, 0), now()).unwrap().kind,
                kind
            );
        }
    }

    #[test]
    fn names_are_trimmed_and_1_to_50_characters() {
        let mut conn = db();
        let saved = create(&mut conn, new("  Bank ", AccountKind::Bank, 0), now()).unwrap();
        assert_eq!(saved.name, "Bank");
        // 50 characters, but 100 bytes.
        let long = "ñ".repeat(50);
        assert!(create(&mut conn, new(&long, AccountKind::Bank, 0), now()).is_ok());
        for bad in ["".to_string(), "   ".into(), "a".repeat(51)] {
            assert_eq!(
                field_of(create(&mut conn, new(&bad, AccountKind::Bank, 0), now())),
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
                now()
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
                now()
            )),
            "name"
        );
        assert_eq!(
            field_of(update(&mut conn, 1, fields("old wallet", 0), now())),
            "name"
        );
        // Its own name in a different case is fine.
        let renamed = update(&mut conn, 1, fields("CASH", 0), now()).unwrap();
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
                    now()
                )),
                "openingBalance",
                "{bad}"
            );
            assert_eq!(
                field_of(update(&mut conn, 1, fields("Cash", bad), now())),
                "openingBalance"
            );
        }
        assert!(create(&mut conn, new("Zero", AccountKind::Bank, 0), now()).is_ok());
        assert!(
            create(
                &mut conn,
                new("Max", AccountKind::Bank, Money::MAX.0),
                now()
            )
            .is_ok()
        );
    }

    #[test]
    fn update_changes_the_name_and_opening_balance() {
        let mut conn = db();
        let later = now() + chrono::Duration::days(1);
        let saved = update(&mut conn, 1, fields(" Wallet ", 32_000), later).unwrap();
        assert_eq!(
            (saved.name.as_str(), saved.opening_balance),
            ("Wallet", Money(32_000))
        );
        assert_eq!(get(&conn, 1, today()).unwrap(), saved);
        let (created, updated): (String, String) = conn
            .query_row(
                "SELECT created_at, updated_at FROM accounts WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            updated,
            later.to_utc().format("%Y-%m-%dT%H:%M:%SZ").to_string()
        );
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
        not_found(get(&conn, 99, today()));
        not_found(update(&mut conn, 99, fields("Gone", 0), now()));
    }

    #[test]
    fn reorder_sets_the_order_given() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        let wallet = add(&mut conn, "Wallet");
        reorder(&mut conn, vec![wallet.id, 1, bank.id], now()).unwrap();
        let names: Vec<String> = list(&conn, today())
            .unwrap()
            .accounts
            .into_iter()
            .map(|a| a.name)
            .collect();
        assert_eq!(names, ["Wallet", "Cash", "Bank"]);

        assert!(matches!(
            reorder(&mut conn, vec![bank.id, 99], now()),
            Err(AppError::NotFound { id: 99, .. })
        ));
        assert_eq!(
            field_of(reorder(&mut conn, vec![bank.id, bank.id], now())),
            "ids"
        );
        let unchanged: Vec<String> = list(&conn, today())
            .unwrap()
            .accounts
            .into_iter()
            .map(|a| a.name)
            .collect();
        assert_eq!(
            unchanged,
            ["Wallet", "Cash", "Bank"],
            "a rejected reorder changes nothing"
        );
    }

    #[test]
    fn a_card_stores_its_amount_owed_as_negative() {
        let mut conn = db();
        let visa = create(&mut conn, card("Visa", 64_000), now()).unwrap();
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

        let edited = update(&mut conn, visa.id, card("Visa", 1_000).fields, now()).unwrap();
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
            assert_eq!(field_of(create(&mut conn, without(strip), now())), field);
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
            assert_eq!(field_of(create(&mut conn, input.clone(), now())), field);
            assert_eq!(
                field_of(update(&mut conn, 1, input.fields, now())),
                field,
                "editing Cash"
            );
        }
    }

    #[test]
    fn a_limit_is_1_to_the_largest_amount_and_days_are_1_to_31() {
        let mut conn = db();
        let with = |limit: i64, statement: i64, due: i64| {
            let mut input = card("Visa", 0);
            input.fields.credit_limit = Some(Money(limit));
            input.fields.statement_day = Some(statement);
            input.fields.due_day = Some(due);
            input
        };
        let mut create_with =
            |limit, statement, due| create(&mut conn, with(limit, statement, due), now());
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
            create(&mut conn, card(name, 0), now()).unwrap();
        }
        let cards = list(&conn, today())
            .unwrap()
            .accounts
            .into_iter()
            .filter(|a| a.kind == AccountKind::CreditCard)
            .count();
        assert_eq!(cards, 3);
    }

    #[test]
    fn update_can_change_the_card_fields() {
        let mut conn = db();
        let visa = create(&mut conn, card("Visa", 0), now()).unwrap();
        let mut changed = card("Visa", 0).fields;
        changed.credit_limit = Some(Money(2_000_000));
        changed.statement_day = Some(31);
        changed.due_day = Some(15);
        let saved = update(&mut conn, visa.id, changed, now()).unwrap();
        assert_eq!(
            (saved.credit_limit, saved.statement_day, saved.due_day),
            (Some(Money(2_000_000)), Some(31), Some(15))
        );
        // A card still needs its fields when edited.
        let mut stripped = card("Visa", 0).fields;
        stripped.due_day = None;
        assert_eq!(
            field_of(update(&mut conn, visa.id, stripped, now())),
            "dueDay"
        );
    }

    #[test]
    fn a_balance_adds_income_and_received_transfers_and_subtracts_the_rest() {
        let mut conn = db();
        let bank = create(&mut conn, new("Bank", AccountKind::Bank, 100_000), now()).unwrap();
        let on = today();
        txn(&conn, "income", 5_000, on, 1, None);
        txn(&conn, "expense", 1_200, on, 1, None);
        txn(&conn, "transfer", 1_000, on, 1, Some(bank.id));
        txn(&conn, "transfer", 300, on, bank.id, Some(1));
        assert_eq!(balance_of(&conn, 1, on), 3_100);
        assert_eq!(balance_of(&conn, bank.id, on), 100_700);
        let all = list(&conn, on).unwrap();
        assert_eq!(
            all.accounts.iter().map(|a| a.balance.0).collect::<Vec<_>>(),
            [3_100, 100_700]
        );
        assert_eq!(all.net_balance, Money(103_800));
    }

    #[test]
    fn future_dated_rows_are_left_out() {
        let conn = db();
        txn(&conn, "income", 5_000, d(2026, 11, 3), 1, None);
        txn(&conn, "income", 7_000, d(2026, 11, 4), 1, None);
        assert_eq!(balance_of(&conn, 1, d(2026, 11, 3)), 5_000);
        assert_eq!(balance_of(&conn, 1, d(2026, 11, 4)), 12_000);
    }

    #[test]
    fn a_card_shows_its_amount_owed_and_credit() {
        let mut conn = db();
        let visa = create(&mut conn, card("Visa", 64_000), now()).unwrap();
        let on = today();
        // CC-2: opening owed + expenses − payments − refunds.
        txn(&conn, "expense", 1_000, on, visa.id, None);
        txn(&conn, "income", 500, on, visa.id, None);
        txn(&conn, "transfer", 2_000, on, 1, Some(visa.id));
        let visa = get(&conn, visa.id, on).unwrap();
        assert_eq!(visa.balance, Money(-62_500));
        let summary = visa.card.expect("a card has a summary");
        assert_eq!(
            (
                summary.amount_owed,
                summary.available_credit,
                summary.utilization_percent
            ),
            (Money(62_500), Money(937_500), 6)
        );
        assert_eq!(get(&conn, 1, on).unwrap().card, None, "Cash isn't a card");
    }

    #[test]
    fn cycle_spending_counts_only_this_cycles_expenses_so_far() {
        let mut conn = db();
        let visa = create(&mut conn, card("Visa", 0), now()).unwrap();
        txn(&conn, "expense", 100, d(2026, 10, 20), visa.id, None);
        txn(&conn, "expense", 200, d(2026, 10, 21), visa.id, None);
        txn(&conn, "expense", 400, d(2026, 10, 26), visa.id, None);
        txn(&conn, "income", 800, d(2026, 10, 22), visa.id, None);
        txn(&conn, "expense", 1_600, d(2026, 10, 27), visa.id, None);
        txn(&conn, "expense", 3_200, d(2026, 10, 23), 1, None);
        let summary = get(&conn, visa.id, d(2026, 10, 26)).unwrap().card.unwrap();
        assert_eq!(summary.cycle_spending, Money(600));
        // UI_FLOWS §5.3: Oct 21 – Nov 20, due Nov 5.
        assert_eq!(
            (summary.cycle_start, summary.cycle_end, summary.due_date),
            (d(2026, 10, 21), d(2026, 11, 20), d(2026, 11, 5))
        );
    }

    #[test]
    fn the_net_balance_counts_archived_accounts_and_subtracts_card_debt() {
        let mut conn = db();
        let bank = create(&mut conn, new("Bank", AccountKind::Bank, 100_000), now()).unwrap();
        conn.execute("UPDATE accounts SET archived = 1 WHERE id = ?1", [bank.id])
            .unwrap();
        create(&mut conn, card("Visa", 64_000), now()).unwrap();
        assert_eq!(list(&conn, today()).unwrap().net_balance, Money(36_000));
    }

    fn conflict(result: Result<(), AppError>) -> String {
        match result {
            Err(AppError::Conflict { message }) => message,
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    fn is_archived(conn: &Connection, id: i64) -> bool {
        get(conn, id, today()).unwrap().archived
    }

    #[test]
    fn a_zero_balance_account_archives_without_a_warning() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        archive(&mut conn, bank.id, false, now()).unwrap();
        assert!(is_archived(&conn, bank.id));
        // Archiving it again changes nothing.
        archive(&mut conn, bank.id, false, now()).unwrap();
        assert!(is_archived(&conn, bank.id));
    }

    #[test]
    fn a_balance_other_than_0_needs_the_warning_acknowledged() {
        let mut conn = db();
        let wallet = create(
            &mut conn,
            new("Old Wallet", AccountKind::Cash, 4_500),
            now(),
        )
        .unwrap();
        let message = conflict(archive(&mut conn, wallet.id, false, now()));
        assert!(message.contains("net balance"), "{message}");
        assert!(!message.contains("45"), "never the amount: {message}");
        assert!(!is_archived(&conn, wallet.id));
        archive(&mut conn, wallet.id, true, now()).unwrap();
        assert!(is_archived(&conn, wallet.id));

        // Card debt counts too, and a future-dated row doesn't.
        let visa = create(&mut conn, card("Visa", 0), now()).unwrap();
        txn(
            &conn,
            "expense",
            100,
            today() + chrono::Duration::days(1),
            visa.id,
            None,
        );
        archive(&mut conn, visa.id, false, now()).unwrap();
        let amex = create(&mut conn, card("Amex", 100), now()).unwrap();
        conflict(archive(&mut conn, amex.id, false, now()));
    }

    #[test]
    fn the_last_active_account_cannot_be_archived_or_deleted() {
        let mut conn = db();
        let old = add(&mut conn, "Old");
        archive(&mut conn, old.id, false, now()).unwrap();
        let keep = "Keep at least one active account";
        assert_eq!(conflict(archive(&mut conn, 1, true, now())), keep);
        assert_eq!(conflict(delete(&mut conn, 1, now())), keep);
        assert!(!is_archived(&conn, 1));
        // An archived one can still go, since Cash stays active.
        delete(&mut conn, old.id, now()).unwrap();
    }

    #[test]
    fn unarchive_restores_an_account() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        archive(&mut conn, bank.id, false, now()).unwrap();
        unarchive(&mut conn, bank.id, now()).unwrap();
        assert!(!is_archived(&conn, bank.id));
    }

    #[test]
    fn an_unused_account_can_be_deleted() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        delete(&mut conn, bank.id, now()).unwrap();
        assert!(matches!(
            get(&conn, bank.id, today()),
            Err(AppError::NotFound { .. })
        ));
        assert_eq!(list(&conn, today()).unwrap().accounts.len(), 1);
    }

    #[test]
    fn an_account_with_transactions_can_only_be_archived() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        let savings = add(&mut conn, "Savings");
        txn(&conn, "transfer", 100, today(), 1, Some(bank.id));
        txn(&conn, "transfer", 100, today(), bank.id, Some(1));
        assert!(get(&conn, bank.id, today()).unwrap().in_use);
        assert!(get(&conn, 1, today()).unwrap().in_use);
        assert!(!get(&conn, savings.id, today()).unwrap().in_use);
        let message = conflict(delete(&mut conn, bank.id, now()));
        assert!(message.contains("archived"), "{message}");
        assert!(get(&conn, bank.id, today()).is_ok());

        // Only ever on the receiving side of a transfer still counts as used.
        let to_only = add(&mut conn, "To only");
        txn(&conn, "transfer", 100, today(), 1, Some(to_only.id));
        conflict(delete(&mut conn, to_only.id, now()));
    }

    #[test]
    fn archive_unarchive_and_delete_of_a_missing_account_are_not_found() {
        let mut conn = db();
        let not_found = |result: Result<(), AppError>| {
            assert!(
                matches!(result, Err(AppError::NotFound { id: 99, .. })),
                "{result:?}"
            );
        };
        not_found(archive(&mut conn, 99, true, now()));
        not_found(unarchive(&mut conn, 99, now()));
        not_found(delete(&mut conn, 99, now()));
    }

    #[test]
    fn reordering_some_accounts_puts_the_rest_after_them_in_their_old_order() {
        let mut conn = db();
        let bank = add(&mut conn, "Bank");
        let old = add(&mut conn, "Old");
        let wallet = add(&mut conn, "Wallet");
        archive(&mut conn, old.id, false, now()).unwrap();
        // The list screen only drags the active accounts.
        reorder(&mut conn, vec![wallet.id, 1, bank.id], now()).unwrap();
        let order: Vec<(String, i64)> = list(&conn, today())
            .unwrap()
            .accounts
            .into_iter()
            .map(|a| (a.name, a.sort_order))
            .collect();
        assert_eq!(
            order,
            [
                ("Wallet".into(), 0),
                ("Cash".into(), 1),
                ("Bank".into(), 2),
                ("Old".into(), 3)
            ]
        );
    }

    #[test]
    fn days_outside_1_to_31_get_a_field_error_whatever_the_number() {
        let mut conn = db();
        for bad in [-1, 300, i64::MAX] {
            let mut input = card("Visa", 0);
            input.fields.statement_day = Some(bad);
            assert_eq!(
                field_of(create(&mut conn, input, now())),
                "statementDay",
                "{bad}"
            );
            let mut input = card("Visa", 0);
            input.fields.due_day = Some(bad);
            assert_eq!(field_of(create(&mut conn, input, now())), "dueDay", "{bad}");
        }
    }

    #[test]
    fn unarchiving_an_active_account_changes_nothing() {
        let mut conn = db();
        conn.execute("UPDATE accounts SET updated_at = 'before' WHERE id = 1", [])
            .unwrap();
        unarchive(&mut conn, 1, now()).unwrap();
        let updated: String = conn
            .query_row("SELECT updated_at FROM accounts WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(updated, "before");
    }
}
