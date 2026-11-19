//! The Vault level (LVL-1 to LVL-8, DATA_MODEL §5.20–5.23): gathers today's figures from the
//! other services and runs the domain's stats, ladder, focus state and streaks on them. It
//! writes nothing to `checkins`.

use chrono::{Days, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::card_cycle::card_cycle;
use crate::domain::checkin::{self, Scores, StatName};
use crate::domain::focus::{self, CapState, Ended, FocusResult, PaymentDates};
use crate::domain::forecast::latest_bill;
use crate::domain::levels::{
    self, CardCycles, Condition, ConditionCode, LevelFacts, NextMove, OverdueCard, OverduePayment,
};
use crate::domain::money::Money;
use crate::domain::period::{Period, PeriodKind, PeriodSettings};
use crate::domain::stats::{self, CardInput, Stat, StatInputs, StatReason, Stats};
use crate::error::AppError;
use crate::repo;
use crate::repo::transactions::Filter;
use crate::services::accounts::{self, Account, AccountKind};
use crate::services::forecast::{self, Forecast, ForecastGrain};
use crate::services::goals::{self, GoalProgress};
use crate::services::loans::{self, LoanList, LoanPaymentStatus};
use crate::services::period::{self, out_of_range};

/// A focus's kind (DATA_MODEL §5.22), as `checkins.focus_kind` stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FocusKind {
    CategoryCap,
    SpendingCap,
    CardLevel,
    LoanOnTime,
    Logging,
}

/// A focus as it's offered and stored (DATA_MODEL §4.9): the fields its kind doesn't use are none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FocusFields {
    pub kind: FocusKind,
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    #[ts(type = "number | null")]
    pub account_id: Option<i64>,
    #[ts(type = "number | null")]
    pub loan_payment_id: Option<i64>,
    /// Minor units for a cap or a card level, days for logging, none for `loan_on_time`.
    #[ts(type = "number | null")]
    pub target: Option<i64>,
    /// The week or month it runs for.
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// An active focus and its progress so far (CHK-8, DATA_MODEL §5.22).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActiveFocus {
    #[serde(flatten)]
    pub focus: FocusFields,
    /// In the target's unit, counted to today; none for `loan_on_time`.
    #[ts(type = "number | null")]
    pub actual: Option<i64>,
    /// `loan_on_time` only: its expense's date, none until it's paid.
    pub paid_on: Option<NaiveDate>,
    /// A cap only: `actual × 100 ÷ target`, rounded down.
    #[ts(type = "number | null")]
    pub percent: Option<i64>,
    /// A cap only.
    pub state: Option<CapState>,
}

/// A focus a check-in graded (DATA_MODEL §4.9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GradedFocus {
    #[serde(flatten)]
    pub focus: FocusFields,
    pub result: FocusResult,
    /// What was reached; none for `loan_on_time` and `void`.
    #[ts(type = "number | null")]
    pub actual: Option<i64>,
}

/// A check-in row's snapshot (DATA_MODEL §4.9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Snapshot {
    pub completed_on: NaiveDate,
    #[ts(type = "number")]
    pub level: i64,
    /// The six scores in §5.20's order, none where a stat didn't apply.
    #[ts(type = "Array<number | null>")]
    pub stats: Scores,
    pub debt_free_on: Option<NaiveDate>,
}

/// One stat today (DATA_MODEL §5.20).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StatView {
    pub stat: StatName,
    /// None ("—") when it doesn't apply.
    #[ts(type = "number | null")]
    pub score: Option<i64>,
    /// None at 100, and when it doesn't apply.
    pub reason: Option<StatReason>,
}

/// The level above today's, or level 10's own at the top, and its ring (DATA_MODEL §5.21).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NextLevel {
    #[ts(type = "number")]
    pub level: i64,
    pub conditions: Vec<Condition>,
    #[ts(type = "number")]
    pub met: i64,
    #[ts(type = "number")]
    pub total: i64,
}

/// The next best move (DATA_MODEL §5.21), by which row of its table applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum NextMoveView {
    /// Move 1, `pay_overdue_loan`.
    PayOverdueLoan {
        #[ts(type = "number")]
        payment_id: i64,
        #[ts(type = "number")]
        loan_id: i64,
        amount: Money,
    },
    /// Move 2, `pay_overdue_card`: the statement amount still to pay.
    PayOverdueCard {
        #[ts(type = "number")]
        account_id: i64,
        amount: Money,
    },
    /// Move 3: the next level's first unmet condition.
    Condition { condition: Condition },
    /// Move 4: the lowest stat's reason, none when every stat that applies is 100.
    LowestStat { reason: Option<StatReason> },
}

/// `get_vault_status` (ARCHITECTURE §4.5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VaultStatus {
    pub today: NaiveDate,
    /// The six, in §5.20's order.
    pub stats: Vec<StatView>,
    #[ts(type = "number")]
    pub level: i64,
    pub next_level: NextLevel,
    /// When the level is below the previous snapshot's: the first unmet condition of the level
    /// above today's.
    pub level_reason: Option<ConditionCode>,
    pub next_move: NextMoveView,
    /// The latest check-in's snapshot.
    pub previous: Option<Snapshot>,
    pub focuses: Vec<ActiveFocus>,
    #[ts(type = "number")]
    pub checkin_streak: i64,
    #[ts(type = "number")]
    pub focus_streak: i64,
    /// §5.11's loan-free streak in days, none when no loan has a date borrowed.
    #[ts(type = "number | null")]
    pub loan_free_days: Option<i64>,
    /// The 8 latest graded focuses, void ones left out, latest first.
    pub focus_record: Vec<GradedFocus>,
    /// The 12 latest snapshots, one per day, oldest first.
    pub history: Vec<Snapshot>,
}

/// What the stats and the ladder are worked out from for `today`.
/// The lists are the ones the figures came from, so the check-in reads each once.
pub(crate) struct Facts {
    pub inputs: StatInputs,
    pub level: LevelFacts,
    pub settings: PeriodSettings,
    /// Latest `completed_on` first.
    pub rows: Vec<repo::checkins::Row>,
    /// Every week row's `(period_start, completed_on)`, with the week being finished, if any.
    pub week_rows: Vec<(NaiveDate, NaiveDate)>,
    pub earliest_transaction: Option<NaiveDate>,
    pub accounts: Vec<Account>,
    pub loans: LoanList,
    pub goals: Vec<GoalProgress>,
    pub forecast: Forecast,
}

pub fn get(conn: &mut Connection, today: NaiveDate) -> Result<VaultStatus, AppError> {
    // Reading the goals can mark one achieved, so it's a database transaction like `list_goals`.
    let tx = conn.transaction()?;
    let facts = gather(&tx, today, None)?;
    let status = status(&tx, &facts, today)?;
    tx.commit()?;
    Ok(status)
}

/// The status from gathered facts.
pub(crate) fn status(
    conn: &Connection,
    facts: &Facts,
    today: NaiveDate,
) -> Result<VaultStatus, AppError> {
    let ladder = levels::levels(&facts.level, &facts.inputs);
    let stats = stats::stats(&facts.inputs);
    let snapshots: Vec<Snapshot> = facts.rows.iter().map(snapshot).collect::<Result<_, _>>()?;
    let previous = snapshots.first().cloned();
    let dropped = previous.as_ref().is_some_and(|p| ladder.level < p.level);
    let level_reason = dropped
        .then(|| ladder.next_conditions.iter().find(|c| !c.met))
        .flatten()
        .map(|c| c.code);
    let mut graded = Vec::new();
    for row in &facts.rows {
        if let (Some(focus), Some(result)) = (focus_of(row)?, row.focus_result.as_deref()) {
            graded.push(GradedFocus {
                focus,
                result: result_of(result, row.id)?,
                actual: row.focus_actual,
            });
        }
    }
    graded.sort_by_key(|g| std::cmp::Reverse(g.focus.end));
    Ok(VaultStatus {
        today,
        stats: views(&stats),
        level: ladder.level,
        next_level: NextLevel {
            level: (ladder.level + 1).min(10),
            met: ladder.ring_met,
            total: ladder.ring_total,
            conditions: ladder.next_conditions,
        },
        level_reason,
        next_move: move_view(ladder.next_move),
        previous,
        focuses: active(conn, &facts.rows, today)?,
        checkin_streak: checkin::checkin_streak(today, &facts.settings, &facts.week_rows),
        focus_streak: checkin::focus_streak(graded.iter().map(|g| g.result)),
        loan_free_days: facts.level.loan_free_days,
        focus_record: checkin::focus_record(graded, |g| g.result),
        history: checkin::history(snapshots, |s| s.completed_on),
    })
}

/// The active focuses with their progress, for `get_dashboard`'s strip (CHK-8).
pub fn focuses(conn: &Connection, today: NaiveDate) -> Result<Vec<ActiveFocus>, AppError> {
    active(conn, &repo::checkins::all(conn)?, today)
}

fn views(stats: &Stats) -> Vec<StatView> {
    let view = |stat, s: Option<Stat>| StatView {
        stat,
        score: s.map(|s| s.score),
        reason: s.and_then(|s| s.reason),
    };
    vec![
        view(StatName::Cashflow, stats.cashflow),
        view(StatName::Bills, stats.bills),
        view(StatName::Card, stats.card),
        view(StatName::Debt, stats.debt),
        view(StatName::Savings, stats.savings),
        view(StatName::Habit, Some(stats.habit)),
    ]
}

fn move_view(next: NextMove) -> NextMoveView {
    match next {
        NextMove::PayOverdueLoan(p) => NextMoveView::PayOverdueLoan {
            payment_id: p.payment_id,
            loan_id: p.loan_id,
            amount: p.amount,
        },
        NextMove::PayOverdueCard(c) => NextMoveView::PayOverdueCard {
            account_id: c.account_id,
            amount: c.amount,
        },
        NextMove::Condition(condition) => NextMoveView::Condition { condition },
        NextMove::LowestStat(reason) => NextMoveView::LowestStat { reason },
    }
}

/// Every focus active today (`focus_start ≤ today ≤ focus_end`), with its progress counted to
/// today. A `loan_on_time` focus whose payment was removed is left out.
fn active(
    conn: &Connection,
    rows: &[repo::checkins::Row],
    today: NaiveDate,
) -> Result<Vec<ActiveFocus>, AppError> {
    let mut list = Vec::new();
    for row in rows {
        let Some(focus) = focus_of(row)?.filter(|f| f.start <= today && today <= f.end) else {
            continue;
        };
        let ended = measure(conn, &focus, today, row.id)?;
        let (mut percent, mut state, mut paid_on) = (None, None, None);
        match ended {
            Ended::LoanOnTime { payment: None } => continue,
            Ended::LoanOnTime { payment: Some(p) } => paid_on = p.paid_on,
            Ended::CategoryCap { target, actual } | Ended::SpendingCap { target, actual } => {
                let progress = focus::cap(actual, target);
                percent = Some(progress.percent);
                state = Some(progress.state);
            }
            Ended::CardLevel { .. } | Ended::Logging { .. } => {}
        }
        list.push(ActiveFocus {
            focus,
            actual: actual(&ended),
            paid_on,
            percent,
            state,
        });
    }
    // Oldest first, so a week's focus and a month's keep their order.
    list.sort_by_key(|f| (f.focus.start, f.focus.end));
    Ok(list)
}

/// What `focus_actual` stores for a measured focus: none for `loan_on_time`.
pub(crate) fn actual(ended: &Ended) -> Option<i64> {
    match *ended {
        Ended::CategoryCap { actual, .. }
        | Ended::SpendingCap { actual, .. }
        | Ended::CardLevel { actual, .. } => Some(actual.0),
        Ended::Logging { actual, .. } => Some(actual),
        Ended::LoanOnTime { .. } => None,
    }
}

/// The focus's figure from `focus_start` to `upto` (its end when grading, today while it's
/// active), by §5.22's grading table.
pub(crate) fn measure(
    conn: &Connection,
    focus: &FocusFields,
    upto: NaiveDate,
    row_id: i64,
) -> Result<Ended, AppError> {
    let (start, end) = (focus.start.to_string(), upto.to_string());
    let missing = || AppError::Database {
        message: format!("Check-in {row_id} has an incomplete focus"),
    };
    let target = || focus.target.ok_or_else(missing);
    let everyday = |category| repo::checkins::everyday_spending(conn, &start, &end, category);
    Ok(match focus.kind {
        FocusKind::CategoryCap => Ended::CategoryCap {
            target: Money(target()?),
            actual: Money(everyday(Some(focus.category_id.ok_or_else(missing)?))?),
        },
        FocusKind::SpendingCap => Ended::SpendingCap {
            target: Money(target()?),
            actual: Money(everyday(None)?),
        },
        FocusKind::CardLevel => Ended::CardLevel {
            target: Money(target()?),
            actual: Money(card_peak(
                conn,
                focus.account_id.ok_or_else(missing)?,
                focus,
                upto,
            )?),
        },
        FocusKind::LoanOnTime => Ended::LoanOnTime {
            payment: match focus.loan_payment_id {
                None => None,
                Some(id) => match repo::loans::find_payment(conn, id)? {
                    None => None,
                    Some(p) => Some(PaymentDates {
                        due_date: stored_date(&p.due_date)?,
                        paid_on: p.paid_on.as_deref().map(stored_date).transpose()?,
                    }),
                },
            },
        },
        FocusKind::Logging => Ended::Logging {
            target: target()?,
            actual: repo::checkins::logged_days(conn, &start, &end)?,
        },
    })
}

/// The highest, over every statement cycle of the card with a day in the focus period, of its
/// expenses dated in that cycle up to `upto` (§5.22).
fn card_peak(
    conn: &Connection,
    card: i64,
    focus: &FocusFields,
    upto: NaiveDate,
) -> Result<i64, AppError> {
    let Some(row) = repo::accounts::find(conn, card, upto)? else {
        return Err(crate::services::not_found("account", card));
    };
    let (Some(statement_day), Some(due_day)) = (row.statement_day, row.due_day) else {
        return Ok(0);
    };
    let mut peak = 0;
    let mut cycle = card_cycle(statement_day, due_day, focus.start).ok_or_else(out_of_range)?;
    while cycle.start <= upto {
        let spent =
            repo::accounts::expenses_between(conn, card, cycle.start, cycle.end.min(upto), None)?;
        peak = peak.max(spent);
        let Some(next) = cycle.end.succ_opt() else {
            break;
        };
        cycle = card_cycle(statement_day, due_day, next).ok_or_else(out_of_range)?;
    }
    Ok(peak)
}

/// The row's focus, none when it set none.
pub(crate) fn focus_of(row: &repo::checkins::Row) -> Result<Option<FocusFields>, AppError> {
    let Some(kind) = row.focus_kind.as_deref() else {
        return Ok(None);
    };
    let kind = match kind {
        "category_cap" => FocusKind::CategoryCap,
        "spending_cap" => FocusKind::SpendingCap,
        "card_level" => FocusKind::CardLevel,
        "loan_on_time" => FocusKind::LoanOnTime,
        "logging" => FocusKind::Logging,
        _ => return Err(bad_row(row.id)),
    };
    let date = |text: &Option<String>| {
        text.as_deref()
            .ok_or_else(|| bad_row(row.id))
            .and_then(stored_date)
    };
    Ok(Some(FocusFields {
        kind,
        category_id: row.focus_category_id,
        account_id: row.focus_account_id,
        loan_payment_id: row.focus_loan_payment_id,
        target: row.focus_target,
        start: date(&row.focus_start)?,
        end: date(&row.focus_end)?,
    }))
}

pub(crate) fn kind_text(kind: FocusKind) -> &'static str {
    match kind {
        FocusKind::CategoryCap => "category_cap",
        FocusKind::SpendingCap => "spending_cap",
        FocusKind::CardLevel => "card_level",
        FocusKind::LoanOnTime => "loan_on_time",
        FocusKind::Logging => "logging",
    }
}

fn result_of(text: &str, row_id: i64) -> Result<FocusResult, AppError> {
    match text {
        "kept" => Ok(FocusResult::Kept),
        "missed" => Ok(FocusResult::Missed),
        "void" => Ok(FocusResult::Void),
        _ => Err(bad_row(row_id)),
    }
}

fn snapshot(row: &repo::checkins::Row) -> Result<Snapshot, AppError> {
    Ok(Snapshot {
        completed_on: stored_date(&row.completed_on)?,
        level: row.level,
        stats: row.stats,
        debt_free_on: row.debt_free_on.as_deref().map(stored_date).transpose()?,
    })
}

/// Every week row's `(period_start, completed_on)`.
pub(crate) fn week_rows(
    rows: &[repo::checkins::Row],
) -> Result<Vec<(NaiveDate, NaiveDate)>, AppError> {
    rows.iter()
        .filter(|r| r.kind == "week")
        .map(|r| Ok((stored_date(&r.period_start)?, stored_date(&r.completed_on)?)))
        .collect()
}

/// Gathers §5.20's and §5.21's figures for `today` from the other services, in the caller's
/// database transaction: reading the goals marks a goal achieved, as `list_goals` does (GOL-4).
/// `finishing` is the start of the week a check-in is finishing, which counts as finished today
/// for Habit and the check-in streak (§5.20).
pub(crate) fn gather(
    conn: &Connection,
    today: NaiveDate,
    finishing: Option<NaiveDate>,
) -> Result<Facts, AppError> {
    let settings = period::period_settings(conn)?;
    let earliest_transaction = repo::checkins::earliest_transaction(conn)?
        .as_deref()
        .map(stored_date)
        .transpose()?;
    let rows = repo::checkins::all(conn)?;
    let goals = goals::list_in(conn, today)?;
    let accounts = accounts::list(conn, today)?.accounts;
    let loans = loans::list(conn, today)?;
    let forecast = forecast::get(conn, ForecastGrain::Window, today)?;
    let before = |days: u64| {
        today
            .checked_sub_days(Days::new(days))
            .ok_or_else(out_of_range)
    };

    // §5.20's `months`: the 3 latest completed, leaving out any that ended before the first row.
    let months: Vec<Period> = match earliest_transaction {
        None => Vec::new(),
        Some(first) => {
            let current =
                Period::containing(PeriodKind::Month, today, &settings).ok_or_else(out_of_range)?;
            std::iter::successors(current.previous(&settings), |m| m.previous(&settings))
                .take(3)
                .take_while(|m| m.end() >= first)
                .collect()
        }
    };
    let borrowed_in = |m: &Period| {
        loans
            .loans
            .iter()
            .filter(|l| {
                l.is_new
                    && l.borrowed_on
                        .is_some_and(|on| m.start() <= on && on <= m.end())
            })
            .filter_map(|l| l.amount_borrowed)
            .fold(0_i64, |sum, b| sum.saturating_add(b.0))
    };
    let (mut income, mut expense, mut deductions, mut borrowed) = (0_i64, 0_i64, 0_i64, 0_i64);
    let mut earned = Vec::new();
    for m in &months {
        let (start, end) = (m.start().to_string(), m.end().to_string());
        let (_, i, e) = repo::transactions::totals(conn, &Filter::dates(&start, &end))?;
        let b = borrowed_in(m);
        income = income.saturating_add(i);
        expense = expense.saturating_add(e);
        borrowed = borrowed.saturating_add(b);
        deductions = deductions.saturating_add(repo::checkins::deductions(conn, &start, &end)?);
        earned.push(Money(i.saturating_sub(e).saturating_sub(b)));
    }

    let payments: Vec<_> = loans
        .loans
        .iter()
        .flat_map(|l| l.payments.iter().map(move |p| (l.id, p)))
        .collect();
    let overdue_payment = payments
        .iter()
        .filter(|(_, p)| p.overdue)
        .min_by_key(|(_, p)| (p.due_date, p.id))
        .map(|(loan_id, p)| OverduePayment {
            payment_id: p.id,
            loan_id: *loan_id,
            amount: p.amount,
        });
    let paid_late = |days: u64| -> Result<i64, AppError> {
        let from = before(days - 1)?;
        Ok(payments
            .iter()
            .filter(|(_, p)| p.status == LoanPaymentStatus::Paid)
            .filter(|(_, p)| {
                p.paid_on
                    .is_some_and(|on| on > p.due_date && from <= on && on <= today)
            })
            .count() as i64)
    };

    let mut cards = Vec::new();
    let mut cycles = Vec::new();
    let mut overdue_cards = Vec::new();
    for a in accounts
        .iter()
        .filter(|a| a.kind == AccountKind::CreditCard && !a.archived)
    {
        let (Some(card), Some(statement_day), Some(due_day)) =
            (&a.card, a.statement_day, a.due_day)
        else {
            continue;
        };
        cards.push(CardInput {
            account_id: a.id,
            level: card.spend_level,
            utilization_percent: card.utilization_percent,
        });
        cycles.push(CardCycles {
            account_id: a.id,
            // A card saved before levels existed has none, so it can't be over one.
            spend_mid: a.spend_mid.unwrap_or(Money(i64::MAX)),
            completed_cycles: completed_cycles(conn, a.id, statement_day, due_day, today)?,
            current_cycle: card.cycle_spending,
        });
        let row = repo::accounts::find(conn, a.id, today)?
            .ok_or_else(|| crate::services::not_found("account", a.id))?;
        let (state, due, close) = forecast::card_state(conn, &row, statement_day, due_day, today)?;
        let amount = latest_bill(&state);
        // A statement that closed before the card was added says nothing about what was on it
        // (DATA_MODEL §9 item 144).
        if amount > Money::ZERO && due < today && close >= added_on(&row.created_at)? {
            overdue_cards.push((due, a.id, amount));
        }
    }
    overdue_cards.sort();

    let completed_weeks: Vec<Period> = Period::containing(PeriodKind::Week, today, &settings)
        .and_then(|w| w.previous(&settings))
        .map(|latest| {
            std::iter::successors(Some(latest), |w| w.previous(&settings))
                .take(4)
                .collect()
        })
        .unwrap_or_default();
    let mut week_rows = week_rows(&rows)?;
    week_rows.extend(finishing.map(|start| (start, today)));
    let on_time_weeks = completed_weeks
        .iter()
        .filter(|w| {
            week_rows
                .iter()
                .any(|(start, on)| *start == w.start() && checkin::on_time(w.end(), *on))
        })
        .count() as i64;

    let inputs = StatInputs {
        months: months.len() as i64,
        income: Money(income),
        expense: Money(expense),
        deductions: Money(deductions),
        borrowed: Money(borrowed),
        cash_balance: Money(
            accounts
                .iter()
                .filter(|a| a.kind != AccountKind::CreditCard)
                .fold(0_i64, |sum, a| sum.saturating_add(a.balance.0)),
        ),
        logged_days: repo::checkins::logged_days(
            conn,
            &before(29)?.to_string(),
            &today.to_string(),
        )?,
        overdue_loan_payments: payments.iter().filter(|(_, p)| p.overdue).count() as i64,
        overdue_cards: overdue_cards.len() as i64,
        paid_late_90: paid_late(90)?,
        has_loan: !loans.loans.is_empty(),
        cards,
        loan_total_left: loans.summary.total_left,
        loan_due_this_month: loans
            .summary
            .freed_up
            .first()
            .map_or(Money::ZERO, |m| m.due),
        goals_behind: goals
            .iter()
            .filter(|g| !g.goal.archived && g.behind.is_some())
            .count() as i64,
        on_time_weeks,
    };
    let level = LevelFacts {
        has_payday_rule: repo::recurring::all(conn)?.iter().any(|r| r.is_payday),
        has_budget: repo::budgets::any(conn)?,
        paid_late_30: paid_late(30)?,
        earned,
        has_short_day: forecast.short.is_some(),
        cards: cycles,
        loan_free_days: loans.summary.streak_days,
        overdue_payment,
        overdue_card: overdue_cards
            .first()
            .map(|&(_, account_id, amount)| OverdueCard { account_id, amount }),
    };
    Ok(Facts {
        inputs,
        level,
        settings,
        rows,
        week_rows,
        earliest_transaction,
        accounts,
        loans,
        goals,
        forecast,
    })
}

/// The card's expense totals in its 3 latest completed statement cycles, latest first.
fn completed_cycles(
    conn: &Connection,
    id: i64,
    statement_day: u8,
    due_day: u8,
    today: NaiveDate,
) -> Result<Vec<Money>, AppError> {
    let mut cycle = card_cycle(statement_day, due_day, today).ok_or_else(out_of_range)?;
    let mut totals = Vec::new();
    for _ in 0..3 {
        cycle = cycle
            .start
            .pred_opt()
            .and_then(|last| card_cycle(statement_day, due_day, last))
            .ok_or_else(out_of_range)?;
        totals.push(Money(repo::accounts::expenses_between(
            conn,
            id,
            cycle.start,
            cycle.end,
            None,
        )?));
    }
    Ok(totals)
}

/// The local date of a `created_at` stamp.
fn added_on(created_at: &str) -> Result<NaiveDate, AppError> {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map(|t| t.with_timezone(&chrono::Local).date_naive())
        .map_err(|_| AppError::Database {
            message: "A stored timestamp is invalid".into(),
        })
}

pub(crate) fn stored_date(text: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| AppError::Database {
        message: "A stored date is invalid".into(),
    })
}

fn bad_row(id: i64) -> AppError {
    AppError::Database {
        message: format!("Check-in {id} has an invalid focus"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::recurrence::Frequency;
    use crate::services::accounts::{AccountFields, NewAccount};
    use crate::services::budgets::{self, BudgetFields, BudgetPeriod};
    use crate::services::categories::{self, CategoryFields, CategoryKind, NewCategory};
    use crate::services::dashboard;
    use crate::services::loans::{LoanFields, LoanPaymentFields, NewLoan};
    use crate::services::payslips::{self, Deduction, PayslipFields};
    use crate::services::period::PeriodQuery;
    use crate::services::recurring::{self, RecurringMode, RecurringRuleFields};
    use crate::services::settings;
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::test_support::{db, now, payday};
    use chrono::{Local, TimeZone};
    use rusqlite::params;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn id_of(conn: &Connection, table: &str, name: &str) -> i64 {
        conn.query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1"),
            [name],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn expense(amount: i64, date: &str, account: i64, category: i64) -> TransactionFields {
        TransactionFields {
            kind: TransactionKind::Expense,
            amount: Money(amount),
            date: date.into(),
            account_id: account,
            to_account_id: None,
            category_id: Some(category),
            payee: None,
            payment_method_id: None,
            note: None,
        }
    }

    fn spend(conn: &mut Connection, amount: i64, date: &str, account: i64, category: i64) -> i64 {
        transactions::create(conn, expense(amount, date, account, category), now())
            .unwrap()
            .saved
            .id
    }

    fn card(conn: &mut Connection) -> i64 {
        accounts::create(
            conn,
            NewAccount {
                kind: AccountKind::CreditCard,
                fields: AccountFields {
                    name: "Card".into(),
                    opening_balance: Money(0),
                    credit_limit: Some(Money(10_000_000)),
                    statement_day: Some(20),
                    due_day: Some(5),
                    spend_soft: None,
                    spend_mid: None,
                    spend_hard: None,
                },
            },
            now(),
        )
        .unwrap()
        .id
    }

    fn loan(conn: &mut Connection, account: i64, payments: &[(&str, i64)]) -> loans::Loan {
        let fields = LoanFields {
            name: "Phone".into(),
            account_id: account,
            category_id: id_of(conn, "categories", "Loan"),
            amount_borrowed: None,
            borrowed_on: Some("2026-10-01".into()),
            payments: payments
                .iter()
                .map(|&(date, amount)| LoanPaymentFields {
                    id: None,
                    due_date: date.into(),
                    amount: Money(amount),
                })
                .collect(),
        };
        loans::create(
            conn,
            NewLoan {
                is_new: true,
                fields,
            },
            now(),
        )
        .unwrap()
        .saved
    }

    /// A check-in row written by hand: `focus` is (kind, category, account, payment, target,
    /// start, end), and `graded` its result and actual.
    #[allow(clippy::type_complexity)]
    fn row(
        conn: &Connection,
        (kind, start, end, completed_on, level): (&str, &str, &str, &str, i64),
        focus: Option<(
            &str,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            &str,
            &str,
        )>,
        graded: Option<(&str, Option<i64>)>,
    ) {
        let (fk, cat, acc, pay, target, fs, fe) = match focus {
            Some((k, c, a, p, t, s, e)) => (Some(k), c, a, p, t, Some(s), Some(e)),
            None => (None, None, None, None, None, None, None),
        };
        let (result, actual) = graded.map_or((None, None), |(r, a)| (Some(r), a));
        conn.execute(
            "INSERT INTO checkins (kind, period_start, period_end, completed_on, level,
                 stat_cashflow, stat_bills, stat_card, stat_debt, stat_savings, stat_habit,
                 focus_kind, focus_category_id, focus_account_id, focus_loan_payment_id,
                 focus_target, focus_start, focus_end, focus_result, focus_actual,
                 created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 50, NULL, NULL, 100, 40, 30,
                     ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'x', 'x')",
            params![
                kind,
                start,
                end,
                completed_on,
                level,
                fk,
                cat,
                acc,
                pay,
                target,
                fs,
                fe,
                result,
                actual
            ],
        )
        .unwrap();
    }

    #[test]
    fn codes_go_on_the_wire_as_data_model_names_them() {
        let name = |v: serde_json::Value| v.as_str().unwrap().to_string();
        let codes = [
            ConditionCode::Logged20,
            ConditionCode::Earned3Months,
            ConditionCode::LoanFree90,
            ConditionCode::Stats80,
            ConditionCode::LoanFree365,
            ConditionCode::NothingOverdue,
        ]
        .map(|c| name(serde_json::to_value(c).unwrap()));
        assert_eq!(
            codes,
            [
                "logged_20",
                "earned_3_months",
                "loan_free_90",
                "stats_80",
                "loan_free_365",
                "nothing_overdue"
            ]
        );
        assert_eq!(
            name(serde_json::to_value(StatReason::FewDaysLogged).unwrap()),
            "few_days_logged"
        );
        let next = serde_json::to_value(NextMoveView::PayOverdueCard {
            account_id: 3,
            amount: Money(100),
        })
        .unwrap();
        assert_eq!(next["kind"], "pay_overdue_card");
        assert_eq!(next["accountId"], 3);
    }

    #[test]
    fn a_new_database_starts_at_level_0_with_nothing_to_show() {
        let mut conn = db();
        let s = get(&mut conn, d(2026, 11, 20)).unwrap();
        assert_eq!(s.level, 0);
        assert_eq!(s.next_level.level, 1);
        assert_eq!((s.next_level.met, s.next_level.total), (0, 1));
        assert_eq!(s.stats.len(), 6);
        assert_eq!(s.stats[0].score, None);
        assert_eq!(s.stats[5].stat, StatName::Habit);
        assert_eq!(s.stats[5].score, Some(0));
        assert_eq!(
            s.next_move,
            NextMoveView::Condition {
                condition: Condition {
                    code: ConditionCode::Logged20,
                    met: false,
                    gap: Some(20),
                    account_id: None,
                }
            }
        );
        assert_eq!(s.previous, None);
        assert!(s.focuses.is_empty() && s.history.is_empty() && s.focus_record.is_empty());
        assert_eq!(
            (s.checkin_streak, s.focus_streak, s.loan_free_days),
            (0, 0, None)
        );
    }

    #[test]
    fn a_loan_payment_going_overdue_drops_the_level_below_3() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let food = id_of(&conn, "categories", "Food & Groceries");
        // 20 days logged, Oct 20 to Nov 8: all within 30 days of Nov 13.
        for day in d(2026, 10, 20).iter_days().take(20) {
            spend(&mut conn, 100, &day.to_string(), cash, food);
        }
        payday(&mut conn, cash, 1_000_000);
        let budget = BudgetFields {
            category_id: None,
            period: BudgetPeriod::Monthly,
            amount: Money(500_000),
        };
        budgets::create(&mut conn, budget, now()).unwrap();
        let phone = loan(&mut conn, cash, &[("2026-11-12", 50_000)]);

        let on_the_day = get(&mut conn, d(2026, 11, 12)).unwrap();
        assert_eq!(on_the_day.level, 3);
        let late = get(&mut conn, d(2026, 11, 13)).unwrap();
        assert_eq!(late.level, 2);
        assert_eq!(late.stats[1].reason, Some(StatReason::LoanOverdue));
        assert_eq!(late.stats[1].score, Some(75));
        assert_eq!(
            late.next_move,
            NextMoveView::PayOverdueLoan {
                payment_id: phone.payments[0].id,
                loan_id: phone.id,
                amount: Money(50_000),
            }
        );
        assert_eq!(late.loan_free_days, Some(43));
    }

    #[test]
    fn a_card_statement_unpaid_past_its_due_date_is_the_next_move() {
        let mut conn = db();
        // Added Nov 3 (`now`).
        let card = card(&mut conn);
        let food = id_of(&conn, "categories", "Food & Groceries");
        // Oct 10 is in the cycle that closed Oct 20, before the card was added, and was due Nov 5:
        // nothing says what was really on that statement, so it isn't overdue.
        spend(&mut conn, 3_000, "2026-10-10", card, food);
        let s = get(&mut conn, d(2026, 11, 10)).unwrap();
        assert_eq!(s.stats[1].score, Some(100));
        // Nov 10 is in the cycle that closes Nov 20, due Dec 5.
        spend(&mut conn, 10_000, "2026-11-10", card, food);
        let s = get(&mut conn, d(2026, 12, 6)).unwrap();
        assert_eq!(s.stats[1].reason, Some(StatReason::CardOverdue));
        assert_eq!(
            s.next_move,
            NextMoveView::PayOverdueCard {
                account_id: card,
                amount: Money(13_000),
            }
        );
        // Not overdue on its due date.
        let s = get(&mut conn, d(2026, 12, 5)).unwrap();
        assert_eq!(s.stats[1].score, Some(100));
    }

    #[test]
    fn a_cap_counts_everyday_spending_only() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let food = id_of(&conn, "categories", "Food & Groceries");
        // Everyday spending.
        spend(&mut conn, 1_000, "2026-11-03", cash, food);
        // A loan payment's expense.
        let phone = loan(&mut conn, cash, &[("2026-11-05", 50_000)]);
        loans::mark_paid(
            &mut conn,
            phone.payments[0].id,
            expense(50_000, "2026-11-03", cash, food),
            now(),
        )
        .unwrap();
        // A recurring date, posted on creation.
        let rule = RecurringRuleFields {
            transaction: expense(2_000, "2026-11-02", cash, food),
            frequency: Frequency::Monthly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode: RecurringMode::Auto,
            is_payday: false,
        };
        recurring::create(&mut conn, rule, now()).unwrap();
        // A payslip deduction.
        let rule = payday(&mut conn, cash, 1_000_000);
        let slip = PayslipFields {
            date: "2026-11-15".into(),
            account_id: cash,
            gross: Money(1_000_000),
            deductions: vec![Deduction {
                category_id: food,
                amount: Money(3_000),
            }],
            note: None,
        };
        let nov_15 = Local.with_ymd_and_hms(2026, 11, 15, 9, 0, 0).unwrap();
        payslips::record(&mut conn, rule, "2026-11-15", slip, nov_15).unwrap();
        let month = ("month", "2026-10-01", "2026-10-31", "2026-11-01", 0);
        row(
            &conn,
            month,
            Some((
                "spending_cap",
                None,
                None,
                None,
                Some(1_000),
                "2026-11-01",
                "2026-11-30",
            )),
            None,
        );
        let week = ("week", "2026-10-26", "2026-11-01", "2026-11-02", 0);
        row(
            &conn,
            week,
            Some((
                "category_cap",
                Some(food),
                None,
                None,
                Some(4_000),
                "2026-11-02",
                "2026-11-08",
            )),
            None,
        );

        let today = d(2026, 11, 20);
        let list = focuses(&conn, today).unwrap();
        // Only the month's focus is still active.
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].focus.kind, FocusKind::SpendingCap);
        assert_eq!(list[0].actual, Some(1_000));
        assert_eq!(
            (list[0].percent, list[0].state),
            (Some(100), Some(CapState::Near))
        );
        let week = FocusFields {
            kind: FocusKind::CategoryCap,
            category_id: Some(food),
            account_id: None,
            loan_payment_id: None,
            target: Some(4_000),
            start: d(2026, 11, 2),
            end: d(2026, 11, 8),
        };
        assert_eq!(
            measure(&conn, &week, week.end, 0).unwrap(),
            Ended::CategoryCap {
                target: Money(4_000),
                actual: Money(1_000)
            }
        );
        let dash = dashboard::get(&conn, &PeriodQuery::Month { anchor: None }, today).unwrap();
        assert_eq!(dash.focuses, list);
    }

    #[test]
    fn a_card_level_focus_takes_the_highest_cycle_including_its_days_before_the_focus() {
        let mut conn = db();
        let card = card(&mut conn);
        let food = id_of(&conn, "categories", "Food & Groceries");
        spend(&mut conn, 2_000, "2026-10-25", card, food);
        spend(&mut conn, 5_000, "2026-11-10", card, food);
        spend(&mut conn, 4_000, "2026-11-25", card, food);
        spend(&mut conn, 9_000, "2026-12-01", card, food);
        let month = ("month", "2026-10-01", "2026-10-31", "2026-11-01", 0);
        row(
            &conn,
            month,
            Some((
                "card_level",
                None,
                Some(card),
                None,
                Some(8_000),
                "2026-11-01",
                "2026-11-30",
            )),
            None,
        );
        let list = focuses(&conn, d(2026, 11, 30)).unwrap();
        assert_eq!(list[0].actual, Some(7_000));
        assert_eq!((list[0].percent, list[0].state), (None, None));
    }

    #[test]
    fn a_loan_on_time_focus_shows_when_it_was_paid_and_goes_once_its_payment_is_removed() {
        let mut conn = db();
        let cash = id_of(&conn, "accounts", "Cash");
        let food = id_of(&conn, "categories", "Food & Groceries");
        let phone = loan(&mut conn, cash, &[("2026-11-10", 50_000)]);
        let pay = phone.payments[0].id;
        let week = ("week", "2026-10-26", "2026-11-01", "2026-11-02", 0);
        row(
            &conn,
            week,
            Some((
                "loan_on_time",
                None,
                None,
                Some(pay),
                None,
                "2026-11-02",
                "2026-11-08",
            )),
            None,
        );
        let today = d(2026, 11, 4);
        assert_eq!(focuses(&conn, today).unwrap()[0].paid_on, None);
        loans::mark_paid(
            &mut conn,
            pay,
            expense(50_000, "2026-11-03", cash, food),
            now(),
        )
        .unwrap();
        let list = focuses(&conn, today).unwrap();
        assert_eq!(
            (list[0].paid_on, list[0].actual),
            (Some(d(2026, 11, 3)), None)
        );
        conn.execute("UPDATE checkins SET focus_loan_payment_id = NULL", [])
            .unwrap();
        assert!(focuses(&conn, today).unwrap().is_empty());
    }

    #[test]
    fn the_status_reads_snapshots_streaks_and_graded_focuses() {
        let mut conn = db();
        let logging = |s, e| Some(("logging", None, None, None, Some(5), s, e));
        row(
            &conn,
            ("week", "2026-11-02", "2026-11-08", "2026-11-09", 5),
            logging("2026-11-09", "2026-11-15"),
            Some(("kept", Some(6))),
        );
        row(
            &conn,
            ("week", "2026-11-09", "2026-11-15", "2026-11-16", 4),
            logging("2026-11-16", "2026-11-22"),
            Some(("missed", Some(2))),
        );
        let s = get(&mut conn, d(2026, 11, 25)).unwrap();
        let previous = s.previous.unwrap();
        assert_eq!(
            (previous.completed_on, previous.level),
            (d(2026, 11, 16), 4)
        );
        assert_eq!(
            previous.stats,
            [Some(50), None, None, Some(100), Some(40), Some(30)]
        );
        // The level fell from 4 to 0, so the reason is level 1's unmet condition.
        assert_eq!(s.level_reason, Some(ConditionCode::Logged20));
        // Nov 16–22 has no row yet and can still be on time, so it's passed over.
        assert_eq!(s.checkin_streak, 2);
        assert_eq!(s.focus_streak, 0);
        let record: Vec<_> = s
            .focus_record
            .iter()
            .map(|f| (f.result, f.actual))
            .collect();
        assert_eq!(
            record,
            [(FocusResult::Missed, Some(2)), (FocusResult::Kept, Some(6))]
        );
        let days: Vec<_> = s.history.iter().map(|h| h.completed_on).collect();
        assert_eq!(days, [d(2026, 11, 9), d(2026, 11, 16)]);
    }

    #[test]
    fn a_focus_puts_its_category_and_card_in_use_and_follows_a_merge() {
        let mut conn = db();
        let food = id_of(&conn, "categories", "Food & Groceries");
        let treats = categories::create(
            &mut conn,
            NewCategory {
                kind: CategoryKind::Expense,
                fields: CategoryFields {
                    name: "Treats".into(),
                    color: "cat-3".into(),
                    icon: "coffee".into(),
                },
            },
            now(),
        )
        .unwrap()
        .id;
        assert!(!settings::get(&conn).unwrap().decimals_locked);
        let week = ("week", "2026-10-26", "2026-11-01", "2026-11-02", 0);
        row(
            &conn,
            week,
            Some((
                "category_cap",
                Some(treats),
                None,
                None,
                Some(4_000),
                "2026-11-02",
                "2026-11-08",
            )),
            None,
        );
        // An amount in a focus locks the decimals, like any other stored amount.
        assert!(settings::get(&conn).unwrap().decimals_locked);
        let card = card(&mut conn);
        let month = ("month", "2026-10-01", "2026-10-31", "2026-11-02", 0);
        row(
            &conn,
            month,
            Some((
                "card_level",
                None,
                Some(card),
                None,
                Some(8_000),
                "2026-11-01",
                "2026-11-30",
            )),
            None,
        );
        let listed = categories::list(&conn).unwrap();
        assert!(listed.categories.iter().any(|c| c.id == treats && c.in_use));
        assert!(matches!(
            categories::delete(&mut conn, treats),
            Err(AppError::Conflict { .. })
        ));
        assert!(matches!(
            accounts::delete(&mut conn, card, now()),
            Err(AppError::Conflict { .. })
        ));
        assert!(accounts::get(&conn, card, d(2026, 11, 3)).unwrap().in_use);
        categories::merge(&mut conn, treats, food, now()).unwrap();
        let moved: i64 = conn
            .query_row(
                "SELECT focus_category_id FROM checkins WHERE focus_kind = 'category_cap'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(moved, food);
    }
}
