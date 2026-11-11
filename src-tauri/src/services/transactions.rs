use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::money::Money;
use crate::error::AppError;
use crate::repo;
use crate::services::categories::{self, CategoryKind};
use crate::services::period::{self, PeriodQuery};
use crate::services::{accounts, invalid, not_found, stamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum TransactionKind {
    Income,
    Expense,
    Transfer,
}

/// What the quick-add and edit form sends (UI_FLOWS §4). A recurring rule's template is one too.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransactionFields {
    pub kind: TransactionKind,
    pub amount: Money,
    /// `YYYY-MM-DD`. Text, so a bad date is a Validation error on `date`, not a decoding failure.
    pub date: String,
    /// The source account for a transfer.
    #[ts(type = "number")]
    pub account_id: i64,
    /// Transfer only.
    #[ts(type = "number | null")]
    pub to_account_id: Option<i64>,
    /// Income and expense only.
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    pub payee: Option<String>,
    #[ts(type = "number | null")]
    pub payment_method_id: Option<i64>,
    pub note: Option<String>,
}

/// One stored transaction (DATA_MODEL §3.4). The screens look names up in the account, category
/// and payment method lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Transaction {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: TransactionKind,
    pub amount: Money,
    pub date: NaiveDate,
    #[ts(type = "number")]
    pub account_id: i64,
    #[ts(type = "number | null")]
    pub to_account_id: Option<i64>,
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    pub payee: Option<String>,
    #[ts(type = "number | null")]
    pub payment_method_id: Option<i64>,
    pub note: Option<String>,
}

/// The settings key quick-add's default account comes from (DATA_MODEL §5.8).
const LAST_USED_ACCOUNT: &str = "last_used_account_id";

/// What quick-add starts with (UI_FLOWS §4). A duplicate uses `today` too (TXN-2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransactionDefaults {
    #[ts(type = "number | null")]
    pub account_id: Option<i64>,
    pub today: NaiveDate,
}

/// What Pay bill's transfer starts with (CC-9, UI_FLOWS F2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayBillDefaults {
    #[ts(type = "number | null")]
    pub from_account_id: Option<i64>,
    /// The amount owed, or `None` when nothing is owed, so the field starts empty.
    pub amount: Option<Money>,
    pub today: NaiveDate,
}

/// The Transactions screen's filters (TXN-4, UI_FLOWS §3). `None` means any.
#[derive(Clone, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransactionFilter {
    pub period: PeriodQuery,
    pub kind: Option<TransactionKind>,
    /// Includes transfers into the account.
    #[ts(type = "number | null")]
    pub account_id: Option<i64>,
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    #[ts(type = "number | null")]
    pub payment_method_id: Option<i64>,
    /// Looks in the payee and the note.
    pub search: Option<String>,
}

/// One page of the list, with figures that cover every matching row (ARCHITECTURE §4.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransactionList {
    pub items: Vec<Transaction>,
    pub has_more: bool,
    pub summary: TransactionSummary,
    /// For every date on this page, including that date's rows on other pages.
    pub day_nets: Vec<DayNet>,
}

/// Transfers are counted, but never as income or expense (RPT-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TransactionSummary {
    #[ts(type = "number")]
    pub count: i64,
    pub income: Money,
    pub expense: Money,
}

/// A day header's figure: income − expense (UI_FLOWS §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DayNet {
    pub date: NaiveDate,
    pub net: Money,
}

pub const PAGE_SIZE: u32 = 100;

/// Page `page` (from 0) of the matching transactions, newest first (TXN-3, TXN-4).
pub fn list(
    conn: &Connection,
    filter: TransactionFilter,
    page: u32,
    today: NaiveDate,
) -> Result<TransactionList, AppError> {
    let period = period::resolve(conn, &filter.period, today)?;
    let (start, end) = (period.start().to_string(), period.end().to_string());
    // shortcut: SQLite's LIKE ignores case for ASCII letters only, so "Ñ" doesn't match "ñ".
    let search = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", like_text(s)));
    let f = repo::transactions::Filter {
        start: &start,
        end: &end,
        kind: filter.kind.map(kind_text),
        account_id: filter.account_id,
        category_id: filter.category_id,
        payment_method_id: filter.payment_method_id,
        search: search.as_deref(),
    };
    let mut rows = repo::transactions::page(
        conn,
        &f,
        PAGE_SIZE + 1,
        i64::from(page) * i64::from(PAGE_SIZE),
    )?;
    let has_more = rows.len() > PAGE_SIZE as usize;
    rows.truncate(PAGE_SIZE as usize);
    let items = rows.into_iter().map(view).collect::<Result<Vec<_>, _>>()?;
    let (count, income, expense) = repo::transactions::totals(conn, &f)?;
    let day_nets = match (items.last(), items.first()) {
        (Some(oldest), Some(newest)) => {
            // Every matching row between the page's oldest and newest dates is on a page date.
            let (start, end) = (oldest.date.to_string(), newest.date.to_string());
            let page_dates = repo::transactions::Filter {
                start: &start,
                end: &end,
                ..f
            };
            repo::transactions::day_nets(conn, &page_dates)?
                .into_iter()
                .map(|(date, net)| {
                    Ok(DayNet {
                        date: parse_date(&date).ok_or_else(|| AppError::Database {
                            message: "A transaction has an invalid date".into(),
                        })?,
                        net: Money(net),
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?
        }
        _ => Vec::new(),
    };
    Ok(TransactionList {
        items,
        has_more,
        summary: TransactionSummary {
            count,
            income: Money(income),
            expense: Money(expense),
        },
        day_nets,
    })
}

/// A stored `YYYY-MM-DD` date.
pub(crate) fn parse_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").ok()
}

/// The last account used, if it's still active, else the first active one (DATA_MODEL §5.8).
pub fn defaults(conn: &Connection, today: NaiveDate) -> Result<TransactionDefaults, AppError> {
    let last = repo::settings::get(conn, LAST_USED_ACCOUNT)?.and_then(|v| v.parse().ok());
    let last = match last {
        Some(id) => repo::accounts::find(conn, id, today)?.filter(|a| !a.archived),
        None => None,
    };
    let account_id = match last {
        Some(account) => Some(account.id),
        None => repo::accounts::first_active(conn, true)?,
    };
    Ok(TransactionDefaults { account_id, today })
}

/// DATA_MODEL §5.8: the latest non-card source, else the first non-card account, and the amount owed.
pub fn pay_bill_defaults(
    conn: &Connection,
    card_id: i64,
    today: NaiveDate,
) -> Result<PayBillDefaults, AppError> {
    let owed = accounts::get(conn, card_id, today)?
        .card
        .ok_or_else(|| AppError::Conflict {
            message: "Only a credit card has a bill to pay".into(),
        })?
        .amount_owed;
    let from_account_id = match repo::transactions::latest_non_card_source(conn)? {
        Some(id) => Some(id),
        None => repo::accounts::first_active(conn, false)?,
    };
    Ok(PayBillDefaults {
        from_account_id,
        amount: (owed > Money::ZERO).then_some(owed),
        today,
    })
}

/// `text` as plain text in a LIKE pattern with `ESCAPE '\'`: its wildcards and escapes escaped.
fn like_text(text: &str) -> String {
    text.replace('\\', r"\\")
        .replace('%', r"\%")
        .replace('_', r"\_")
}

/// A payee used before, and the category choosing it fills in (TXN-6).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PayeeSuggestion {
    pub payee: String,
    /// The category of the latest row of this type with this payee; none if it's archived now.
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
}

/// How many payees the form suggests at once.
const SUGGESTIONS: u32 = 8;

/// The payees used on `kind` rows that start with `prefix` (ASCII case ignored), most recently used
/// first (TXN-6, DATA_MODEL §3.4). Nothing typed suggests nothing.
pub fn suggest_payees(
    conn: &Connection,
    prefix: &str,
    kind: CategoryKind,
) -> Result<Vec<PayeeSuggestion>, AppError> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = format!("{}%", like_text(prefix));
    Ok(
        repo::transactions::payees(conn, categories::kind_text(kind), &pattern, SUGGESTIONS)?
            .into_iter()
            .map(|(payee, category_id)| PayeeSuggestion { payee, category_id })
            .collect(),
    )
}

pub fn get(conn: &Connection, id: i64) -> Result<Transaction, AppError> {
    view(repo::transactions::find(conn, id)?.ok_or_else(|| not_found("transaction", id))?)
}

/// Saves a new transaction and remembers its account for the next quick-add (TXN-1).
pub fn create(
    conn: &mut Connection,
    mut fields: TransactionFields,
    now: DateTime<Local>,
) -> Result<Transaction, AppError> {
    let tx = conn.transaction()?;
    validate(&tx, &mut fields, None, now.date_naive(), false)?;
    let id = repo::transactions::insert(&tx, &values(&fields), &stamp(now))?;
    repo::settings::set(&tx, LAST_USED_ACCOUNT, &fields.account_id.to_string())?;
    let saved = get(&tx, id)?;
    tx.commit()?;
    Ok(saved)
}

pub fn update(
    conn: &mut Connection,
    id: i64,
    mut fields: TransactionFields,
    now: DateTime<Local>,
) -> Result<Transaction, AppError> {
    let tx = conn.transaction()?;
    let stored = get(&tx, id)?;
    validate(&tx, &mut fields, Some(&stored), now.date_naive(), false)?;
    repo::transactions::update(&tx, id, &values(&fields), &stamp(now))?;
    let saved = get(&tx, id)?;
    tx.commit()?;
    Ok(saved)
}

pub fn delete(conn: &mut Connection, id: i64) -> Result<(), AppError> {
    let tx = conn.transaction()?;
    if repo::transactions::delete(&tx, id)? == 0 {
        return Err(not_found("transaction", id));
    }
    tx.commit()?;
    Ok(())
}

/// Every rule in DATA_MODEL §6, after trimming the text. `stored` is the row being edited: it may
/// keep an archived record it already uses, but a new one can't pick one. A recurring rule's
/// template passes `allow_future`, since its date is the start date (DATA_MODEL §4.1).
pub(crate) fn validate(
    conn: &Connection,
    f: &mut TransactionFields,
    stored: Option<&Transaction>,
    today: NaiveDate,
    allow_future: bool,
) -> Result<(), AppError> {
    if f.amount <= Money::ZERO {
        return Err(invalid("amount", "Enter an amount above 0"));
    }
    if f.amount > Money::MAX {
        return Err(invalid("amount", "That amount is too large"));
    }
    let date = real_date(&f.date).ok_or_else(|| invalid("date", "Enter a real date"))?;
    // Phases 1–2 only: Phase 3 allows future dates (TXN-8).
    if date > today && !allow_future {
        return Err(invalid("date", "Date can't be in the future"));
    }
    f.payee = clean(f.payee.take());
    f.note = clean(f.note.take());
    if f.payee.as_ref().is_some_and(|p| p.chars().count() > 100) {
        return Err(invalid("payee", "Enter up to 100 characters"));
    }
    if f.note.as_ref().is_some_and(|n| n.chars().count() > 500) {
        return Err(invalid("note", "Enter up to 500 characters"));
    }
    let kept = |pick: fn(&Transaction) -> Option<i64>| stored.and_then(pick);
    // An account the edited row uses on either side stays usable, so a transfer can be reversed.
    let uses = |id: i64| stored.is_some_and(|s| s.account_id == id || s.to_account_id == Some(id));
    check_account(conn, "accountId", f.account_id, uses(f.account_id), today)?;
    if f.kind == TransactionKind::Transfer {
        for (field, present) in [
            ("categoryId", f.category_id.is_some()),
            ("payee", f.payee.is_some()),
            ("paymentMethodId", f.payment_method_id.is_some()),
        ] {
            if present {
                return Err(invalid(field, "A transfer doesn't have this field"));
            }
        }
        let to = f
            .to_account_id
            .ok_or_else(|| invalid("toAccountId", "Choose the account the money goes to"))?;
        if to == f.account_id {
            return Err(invalid("toAccountId", "Choose a different account"));
        }
        return check_account(conn, "toAccountId", to, uses(to), today);
    }
    if f.to_account_id.is_some() {
        return Err(invalid("toAccountId", "Only a transfer has this field"));
    }
    let category_id = f
        .category_id
        .ok_or_else(|| invalid("categoryId", "Choose a category"))?;
    let category = repo::categories::find(conn, category_id)?
        .ok_or_else(|| invalid("categoryId", "This category no longer exists"))?;
    if category.kind != kind_text(f.kind) {
        return Err(invalid(
            "categoryId",
            &format!("Choose an {} category", kind_text(f.kind)),
        ));
    }
    if category.archived && kept(|s| s.category_id) != Some(category_id) {
        return Err(invalid("categoryId", "This category is archived"));
    }
    if let Some(method_id) = f.payment_method_id {
        let method = repo::payment_methods::find(conn, method_id)?
            .ok_or_else(|| invalid("paymentMethodId", "This payment method no longer exists"))?;
        if method.archived && kept(|s| s.payment_method_id) != Some(method_id) {
            return Err(invalid(
                "paymentMethodId",
                "This payment method is archived",
            ));
        }
    }
    Ok(())
}

/// The account must exist, and be active unless the edited row already uses it.
fn check_account(
    conn: &Connection,
    field: &str,
    id: i64,
    kept: bool,
    today: NaiveDate,
) -> Result<(), AppError> {
    let account = repo::accounts::find(conn, id, today)?
        .ok_or_else(|| invalid(field, "This account no longer exists"))?;
    if account.archived && !kept {
        return Err(invalid(field, "This account is archived"));
    }
    Ok(())
}

/// `text` as a date, only in the stored `YYYY-MM-DD` form.
pub(crate) fn real_date(text: &str) -> Option<NaiveDate> {
    // Parsing also takes "2026-1-5" and "-0001-01-01"; only the stored YYYY-MM-DD form is
    // accepted, which the column's CHECK requires.
    parse_date(text).filter(|d| text.len() == 10 && d.to_string() == text)
}

/// Trimmed, and `None` when nothing is left (DATA_MODEL §6).
fn clean(text: Option<String>) -> Option<String> {
    text.map(|t| t.trim().to_owned()).filter(|t| !t.is_empty())
}

pub(crate) fn values(f: &TransactionFields) -> repo::transactions::Values<'_> {
    repo::transactions::Values {
        kind: kind_text(f.kind),
        amount: f.amount.0,
        date: &f.date,
        account_id: f.account_id,
        to_account_id: f.to_account_id,
        category_id: f.category_id,
        payee: f.payee.as_deref(),
        payment_method_id: f.payment_method_id,
        note: f.note.as_deref(),
    }
}

pub(crate) fn view(row: repo::transactions::Row) -> Result<Transaction, AppError> {
    let bad = |what: &str| AppError::Database {
        message: format!("Transaction {} has an invalid {what}", row.id),
    };
    let kind = kind_of(&row.kind).ok_or_else(|| bad("kind"))?;
    let date = parse_date(&row.date).ok_or_else(|| bad("date"))?;
    Ok(Transaction {
        id: row.id,
        kind,
        amount: Money(row.amount),
        date,
        account_id: row.account_id,
        to_account_id: row.to_account_id,
        category_id: row.category_id,
        payee: row.payee,
        payment_method_id: row.payment_method_id,
        note: row.note,
    })
}

pub(crate) fn kind_of(text: &str) -> Option<TransactionKind> {
    [
        TransactionKind::Income,
        TransactionKind::Expense,
        TransactionKind::Transfer,
    ]
    .into_iter()
    .find(|k| kind_text(*k) == text)
}

pub(crate) fn kind_text(kind: TransactionKind) -> &'static str {
    match kind {
        TransactionKind::Income => "income",
        TransactionKind::Expense => "expense",
        TransactionKind::Transfer => "transfer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::services::accounts::{self, AccountFields, AccountKind, NewAccount};
    use crate::services::payment_methods;
    use crate::test_support::{db, now};
    use chrono::Days;
    use rusqlite::Connection;

    fn today() -> NaiveDate {
        now().date_naive()
    }

    fn id_of(conn: &Connection, table: &str, name: &str) -> i64 {
        conn.query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1"),
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn food(conn: &Connection) -> i64 {
        id_of(conn, "categories", "Food & Groceries")
    }

    fn salary(conn: &Connection) -> i64 {
        id_of(conn, "categories", "Salary")
    }

    fn cash(conn: &Connection) -> i64 {
        id_of(conn, "accounts", "Cash")
    }

    /// A bank account holding `opening`.
    fn bank(conn: &mut Connection, name: &str, opening: i64) -> i64 {
        accounts::create(
            conn,
            NewAccount {
                kind: AccountKind::Bank,
                fields: AccountFields {
                    name: name.into(),
                    opening_balance: Money(opening),
                    credit_limit: None,
                    statement_day: None,
                    due_day: None,
                },
            },
            now(),
        )
        .unwrap()
        .id
    }

    fn balance(conn: &Connection, id: i64) -> i64 {
        accounts::get(conn, id, today()).unwrap().balance.0
    }

    fn expense(conn: &Connection, amount: i64, account: i64) -> TransactionFields {
        TransactionFields {
            kind: TransactionKind::Expense,
            amount: Money(amount),
            date: today().to_string(),
            account_id: account,
            to_account_id: None,
            category_id: Some(food(conn)),
            payee: None,
            payment_method_id: None,
            note: None,
        }
    }

    fn income(conn: &Connection, amount: i64, account: i64) -> TransactionFields {
        TransactionFields {
            kind: TransactionKind::Income,
            category_id: Some(salary(conn)),
            ..expense(conn, amount, account)
        }
    }

    fn transfer(amount: i64, from: i64, to: i64) -> TransactionFields {
        TransactionFields {
            kind: TransactionKind::Transfer,
            amount: Money(amount),
            date: today().to_string(),
            account_id: from,
            to_account_id: Some(to),
            category_id: None,
            payee: None,
            payment_method_id: None,
            note: None,
        }
    }

    fn field_of<T: std::fmt::Debug>(result: Result<T, AppError>) -> String {
        match result {
            Err(AppError::Validation { field, .. }) => field,
            other => panic!("expected a validation error, got {other:?}"),
        }
    }

    fn last_used(conn: &Connection) -> Option<String> {
        crate::repo::settings::get(conn, "last_used_account_id").unwrap()
    }

    #[test]
    fn create_saves_an_expense_and_lowers_the_balance() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 100_000);
        let mut fields = expense(&conn, 2_550, bank);
        fields.payee = Some("Market".into());
        fields.note = Some("weekly shop".into());
        let saved = create(&mut conn, fields, now()).unwrap();
        assert_eq!(
            saved,
            Transaction {
                id: saved.id,
                kind: TransactionKind::Expense,
                amount: Money(2_550),
                date: today(),
                account_id: bank,
                to_account_id: None,
                category_id: Some(food(&conn)),
                payee: Some("Market".into()),
                payment_method_id: None,
                note: Some("weekly shop".into()),
            }
        );
        assert_eq!(get(&conn, saved.id).unwrap(), saved);
        assert_eq!(balance(&conn, bank), 97_450);
    }

    #[test]
    fn income_raises_the_balance() {
        let mut conn = db();
        let cash = cash(&conn);
        {
            let f = income(&conn, 420_000, cash);
            create(&mut conn, f, now())
        }
        .unwrap();
        assert_eq!(balance(&conn, cash), 420_000);
    }

    #[test]
    fn a_transfer_moves_money_between_both_accounts() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 100_000);
        let cash = cash(&conn);
        create(&mut conn, transfer(30_000, bank, cash), now()).unwrap();
        assert_eq!(balance(&conn, bank), 70_000);
        assert_eq!(balance(&conn, cash), 30_000);
    }

    #[test]
    fn editing_and_deleting_update_the_balances() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 100_000);
        let cash = cash(&conn);
        let saved = {
            let f = expense(&conn, 10_000, bank);
            create(&mut conn, f, now())
        }
        .unwrap();
        {
            let f = expense(&conn, 25_000, bank);
            update(&mut conn, saved.id, f, now())
        }
        .unwrap();
        assert_eq!(balance(&conn, bank), 75_000);
        // Moving it to another account gives the first one its money back.
        {
            let f = expense(&conn, 25_000, cash);
            update(&mut conn, saved.id, f, now())
        }
        .unwrap();
        assert_eq!(balance(&conn, bank), 100_000);
        assert_eq!(balance(&conn, cash), -25_000);
        delete(&mut conn, saved.id).unwrap();
        assert_eq!(balance(&conn, cash), 0);
        assert!(matches!(
            get(&conn, saved.id),
            Err(AppError::NotFound { .. })
        ));
    }

    #[test]
    fn amount_must_be_above_0_and_at_most_the_max() {
        let mut conn = db();
        let cash = cash(&conn);
        for bad in [0, -1, Money::MAX.0 + 1] {
            assert_eq!(
                field_of({
                    let f = expense(&conn, bad, cash);
                    create(&mut conn, f, now())
                }),
                "amount",
                "{bad}"
            );
        }
        assert!(
            {
                let f = expense(&conn, Money::MAX.0, cash);
                create(&mut conn, f, now())
            }
            .is_ok()
        );
    }

    #[test]
    fn the_date_must_be_real_and_not_in_the_future() {
        let mut conn = db();
        let cash = cash(&conn);
        for bad in ["2026-02-30", "2026-1-5", "x", "", "-0001-01-01"] {
            let fields = TransactionFields {
                date: bad.into(),
                ..expense(&conn, 100, cash)
            };
            assert_eq!(field_of(create(&mut conn, fields, now())), "date", "{bad}");
        }
        let tomorrow = TransactionFields {
            date: today().checked_add_days(Days::new(1)).unwrap().to_string(),
            ..expense(&conn, 100, cash)
        };
        match create(&mut conn, tomorrow, now()) {
            Err(AppError::Validation { field, message }) => {
                assert_eq!(
                    (field.as_str(), message.as_str()),
                    ("date", "Date can't be in the future")
                );
            }
            other => panic!("expected a validation error, got {other:?}"),
        }
        let leap = TransactionFields {
            date: "2024-02-29".into(),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(
            create(&mut conn, leap, now()).unwrap().date,
            NaiveDate::from_ymd_opt(2024, 2, 29).unwrap()
        );
    }

    #[test]
    fn a_transfer_has_no_category_payee_or_method() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 0);
        let cash = cash(&conn);
        let method = id_of(&conn, "payment_methods", "Cash");
        let with_category = TransactionFields {
            category_id: Some(food(&conn)),
            ..transfer(100, bank, cash)
        };
        let with_payee = TransactionFields {
            payee: Some("Someone".into()),
            ..transfer(100, bank, cash)
        };
        let with_method = TransactionFields {
            payment_method_id: Some(method),
            ..transfer(100, bank, cash)
        };
        assert_eq!(
            field_of(create(&mut conn, with_category, now())),
            "categoryId"
        );
        assert_eq!(field_of(create(&mut conn, with_payee, now())), "payee");
        assert_eq!(
            field_of(create(&mut conn, with_method, now())),
            "paymentMethodId"
        );
        // Blank text is no payee at all.
        let blank = TransactionFields {
            payee: Some("  ".into()),
            ..transfer(100, bank, cash)
        };
        assert_eq!(create(&mut conn, blank, now()).unwrap().payee, None);
    }

    #[test]
    fn a_transfer_needs_a_different_destination() {
        let mut conn = db();
        let cash = cash(&conn);
        let missing = TransactionFields {
            to_account_id: None,
            ..transfer(100, cash, cash)
        };
        assert_eq!(field_of(create(&mut conn, missing, now())), "toAccountId");
        assert_eq!(
            field_of(create(&mut conn, transfer(100, cash, cash), now())),
            "toAccountId"
        );
    }

    #[test]
    fn income_and_expense_need_a_category_of_their_kind() {
        let mut conn = db();
        let cash = cash(&conn);
        let none = TransactionFields {
            category_id: None,
            ..expense(&conn, 100, cash)
        };
        assert_eq!(field_of(create(&mut conn, none, now())), "categoryId");
        let wrong_kind = TransactionFields {
            category_id: Some(salary(&conn)),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(field_of(create(&mut conn, wrong_kind, now())), "categoryId");
        let wrong_kind = TransactionFields {
            category_id: Some(food(&conn)),
            ..income(&conn, 100, cash)
        };
        assert_eq!(field_of(create(&mut conn, wrong_kind, now())), "categoryId");
    }

    #[test]
    fn income_and_expense_have_no_destination() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 0);
        let cash = cash(&conn);
        let fields = TransactionFields {
            to_account_id: Some(bank),
            ..income(&conn, 100, cash)
        };
        assert_eq!(field_of(create(&mut conn, fields, now())), "toAccountId");
    }

    #[test]
    fn text_is_trimmed_and_limited() {
        let mut conn = db();
        let cash = cash(&conn);
        let fields = TransactionFields {
            payee: Some(format!(" {} ", "ñ".repeat(100))),
            note: Some(format!(" {} ", "ñ".repeat(500))),
            ..expense(&conn, 100, cash)
        };
        let saved = create(&mut conn, fields, now()).unwrap();
        assert_eq!(saved.payee.unwrap().chars().count(), 100);
        assert_eq!(saved.note.unwrap().chars().count(), 500);
        let blank = TransactionFields {
            payee: Some("".into()),
            note: Some("   ".into()),
            ..expense(&conn, 100, cash)
        };
        let saved = create(&mut conn, blank, now()).unwrap();
        assert_eq!((saved.payee, saved.note), (None, None));
        let long_payee = TransactionFields {
            payee: Some("a".repeat(101)),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(field_of(create(&mut conn, long_payee, now())), "payee");
        let long_note = TransactionFields {
            note: Some("a".repeat(501)),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(field_of(create(&mut conn, long_note, now())), "note");
    }

    #[test]
    fn a_new_transaction_cant_use_archived_records() {
        let mut conn = db();
        let old = bank(&mut conn, "Old bank", 0);
        let cash = cash(&conn);
        let method = id_of(&conn, "payment_methods", "Other");
        let dining = id_of(&conn, "categories", "Dining Out");
        accounts::archive(&mut conn, old, false, now()).unwrap();
        categories::archive(&mut conn, dining, now()).unwrap();
        payment_methods::archive(&mut conn, method, now()).unwrap();
        assert_eq!(
            field_of({
                let f = expense(&conn, 100, old);
                create(&mut conn, f, now())
            }),
            "accountId"
        );
        assert_eq!(
            field_of(create(&mut conn, transfer(100, cash, old), now())),
            "toAccountId"
        );
        let archived_category = TransactionFields {
            category_id: Some(dining),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(
            field_of(create(&mut conn, archived_category, now())),
            "categoryId"
        );
        let archived_method = TransactionFields {
            payment_method_id: Some(method),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(
            field_of(create(&mut conn, archived_method, now())),
            "paymentMethodId"
        );
    }

    #[test]
    fn an_edit_keeps_its_archived_records_but_cant_pick_new_ones() {
        let mut conn = db();
        let old = bank(&mut conn, "Old bank", 0);
        let other = bank(&mut conn, "Other bank", 0);
        let method = id_of(&conn, "payment_methods", "Other");
        let with_method = TransactionFields {
            payment_method_id: Some(method),
            ..expense(&conn, 100, old)
        };
        let saved = create(&mut conn, with_method.clone(), now()).unwrap();
        bank(&mut conn, "Keeps one active", 0);
        accounts::archive(&mut conn, old, true, now()).unwrap();
        accounts::archive(&mut conn, other, true, now()).unwrap();
        payment_methods::archive(&mut conn, method, now()).unwrap();
        let edited = TransactionFields {
            amount: Money(300),
            ..with_method
        };
        assert_eq!(
            update(&mut conn, saved.id, edited, now()).unwrap().amount,
            Money(300)
        );
        assert_eq!(
            field_of({
                let f = expense(&conn, 300, other);
                update(&mut conn, saved.id, f, now())
            }),
            "accountId"
        );
    }

    #[test]
    fn missing_records_are_not_found() {
        let mut conn = db();
        let cash = cash(&conn);
        assert!(matches!(get(&conn, 99), Err(AppError::NotFound { .. })));
        assert!(matches!(
            {
                let f = expense(&conn, 100, cash);
                update(&mut conn, 99, f, now())
            },
            Err(AppError::NotFound { .. })
        ));
        assert!(matches!(
            delete(&mut conn, 99),
            Err(AppError::NotFound { .. })
        ));
    }

    #[test]
    fn a_record_that_no_longer_exists_is_an_error_on_its_field() {
        let mut conn = db();
        let cash = cash(&conn);
        let gone_account = {
            let f = expense(&conn, 100, 99);
            create(&mut conn, f, now())
        };
        assert_eq!(field_of(gone_account), "accountId");
        assert_eq!(
            field_of(create(&mut conn, transfer(100, cash, 99), now())),
            "toAccountId"
        );
        let gone_category = TransactionFields {
            category_id: Some(999),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(
            field_of(create(&mut conn, gone_category, now())),
            "categoryId"
        );
        let gone_method = TransactionFields {
            payment_method_id: Some(999),
            ..expense(&conn, 100, cash)
        };
        assert_eq!(
            field_of(create(&mut conn, gone_method, now())),
            "paymentMethodId"
        );
    }

    #[test]
    fn an_edit_keeps_an_archived_account_it_uses_on_either_side() {
        let mut conn = db();
        let cash = cash(&conn);
        let old = bank(&mut conn, "Old bank", 10_000);
        let out = create(&mut conn, transfer(1_000, old, cash), now()).unwrap();
        let into = create(&mut conn, transfer(1_000, cash, old), now()).unwrap();
        accounts::archive(&mut conn, old, true, now()).unwrap();
        // Fixing the direction keeps the same two accounts.
        assert!(update(&mut conn, out.id, transfer(1_000, cash, old), now()).is_ok());
        // So does turning it into an expense on the account it used to receive.
        let f = expense(&conn, 1_000, old);
        assert!(update(&mut conn, into.id, f, now()).is_ok());
    }

    #[test]
    fn create_remembers_the_account_but_edit_doesnt() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 0);
        let cash = cash(&conn);
        assert_eq!(last_used(&conn), None);
        let saved = {
            let f = expense(&conn, 100, bank);
            create(&mut conn, f, now())
        }
        .unwrap();
        assert_eq!(last_used(&conn), Some(bank.to_string()));
        {
            let f = expense(&conn, 100, cash);
            update(&mut conn, saved.id, f, now())
        }
        .unwrap();
        assert_eq!(last_used(&conn), Some(bank.to_string()));
    }

    #[test]
    fn an_expense_can_become_a_transfer_on_edit() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 10_000);
        let cash = cash(&conn);
        let saved = {
            let f = expense(&conn, 1_000, bank);
            create(&mut conn, f, now())
        }
        .unwrap();
        let moved = update(&mut conn, saved.id, transfer(1_000, bank, cash), now()).unwrap();
        assert_eq!(
            (moved.kind, moved.category_id, moved.to_account_id),
            (TransactionKind::Transfer, None, Some(cash))
        );
        assert_eq!((balance(&conn, bank), balance(&conn, cash)), (9_000, 1_000));
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    /// Saves an expense of `amount` on `account`, dated `date`.
    fn spend(conn: &mut Connection, amount: i64, account: i64, date: NaiveDate) -> Transaction {
        let f = TransactionFields {
            date: date.to_string(),
            ..expense(conn, amount, account)
        };
        create(conn, f, now()).unwrap()
    }

    /// A card owing `owed`, with a 10,000.00 limit, closing on the 20th and due on the 5th.
    fn card(conn: &mut Connection, name: &str, owed: i64) -> i64 {
        accounts::create(
            conn,
            NewAccount {
                kind: AccountKind::CreditCard,
                fields: AccountFields {
                    name: name.into(),
                    opening_balance: Money(owed),
                    credit_limit: Some(Money(1_000_000)),
                    statement_day: Some(20),
                    due_day: Some(5),
                },
            },
            now(),
        )
        .unwrap()
        .id
    }

    #[test]
    fn defaults_use_the_last_account_or_the_first_active_one() {
        let mut conn = db();
        let cash = cash(&conn);
        // `today` is whatever the caller passes: the service never reads the clock.
        assert_eq!(
            defaults(&conn, d(2027, 1, 1)).unwrap(),
            TransactionDefaults {
                account_id: Some(cash),
                today: d(2027, 1, 1)
            }
        );
        let bank = bank(&mut conn, "Bank", 0);
        spend(&mut conn, 100, bank, today());
        assert_eq!(defaults(&conn, today()).unwrap().account_id, Some(bank));
        accounts::archive(&mut conn, bank, true, now()).unwrap();
        assert_eq!(defaults(&conn, today()).unwrap().account_id, Some(cash));
        // The first active one by sort order, not by id.
        let savings = accounts::create(
            &mut conn,
            NewAccount {
                kind: AccountKind::Bank,
                fields: AccountFields {
                    name: "Savings".into(),
                    opening_balance: Money(0),
                    credit_limit: None,
                    statement_day: None,
                    due_day: None,
                },
            },
            now(),
        )
        .unwrap()
        .id;
        accounts::reorder(&mut conn, vec![savings, cash], now()).unwrap();
        assert_eq!(defaults(&conn, today()).unwrap().account_id, Some(savings));
        for gone in ["999", "junk"] {
            repo::settings::set(&conn, LAST_USED_ACCOUNT, gone).unwrap();
            assert_eq!(defaults(&conn, today()).unwrap().account_id, Some(savings));
        }
    }

    #[test]
    fn duplicating_copies_the_row_with_todays_date() {
        let mut conn = db();
        let cash = cash(&conn);
        let mut f = expense(&conn, 1_234, cash);
        f.date = "2026-10-01".into();
        f.payee = Some("Market".into());
        f.note = Some("weekly shop".into());
        let original = create(&mut conn, f, now()).unwrap();
        // What the form does (ARCHITECTURE §4.5): load the row, take today from the defaults, save.
        let row = get(&conn, original.id).unwrap();
        let copy = TransactionFields {
            kind: row.kind,
            amount: row.amount,
            date: defaults(&conn, today()).unwrap().today.to_string(),
            account_id: row.account_id,
            to_account_id: row.to_account_id,
            category_id: row.category_id,
            payee: row.payee,
            payment_method_id: row.payment_method_id,
            note: row.note,
        };
        let copy = create(&mut conn, copy, now()).unwrap();
        assert_ne!(copy.id, original.id);
        assert_eq!(
            copy,
            Transaction {
                id: copy.id,
                date: today(),
                ..original.clone()
            }
        );
        assert_eq!(get(&conn, original.id).unwrap(), original);
    }

    #[test]
    fn pay_bill_defaults_pick_the_latest_non_card_source() {
        let mut conn = db();
        let cash = cash(&conn);
        let visa = card(&mut conn, "Visa", 50_000);
        let wallet = bank(&mut conn, "Wallet", 100_000);
        let bank = bank(&mut conn, "Bank", 100_000);
        let from = |conn: &Connection| {
            pay_bill_defaults(conn, visa, today())
                .unwrap()
                .from_account_id
        };
        assert_eq!(from(&conn), Some(cash));
        // The latest by date, then by id.
        spend(&mut conn, 100, bank, d(2026, 11, 2));
        spend(&mut conn, 100, wallet, d(2026, 11, 1));
        assert_eq!(from(&conn), Some(bank));
        // A card is never the From account.
        spend(&mut conn, 100, visa, today());
        assert_eq!(from(&conn), Some(bank));
        spend(&mut conn, 100, wallet, d(2026, 11, 2));
        assert_eq!(from(&conn), Some(wallet));
        accounts::archive(&mut conn, wallet, true, now()).unwrap();
        assert_eq!(from(&conn), Some(bank));
    }

    #[test]
    fn pay_bill_from_is_empty_without_an_active_non_card_account() {
        let mut conn = db();
        let cash = cash(&conn);
        let visa = card(&mut conn, "Visa", 50_000);
        accounts::archive(&mut conn, cash, false, now()).unwrap();
        assert_eq!(
            pay_bill_defaults(&conn, visa, today())
                .unwrap()
                .from_account_id,
            None
        );
    }

    #[test]
    fn pay_bill_amount_is_the_amount_owed_or_empty() {
        let mut conn = db();
        let cash = cash(&conn);
        let visa = card(&mut conn, "Visa", 50_000);
        assert_eq!(
            pay_bill_defaults(&conn, visa, today()).unwrap(),
            PayBillDefaults {
                from_account_id: Some(cash),
                amount: Some(Money(50_000)),
                today: today()
            }
        );
        create(&mut conn, transfer(50_000, cash, visa), now()).unwrap();
        assert_eq!(
            pay_bill_defaults(&conn, visa, today()).unwrap().amount,
            None
        );
        // Overpaid: credit on the card, nothing owed.
        create(&mut conn, transfer(1_000, cash, visa), now()).unwrap();
        assert_eq!(
            pay_bill_defaults(&conn, visa, today()).unwrap().amount,
            None
        );
    }

    #[test]
    fn pay_bill_needs_a_card() {
        let conn = db();
        assert!(matches!(
            pay_bill_defaults(&conn, cash(&conn), today()),
            Err(AppError::Conflict { .. })
        ));
        assert!(matches!(
            pay_bill_defaults(&conn, 999, today()),
            Err(AppError::NotFound { .. })
        ));
    }

    /// Every row this month, with no other filter.
    fn this_month() -> TransactionFilter {
        TransactionFilter {
            period: PeriodQuery::Month { anchor: None },
            kind: None,
            account_id: None,
            category_id: None,
            payment_method_id: None,
            search: None,
        }
    }

    fn ids(list: &TransactionList) -> Vec<i64> {
        list.items.iter().map(|t| t.id).collect()
    }

    fn page(conn: &Connection, filter: TransactionFilter, n: u32) -> TransactionList {
        list(conn, filter, n, today()).unwrap()
    }

    #[test]
    fn the_list_is_newest_first_and_paged_by_100() {
        let mut conn = db();
        let cash = cash(&conn);
        for i in 0..205 {
            spend(&mut conn, 100, cash, d(2026, 11, 1 + i % 3));
        }
        let pages: Vec<_> = (0..3).map(|n| page(&conn, this_month(), n)).collect();
        assert_eq!(
            pages
                .iter()
                .map(|p| (p.items.len(), p.has_more))
                .collect::<Vec<_>>(),
            [(100, true), (100, true), (5, false)]
        );
        let all: Vec<_> = pages.iter().flat_map(|p| &p.items).collect();
        assert!(
            all.windows(2)
                .all(|w| (w[0].date, w[0].id) > (w[1].date, w[1].id))
        );
        assert!(page(&conn, this_month(), 3).items.is_empty());
    }

    #[test]
    fn day_nets_cover_dates_split_across_pages() {
        let mut conn = db();
        let cash = cash(&conn);
        let pay = TransactionFields {
            date: d(2026, 11, 2).to_string(),
            ..income(&conn, 100_000, cash)
        };
        create(&mut conn, pay, now()).unwrap();
        for _ in 0..150 {
            spend(&mut conn, 100, cash, d(2026, 11, 2));
        }
        spend(&mut conn, 7, cash, d(2026, 11, 3));
        let net = |date, net| DayNet {
            date,
            net: Money(net),
        };
        // Page 0 holds Nov 3 and 99 of Nov 2's rows; the rest of Nov 2 is on page 1.
        assert_eq!(
            page(&conn, this_month(), 0).day_nets,
            [
                net(d(2026, 11, 3), -7),
                net(d(2026, 11, 2), 100_000 - 15_000)
            ]
        );
        assert_eq!(
            page(&conn, this_month(), 1).day_nets,
            [net(d(2026, 11, 2), 100_000 - 15_000)]
        );
    }

    #[test]
    fn summary_counts_every_page_and_leaves_out_transfers() {
        let mut conn = db();
        let cash = cash(&conn);
        let bank = bank(&mut conn, "Bank", 0);
        for _ in 0..101 {
            spend(&mut conn, 10, cash, today());
        }
        {
            let f = income(&conn, 5_000, bank);
            create(&mut conn, f, now()).unwrap();
        }
        create(&mut conn, transfer(2_000, bank, cash), now()).unwrap();
        let first = page(&conn, this_month(), 0);
        assert_eq!(
            first.summary,
            TransactionSummary {
                count: 103,
                income: Money(5_000),
                expense: Money(1_010)
            }
        );
        assert_eq!(page(&conn, this_month(), 1).summary, first.summary);
        assert_eq!(first.day_nets[0].net, Money(5_000 - 1_010));
    }

    #[test]
    fn an_accounts_list_includes_incoming_transfers() {
        let mut conn = db();
        let cash = cash(&conn);
        let bank = bank(&mut conn, "Bank", 10_000);
        let incoming = create(&mut conn, transfer(2_000, bank, cash), now()).unwrap();
        let spent = spend(&mut conn, 100, bank, today());
        spend(&mut conn, 100, cash, today());
        let of = |account| TransactionFilter {
            account_id: Some(account),
            ..this_month()
        };
        let banks = page(&conn, of(bank), 0);
        assert_eq!(ids(&banks), [spent.id, incoming.id]);
        assert_eq!(banks.summary.count, 2);
        assert_eq!(banks.summary.expense, Money(100));
        assert_eq!(page(&conn, of(cash), 0).items.len(), 2);
    }

    #[test]
    fn filters_by_kind_category_and_method() {
        let mut conn = db();
        let cash = cash(&conn);
        let method = id_of(&conn, "payment_methods", "E-wallet");
        let paid = {
            let f = TransactionFields {
                payment_method_id: Some(method),
                ..expense(&conn, 100, cash)
            };
            create(&mut conn, f, now()).unwrap()
        };
        let plain = spend(&mut conn, 200, cash, today());
        let earned = {
            let f = income(&conn, 300, cash);
            create(&mut conn, f, now()).unwrap()
        };
        let by_kind = TransactionFilter {
            kind: Some(TransactionKind::Income),
            ..this_month()
        };
        assert_eq!(ids(&page(&conn, by_kind, 0)), [earned.id]);
        let by_category = TransactionFilter {
            category_id: Some(food(&conn)),
            ..this_month()
        };
        assert_eq!(ids(&page(&conn, by_category, 0)), [plain.id, paid.id]);
        let by_method = TransactionFilter {
            payment_method_id: Some(method),
            ..this_month()
        };
        let by_method = page(&conn, by_method, 0);
        assert_eq!(ids(&by_method), [paid.id]);
        assert_eq!(by_method.summary.expense, Money(100));
    }

    #[test]
    fn search_matches_payee_or_note_ignoring_case_and_wildcards() {
        let mut conn = db();
        let cash = cash(&conn);
        let mut save = |payee: Option<&str>, note: Option<&str>| {
            let f = TransactionFields {
                payee: payee.map(Into::into),
                note: note.map(Into::into),
                ..expense(&conn, 100, cash)
            };
            create(&mut conn, f, now()).unwrap().id
        };
        let sale = save(Some("Store"), Some("50% off"));
        let other = save(Some("Store"), Some("500 off"));
        let market = save(Some("Corner Market"), None);
        let underscore = save(None, Some("a_b"));
        let found = |text: &str| {
            let filter = TransactionFilter {
                search: Some(text.into()),
                ..this_month()
            };
            ids(&page(&conn, filter, 0))
        };
        assert_eq!(found("50%"), [sale]);
        assert_eq!(found("  MARKET "), [market]);
        assert_eq!(found("_"), [underscore]);
        assert_eq!(found("store"), [other, sale]);
        assert_eq!(found("   ").len(), 4);
    }

    #[test]
    fn the_period_limits_the_list() {
        let mut conn = db();
        let cash = cash(&conn);
        spend(&mut conn, 100, cash, d(2026, 10, 31));
        let november = spend(&mut conn, 100, cash, d(2026, 11, 1));
        assert_eq!(ids(&page(&conn, this_month(), 0)), [november.id]);
        let custom = TransactionFilter {
            period: PeriodQuery::Custom {
                start: d(2026, 10, 1),
                end: d(2026, 10, 31),
            },
            ..this_month()
        };
        assert_eq!(page(&conn, custom, 0).summary.count, 1);
    }

    fn card_now(conn: &Connection, card: i64) -> accounts::CardSummary {
        accounts::get(conn, card, today()).unwrap().card.unwrap()
    }

    #[test]
    fn a_card_purchase_is_an_expense_on_the_day() {
        let mut conn = db();
        let visa = card(&mut conn, "Visa", 0);
        spend(&mut conn, 2_500, visa, today());
        let c = card_now(&conn, visa);
        assert_eq!(
            (c.amount_owed, c.cycle_spending),
            (Money(2_500), Money(2_500))
        );
        assert_eq!(page(&conn, this_month(), 0).summary.expense, Money(2_500));
    }

    #[test]
    fn a_card_payment_never_changes_the_expense_total() {
        let mut conn = db();
        let bank = bank(&mut conn, "Bank", 100_000);
        let visa = card(&mut conn, "Visa", 0);
        spend(&mut conn, 30_000, visa, today());
        let before = page(&conn, this_month(), 0);
        create(&mut conn, transfer(30_000, bank, visa), now()).unwrap();
        let after = page(&conn, this_month(), 0);
        assert_eq!(
            (after.summary.income, after.summary.expense),
            (before.summary.income, before.summary.expense)
        );
        assert_eq!(after.summary.count, before.summary.count + 1);
        assert_eq!(after.day_nets, before.day_nets);
        let of_card = TransactionFilter {
            account_id: Some(visa),
            ..this_month()
        };
        assert_eq!(page(&conn, of_card, 0).summary.expense, Money(30_000));
        // The payment lowers what's owed and the bank's balance, not the cycle's spending.
        let c = card_now(&conn, visa);
        assert_eq!((c.amount_owed, c.cycle_spending), (Money(0), Money(30_000)));
        assert_eq!(balance(&conn, bank), 70_000);
    }

    #[test]
    fn a_refund_is_income_and_lowers_the_amount_owed() {
        let mut conn = db();
        let visa = card(&mut conn, "Visa", 0);
        spend(&mut conn, 10_000, visa, today());
        let refund = TransactionFields {
            category_id: Some(id_of(&conn, "categories", "Refund")),
            ..income(&conn, 2_000, visa)
        };
        create(&mut conn, refund, now()).unwrap();
        let c = card_now(&conn, visa);
        assert_eq!(
            (c.amount_owed, c.cycle_spending),
            (Money(8_000), Money(10_000))
        );
        let summary = page(&conn, this_month(), 0).summary;
        assert_eq!(
            (summary.income, summary.expense),
            (Money(2_000), Money(10_000))
        );
    }

    /// Saves a row of `kind` with `payee` and `category`, `days` before today.
    fn paid(conn: &mut Connection, kind: TransactionKind, payee: &str, category: i64, days: u64) {
        let f = TransactionFields {
            kind,
            category_id: Some(category),
            payee: Some(payee.into()),
            date: (today() - Days::new(days)).to_string(),
            ..expense(conn, 100, cash(conn))
        };
        create(conn, f, now()).unwrap();
    }

    fn names(list: &[PayeeSuggestion]) -> Vec<&str> {
        list.iter().map(|s| s.payee.as_str()).collect()
    }

    #[test]
    fn payees_are_suggested_most_recent_first_ignoring_case() {
        let mut conn = db();
        let (food, dining) = (food(&conn), id_of(&conn, "categories", "Dining Out"));
        let expense = TransactionKind::Expense;
        paid(&mut conn, expense, "Corner Cafe", food, 9);
        paid(&mut conn, expense, "Market", food, 5);
        paid(&mut conn, expense, "corner cafe", dining, 2);
        paid(&mut conn, expense, "Cinema", food, 1);

        let found = suggest_payees(&conn, "co", CategoryKind::Expense).unwrap();
        // One suggestion per payee, spelled as it was used last, with that row's category.
        assert_eq!(
            found,
            [PayeeSuggestion {
                payee: "corner cafe".into(),
                category_id: Some(dining),
            }]
        );
        let found = suggest_payees(&conn, " C ", CategoryKind::Expense).unwrap();
        assert_eq!(names(&found), ["Cinema", "corner cafe"]);
        assert!(
            suggest_payees(&conn, "  ", CategoryKind::Expense)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn payees_come_from_the_forms_type_only() {
        let mut conn = db();
        let (food, salary) = (food(&conn), salary(&conn));
        paid(&mut conn, TransactionKind::Expense, "Acme Store", food, 3);
        paid(&mut conn, TransactionKind::Income, "Acme Corp", salary, 2);

        let expense = suggest_payees(&conn, "acme", CategoryKind::Expense).unwrap();
        assert_eq!(names(&expense), ["Acme Store"]);
        let income = suggest_payees(&conn, "acme", CategoryKind::Income).unwrap();
        assert_eq!(
            income,
            [PayeeSuggestion {
                payee: "Acme Corp".into(),
                category_id: Some(salary),
            }]
        );
    }

    #[test]
    fn a_payee_whose_last_category_is_archived_fills_no_category() {
        let mut conn = db();
        let (food, dining) = (food(&conn), id_of(&conn, "categories", "Dining Out"));
        paid(&mut conn, TransactionKind::Expense, "Corner Cafe", food, 5);
        paid(
            &mut conn,
            TransactionKind::Expense,
            "Corner Cafe",
            dining,
            1,
        );
        categories::archive(&mut conn, dining, now()).unwrap();

        let found = suggest_payees(&conn, "Corner", CategoryKind::Expense).unwrap();
        assert_eq!(
            found,
            [PayeeSuggestion {
                payee: "Corner Cafe".into(),
                category_id: None,
            }]
        );
    }

    #[test]
    fn wildcards_in_what_was_typed_are_plain_text() {
        let mut conn = db();
        let food = food(&conn);
        let expense = TransactionKind::Expense;
        paid(&mut conn, expense, "50% off shop", food, 3);
        paid(&mut conn, expense, "500 Store", food, 2);
        paid(&mut conn, expense, "A_B Mart", food, 1);
        paid(&mut conn, expense, "AxB Mart", food, 1);

        let suggest = |text| {
            names(&suggest_payees(&conn, text, CategoryKind::Expense).unwrap())
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        };
        assert_eq!(suggest("50%"), ["50% off shop"]);
        assert_eq!(suggest("A_"), ["A_B Mart"]);
        assert_eq!(suggest("\\"), Vec::<String>::new());
    }

    #[test]
    fn at_most_eight_payees_are_suggested() {
        let mut conn = db();
        let food = food(&conn);
        for i in 0..10 {
            paid(
                &mut conn,
                TransactionKind::Expense,
                &format!("Shop {i}"),
                food,
                10 - i,
            );
        }
        let found = suggest_payees(&conn, "shop", CategoryKind::Expense).unwrap();
        assert_eq!(
            names(&found),
            [
                "Shop 9", "Shop 8", "Shop 7", "Shop 6", "Shop 5", "Shop 4", "Shop 3", "Shop 2"
            ]
        );
    }
}
