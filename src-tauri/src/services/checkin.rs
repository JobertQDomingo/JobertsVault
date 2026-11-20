//! The required check-in (CHK-1 to CHK-7, DATA_MODEL §5.19–5.23): what's due, every step's
//! figures, the focus offers, and Finish, which writes the rows and the grades in one database
//! transaction. The figures come from the services the other screens use.

use std::collections::HashMap;

use chrono::{DateTime, Local, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;
use ts_rs::TS;

use crate::domain::account::{SpendLevel, SpendLevels};
use crate::domain::checkin::{self, DuePeriods, PeriodTotals, PersonalBest, Win, WinFacts};
use crate::domain::focus::{
    self, CardOffer, CategorySpending, Ended, Focus, FocusResult, OfferFacts,
};
use crate::domain::levels;
use crate::domain::money::Money;
use crate::domain::period::{ComparisonMode, Period, PeriodKind};
use crate::domain::stats;
use crate::error::AppError;
use crate::repo;
use crate::repo::transactions::Filter;
use crate::services::accounts::{Account, AccountKind};
use crate::services::dashboard::{self, LoanDue, LoansLeft};
use crate::services::forecast::{Forecast, Span};
use crate::services::goals::GoalProgress;
use crate::services::loans::LoanPaymentStatus;
use crate::services::period::{PeriodQuery, out_of_range};
use crate::services::reports::{self, CategoryLine, Comparison};
use crate::services::vault::{self, Facts, FocusFields, FocusKind, GradedFocus, VaultStatus};
use crate::services::{invalid, stamp};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CheckinKind {
    Week,
    Month,
    Year,
}

/// `get_checkin` (ARCHITECTURE §4.5): the whole check-in, one object per step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Checkin {
    /// The kinds Finish settles, week, month, year in that order.
    pub kinds: Vec<CheckinKind>,
    /// The period shown: the month when it's due, else the week.
    pub period: Span,
    /// The period it's compared with, in full.
    pub compare: Span,
    pub today: NaiveDate,
    pub start: StartStep,
    pub money: MoneyStep,
    /// Every active card, as `list_accounts` returns it, as it is today.
    pub cards: Vec<Account>,
    pub loans: LoansStep,
    /// None when the step is skipped; else every active goal.
    pub goals: Option<Vec<GoalProgress>>,
    /// None unless the month is due (CHK-4).
    pub balances: Option<Vec<Balance>>,
    /// None unless the year is due (CHK-5).
    pub year: Option<YearStep>,
    pub status: CheckinStatus,
    /// 1 to 3 focuses, the one to pick first first.
    pub offers: Vec<Offer>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct StartStep {
    pub win: Option<Win>,
    pub personal_best: Option<PersonalBest>,
    /// Of the focuses this check-in grades, the one that ended latest, as Finish will grade it.
    pub last_focus: Option<GradedFocus>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MoneyStep {
    /// As `get_comparison` returns it: the shown period in full against the one before.
    pub comparison: Comparison,
    /// The 3 largest expense categories, each with its change.
    pub top_categories: Vec<CategoryLine>,
    /// Each weekly budget for a week, or monthly one for a month, over the period shown.
    pub budgets: Vec<BudgetKept>,
}

/// A budget measured over a finished period (DATA_MODEL §5.19).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BudgetKept {
    #[ts(type = "number")]
    pub budget_id: i64,
    /// None for all spending.
    #[ts(type = "number | null")]
    pub category_id: Option<i64>,
    pub spent: Money,
    pub amount: Money,
    /// `spent ≤ amount`.
    pub kept: bool,
    /// How far `spent` went past `amount`, 0 when kept.
    pub over: Money,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LoansStep {
    /// The loan payments whose expense is dated in the period shown, by that date.
    pub paid: Vec<PaidPayment>,
    /// Σ `paid`'s amounts.
    pub paid_total: Money,
    /// As the dashboard has them.
    pub loans_due: Vec<LoanDue>,
    pub loans_left: Option<LoansLeft>,
    /// The debt-free date moved since the latest earlier check-in.
    pub debt_free_moved: Option<DateMove>,
    /// The forecast summary today, with its next 3 rows.
    pub forecast: Forecast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaidPayment {
    #[ts(type = "number")]
    pub loan_id: i64,
    #[ts(type = "number")]
    pub payment_id: i64,
    /// Its expense's amount.
    pub amount: Money,
    pub paid_on: NaiveDate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DateMove {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// One account in the Balance check (CHK-4): its balance today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Balance {
    #[ts(type = "number")]
    pub account_id: i64,
    pub balance: Money,
    /// A card only: what the user checks against the statement.
    pub amount_owed: Option<Money>,
}

/// Your year (CHK-5).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct YearStep {
    /// The year in full against the year before.
    pub comparison: Comparison,
    pub top_categories: Vec<CategoryLine>,
    /// Each yearly budget over the year.
    pub budgets: Vec<BudgetKept>,
    /// The loans whose last payment was paid in the year.
    #[ts(type = "Array<number>")]
    pub loans_paid_off: Vec<i64>,
    /// The level on the first and the last check-in finished in the year.
    #[ts(type = "number | null")]
    pub first_level: Option<i64>,
    #[ts(type = "number | null")]
    pub last_level: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LevelChange {
    Up,
    Down,
    Same,
}

/// The Stats and level step: `get_vault_status` with this check-in counted as finished today.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CheckinStatus {
    #[serde(flatten)]
    pub status: VaultStatus,
    /// Against the latest earlier check-in; `same` when there's none (§5.21).
    pub level_change: LevelChange,
}

/// A focus offered, with what its sentence needs (§5.22).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Offer {
    #[serde(flatten)]
    pub focus: FocusFields,
    /// `loan_on_time`: the payment's loan and due date.
    #[ts(type = "number | null")]
    pub loan_id: Option<i64>,
    pub due_date: Option<NaiveDate>,
    /// `card_level`: the level its target is.
    pub level: Option<SpendLevel>,
    /// A cap: the earlier period's spending (`category_cap`) or `perDay` (`spending_cap`).
    pub basis: Option<Money>,
    /// `spending_cap`: the everyday spending already recorded for the week, which its target
    /// includes.
    pub spent: Option<Money>,
}

/// A row `complete_checkin` wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Written {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: CheckinKind,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub completed_on: NaiveDate,
}

/// What's due today and everything worked out from it, the same for `get` and `complete`.
struct Due {
    due: DuePeriods,
    shown: Period,
    facts: Facts,
    /// Every ended focus not graded yet: its row, its fields, what it reached, its grade.
    ended: Vec<(i64, FocusFields, Ended, FocusResult)>,
    offers: Vec<Offer>,
}

/// None when nothing is due.
pub fn get(conn: &mut Connection, today: NaiveDate) -> Result<Option<Checkin>, AppError> {
    // Reading the goals can mark one achieved (GOL-4), so it's a database transaction.
    let tx = conn.transaction()?;
    let checkin = match due(&tx, today)? {
        None => None,
        Some(d) => Some(view(&tx, d, today)?),
    };
    tx.commit()?;
    Ok(checkin)
}

/// Finish: works out again what's due, checks `focus` is one of the offers it would make now,
/// grades every ended focus, and writes one row per due kind, all in one database transaction.
pub fn complete(
    conn: &mut Connection,
    focus: FocusFields,
    now: DateTime<Local>,
) -> Result<Vec<Written>, AppError> {
    let today = now.date_naive();
    let tx = conn.transaction()?;
    let d = due(&tx, today)?.ok_or_else(|| AppError::Conflict {
        message: "This check-in is already done".into(),
    })?;
    if !d.offers.iter().any(|o| o.focus == focus) {
        return Err(invalid("focus", "Pick one of the offered focuses"));
    }
    let stamp = stamp(now);
    for (id, _, ended, result) in &d.ended {
        let actual = (*result != FocusResult::Void)
            .then(|| vault::actual(ended))
            .flatten();
        repo::checkins::set_grade(&tx, *id, result_text(*result), actual, &stamp)?;
    }
    let level = levels::levels(&d.facts.level, &d.facts.inputs).level;
    let scores = checkin::scores(&stats::stats(&d.facts.inputs));
    let debt_free_on = d.facts.loans.summary.debt_free_on.map(|on| on.to_string());
    let (focus_start, focus_end) = (focus.start.to_string(), focus.end.to_string());
    let today_text = today.to_string();
    let mut written = Vec::new();
    for (kind, period) in [
        (CheckinKind::Week, d.due.week),
        (CheckinKind::Month, d.due.month),
        (CheckinKind::Year, d.due.year),
    ] {
        let Some(period) = period else { continue };
        let (start, end) = (period.start().to_string(), period.end().to_string());
        // A merged check-in keeps its focus on the shown period's row; a year never has one.
        let has_focus = period == d.shown;
        let row = repo::checkins::NewRow {
            kind: kind_text(kind),
            period_start: &start,
            period_end: &end,
            completed_on: &today_text,
            level,
            stats: scores,
            debt_free_on: debt_free_on.as_deref(),
            focus: has_focus.then(|| repo::checkins::NewFocus {
                kind: vault::kind_text(focus.kind),
                category_id: focus.category_id,
                account_id: focus.account_id,
                loan_payment_id: focus.loan_payment_id,
                target: focus.target,
                start: &focus_start,
                end: &focus_end,
            }),
        };
        written.push(Written {
            id: repo::checkins::insert(&tx, &row, &stamp)?,
            kind,
            period_start: period.start(),
            period_end: period.end(),
            completed_on: today,
        });
    }
    tx.commit()?;
    Ok(written)
}

fn due(conn: &Connection, today: NaiveDate) -> Result<Option<Due>, AppError> {
    let settings = crate::services::period::period_settings(conn)?;
    let rows = repo::checkins::all(conn)?;
    let mut done = Vec::new();
    for row in &rows {
        let kind = match row.kind.as_str() {
            "week" => PeriodKind::Week,
            "month" => PeriodKind::Month,
            "year" => PeriodKind::Year,
            _ => {
                return Err(AppError::Database {
                    message: format!("Check-in {} has an unknown kind", row.id),
                });
            }
        };
        done.push((kind, vault::stored_date(&row.period_start)?));
    }
    let earliest = repo::checkins::earliest_transaction(conn)?
        .as_deref()
        .map(vault::stored_date)
        .transpose()?;
    let due = checkin::due_periods(today, &settings, &done, earliest);
    let Some(shown) = due.shown() else {
        return Ok(None);
    };
    let facts = vault::gather(conn, today, due.week.map(|w| w.start()))?;
    let mut ended = Vec::new();
    for row in facts.rows.iter().filter(|r| r.focus_result.is_none()) {
        let Some(focus) = vault::focus_of(row)?.filter(|f| f.end < today) else {
            continue;
        };
        let measured = vault::measure(conn, &focus, focus.end, row.id)?;
        ended.push((row.id, focus, measured, focus::grade(&measured)));
    }
    let offers = offers(conn, &facts, &due, shown, today)?;
    Ok(Some(Due {
        due,
        shown,
        facts,
        ended,
        offers,
    }))
}

/// §5.22's offers for the week or month that contains today.
fn offers(
    conn: &Connection,
    facts: &Facts,
    due: &DuePeriods,
    shown: Period,
    today: NaiveDate,
) -> Result<Vec<Offer>, AppError> {
    let settings = &facts.settings;
    let for_week = due.month.is_none();
    let kind = if for_week {
        PeriodKind::Week
    } else {
        PeriodKind::Month
    };
    let period = Period::containing(kind, today, settings).ok_or_else(out_of_range)?;
    let in_period = |date: NaiveDate| period.start() <= date && date <= period.end();
    let payment = facts
        .loans
        .loans
        .iter()
        .filter(|l| !l.archived)
        .flat_map(|l| l.payments.iter().map(move |p| (l.id, p)))
        // One already past its due date can't be paid on time any more.
        .filter(|(_, p)| {
            p.status == LoanPaymentStatus::Unpaid && in_period(p.due_date) && p.due_date >= today
        })
        .min_by_key(|(_, p)| (p.due_date, p.id));
    let mut cards: Vec<(CardOffer, SpendLevels)> = Vec::new();
    for a in active_cards(&facts.accounts) {
        let (Some(card), Some(soft), Some(mid), Some(hard)) =
            (a.card.as_ref(), a.spend_soft, a.spend_mid, a.spend_hard)
        else {
            continue;
        };
        let levels = SpendLevels { soft, mid, hard };
        let peak = vault::card_peak(conn, a.id, period.start(), period.end())?;
        cards.push((
            CardOffer {
                account_id: a.id,
                levels,
                spent: card.cycle_spending,
                peak: Money(peak),
                utilization_percent: card.utilization_percent,
            },
            levels,
        ));
    }
    let before = shown.previous(settings).ok_or_else(out_of_range)?;
    let by_category = |p: Period| -> Result<HashMap<i64, i64>, AppError> {
        let (start, end) = (p.start().to_string(), p.end().to_string());
        Ok(repo::checkins::everyday_by_category(conn, &start, &end)?
            .into_iter()
            .collect())
    };
    let (reviewed, earlier) = (by_category(shown)?, by_category(before)?);
    // What's already recorded for the period, future-dated rows included: grading counts them.
    let (start, end) = (period.start().to_string(), period.end().to_string());
    let so_far: HashMap<i64, i64> = repo::checkins::everyday_by_category(conn, &start, &end)?
        .into_iter()
        .collect();
    let categories = repo::categories::all(conn)?
        .into_iter()
        .filter(|c| c.kind == "expense" && !c.archived)
        .map(|c| CategorySpending {
            category_id: c.id,
            reviewed: Money(reviewed.get(&c.id).copied().unwrap_or(0)),
            before: Money(earlier.get(&c.id).copied().unwrap_or(0)),
            so_far: Money(so_far.get(&c.id).copied().unwrap_or(0)),
        })
        .collect();
    let per_day = facts.forecast.per_day.unwrap_or(Money::ZERO);
    let spent = Money(repo::checkins::everyday_spending(conn, &start, &end, None)?);
    let offer_facts = OfferFacts {
        for_week,
        loan_payment_id: payment.map(|(_, p)| p.id),
        cards: cards.iter().map(|(c, _)| *c).collect(),
        has_payday_rule: facts.level.has_payday_rule,
        per_day,
        categories,
        spent_so_far: spent,
        days_left: (period.end() - today).num_days() + 1,
        logged_before_today: repo::checkins::logged_days(
            conn,
            &start,
            &(today - chrono::Days::new(1)).to_string(),
        )?,
    };
    let fields = |kind, target: Option<i64>| FocusFields {
        kind,
        category_id: None,
        account_id: None,
        loan_payment_id: None,
        target,
        start: period.start(),
        end: period.end(),
    };
    let offer = |focus| Offer {
        focus,
        loan_id: None,
        due_date: None,
        level: None,
        basis: None,
        spent: None,
    };
    Ok(focus::offers(&offer_facts, &stats::stats(&facts.inputs))
        .into_iter()
        .map(|f| match f {
            Focus::LoanOnTime { payment_id } => Offer {
                loan_id: payment.map(|(loan, _)| loan),
                due_date: payment.map(|(_, p)| p.due_date),
                ..offer(FocusFields {
                    loan_payment_id: Some(payment_id),
                    ..fields(FocusKind::LoanOnTime, None)
                })
            },
            Focus::CardLevel { account_id, target } => {
                let levels = cards.iter().find(|(c, _)| c.account_id == account_id);
                Offer {
                    level: levels.map(|(_, l)| match target {
                        t if t == l.soft => SpendLevel::Soft,
                        t if t == l.mid => SpendLevel::Mid,
                        _ => SpendLevel::Hard,
                    }),
                    ..offer(FocusFields {
                        account_id: Some(account_id),
                        ..fields(FocusKind::CardLevel, Some(target.0))
                    })
                }
            }
            Focus::SpendingCap { target } => Offer {
                basis: Some(per_day),
                spent: Some(spent),
                ..offer(fields(FocusKind::SpendingCap, Some(target.0)))
            },
            Focus::CategoryCap {
                category_id,
                target,
            } => Offer {
                basis: Some(target),
                ..offer(FocusFields {
                    category_id: Some(category_id),
                    ..fields(FocusKind::CategoryCap, Some(target.0))
                })
            },
            Focus::Logging { target } => offer(fields(FocusKind::Logging, Some(target))),
        })
        .collect())
}

fn view(conn: &Connection, d: Due, today: NaiveDate) -> Result<Checkin, AppError> {
    let Due {
        due,
        shown,
        facts,
        ended,
        offers,
    } = d;
    let settings = &facts.settings;
    let query = |p: Period| match p.kind() {
        PeriodKind::Week => PeriodQuery::Week {
            anchor: Some(p.start()),
        },
        PeriodKind::Year => PeriodQuery::Year {
            anchor: Some(p.start()),
        },
        _ => PeriodQuery::Month {
            anchor: Some(p.start()),
        },
    };
    let compared =
        |p: Period| reports::comparison(conn, &query(p), Some(ComparisonMode::Full), None, today);
    let comparison = compared(shown)?;
    let previous = facts.rows.first();

    // Start.
    let last_focus =
        ended
            .iter()
            .max_by_key(|(id, f, ..)| (f.end, *id))
            .map(|(_, focus, measured, result)| GradedFocus {
                focus: *focus,
                result: *result,
                actual: (*result != FocusResult::Void)
                    .then(|| vault::actual(measured))
                    .flatten(),
            });
    let in_shown = |date: NaiveDate| shown.start() <= date && date <= shown.end();
    let payments: Vec<_> = facts
        .loans
        .loans
        .iter()
        .flat_map(|l| l.payments.iter().map(move |p| (l.id, p)))
        .collect();
    let stats = stats::stats(&facts.inputs);
    let win = checkin::win(&WinFacts {
        focus_kept: last_focus
            .as_ref()
            .is_some_and(|f| f.result == FocusResult::Kept),
        scores: checkin::scores(&stats),
        previous: previous.map(|r| r.stats),
        categories: comparison
            .expense_categories
            .iter()
            .map(|c| checkin::CategoryTotals {
                category_id: c.category_id,
                amount: c.line.current,
                before: c.line.compare,
            })
            .collect(),
        expense: comparison.expense.current,
        expense_before: comparison.expense.compare,
        loan_payments: payments
            .iter()
            .filter(|(_, p)| p.status != LoanPaymentStatus::PaidBefore && in_shown(p.due_date))
            .map(|(_, p)| focus::PaymentDates {
                due_date: p.due_date,
                paid_on: p.paid_on,
            })
            .collect(),
    });
    let personal_best = match facts.earliest_transaction {
        None => None,
        Some(first) => {
            let mut earlier = Vec::new();
            for p in checkin::best_periods(shown, settings, first) {
                let (start, end) = (p.start().to_string(), p.end().to_string());
                let (_, income, expense) =
                    repo::transactions::totals(conn, &Filter::dates(&start, &end))?;
                earlier.push(PeriodTotals {
                    expense: Money(expense),
                    net: Money(income.saturating_sub(expense)),
                });
            }
            let shown_totals = PeriodTotals {
                expense: comparison.expense.current,
                net: comparison.net.current,
            };
            checkin::personal_best(shown_totals, &earlier)
        }
    };

    // Loans and bills.
    let (loans_due, loans_left) = dashboard::loan_reminders(&facts.accounts, &facts.loans, today);
    let mut paid: Vec<PaidPayment> = payments
        .iter()
        .filter(|(_, p)| p.status == LoanPaymentStatus::Paid)
        .filter_map(|(loan_id, p)| {
            let paid_on = p.paid_on.filter(|on| in_shown(*on))?;
            Some(PaidPayment {
                loan_id: *loan_id,
                payment_id: p.id,
                amount: p.paid_amount.unwrap_or(p.amount),
                paid_on,
            })
        })
        .collect();
    paid.sort_by_key(|p| (p.paid_on, p.payment_id));
    // A date stored on or before the day that check-in was finished was "today", with everything
    // overdue then, so it says nothing about a move (§5.19).
    let moved_from = match previous {
        Some(r) => {
            let finished = vault::stored_date(&r.completed_on)?;
            r.debt_free_on
                .as_deref()
                .map(vault::stored_date)
                .transpose()?
                .filter(|from| *from > finished)
        }
        None => None,
    };
    let debt_free_moved = match (moved_from, facts.loans.summary.debt_free_on, &loans_left) {
        (Some(from), Some(to), Some(left)) if from != to && !left.overdue => {
            Some(DateMove { from, to })
        }
        _ => None,
    };
    let mut forecast = facts.forecast.clone();
    forecast.rows.truncate(3);

    // Goals: with a month when there's an active one; with only a week, when one is Behind or
    // was moved in it.
    let active_goals: Vec<GoalProgress> = facts
        .goals
        .iter()
        .filter(|g| !g.goal.archived)
        .cloned()
        .collect();
    let goals_show = (due.month.is_some() && !active_goals.is_empty())
        || active_goals
            .iter()
            .any(|g| g.behind.is_some() || g.goal.moved_on.is_some_and(|on| on >= shown.start()));

    let status = vault::status(conn, &facts, today)?;
    let level_change = match previous.map(|r| r.level) {
        Some(before) if status.level > before => LevelChange::Up,
        Some(before) if status.level < before => LevelChange::Down,
        _ => LevelChange::Same,
    };
    let year = match due.year {
        None => None,
        Some(y) => {
            let comparison = compared(y)?;
            let in_year = |date: NaiveDate| y.start() <= date && date <= y.end();
            let levels_in_year: Vec<i64> = facts
                .rows
                .iter()
                .filter(|r| vault::stored_date(&r.completed_on).is_ok_and(in_year))
                .map(|r| r.level)
                .collect();
            Some(YearStep {
                top_categories: top(&comparison),
                budgets: kept(conn, "yearly", y)?,
                loans_paid_off: facts
                    .loans
                    .loans
                    .iter()
                    .filter(|l| l.paid_off)
                    .filter(|l| {
                        l.payments
                            .iter()
                            .filter_map(|p| p.paid_on)
                            .max()
                            .is_some_and(in_year)
                    })
                    .map(|l| l.id)
                    .collect(),
                // The rows are latest first.
                first_level: levels_in_year.last().copied(),
                last_level: levels_in_year.first().copied(),
                comparison,
            })
        }
    };
    Ok(Checkin {
        kinds: [
            due.week.map(|_| CheckinKind::Week),
            due.month.map(|_| CheckinKind::Month),
            due.year.map(|_| CheckinKind::Year),
        ]
        .into_iter()
        .flatten()
        .collect(),
        period: span(shown),
        compare: Span {
            start: comparison.compare_start,
            end: comparison.compare_end,
        },
        today,
        start: StartStep {
            win,
            personal_best,
            last_focus,
        },
        money: MoneyStep {
            top_categories: top(&comparison),
            budgets: kept(
                conn,
                if due.month.is_some() {
                    "monthly"
                } else {
                    "weekly"
                },
                shown,
            )?,
            comparison,
        },
        cards: active_cards(&facts.accounts).cloned().collect(),
        loans: LoansStep {
            paid_total: Money(
                paid.iter()
                    .fold(0_i64, |sum, p| sum.saturating_add(p.amount.0)),
            ),
            paid,
            loans_due,
            loans_left,
            debt_free_moved,
            forecast,
        },
        goals: goals_show.then_some(active_goals),
        balances: due.month.map(|_| {
            facts
                .accounts
                .iter()
                .filter(|a| !a.archived || a.balance != Money::ZERO)
                .map(|a| Balance {
                    account_id: a.id,
                    balance: a.balance,
                    amount_owed: a.card.as_ref().map(|c| c.amount_owed),
                })
                .collect()
        }),
        year,
        status: CheckinStatus {
            status,
            level_change,
        },
        offers,
    })
}

fn active_cards(accounts: &[Account]) -> impl Iterator<Item = &Account> {
    accounts
        .iter()
        .filter(|a| a.kind == AccountKind::CreditCard && !a.archived)
}

/// The 3 largest expense categories with an amount this period.
fn top(comparison: &Comparison) -> Vec<CategoryLine> {
    comparison
        .expense_categories
        .iter()
        .filter(|c| c.line.current > Money::ZERO)
        .take(3)
        .cloned()
        .collect()
}

/// Each budget of `period` (`weekly`, `monthly` or `yearly`) measured over `over` (§5.6).
fn kept(conn: &Connection, period: &str, over: Period) -> Result<Vec<BudgetKept>, AppError> {
    let (start, end) = (over.start().to_string(), over.end().to_string());
    let mut list = Vec::new();
    for b in repo::budgets::all(conn)?
        .into_iter()
        .filter(|b| b.period == period)
    {
        let filter = Filter {
            kind: Some("expense"),
            category_id: b.category_id,
            ..Filter::dates(&start, &end)
        };
        let spent = Money(repo::transactions::totals(conn, &filter)?.2);
        list.push(BudgetKept {
            budget_id: b.id,
            category_id: b.category_id,
            spent,
            amount: Money(b.amount),
            kept: spent.0 <= b.amount,
            over: Money(spent.0.saturating_sub(b.amount).max(0)),
        });
    }
    Ok(list)
}

fn span(p: Period) -> Span {
    Span {
        start: p.start(),
        end: p.end(),
    }
}

fn kind_text(kind: CheckinKind) -> &'static str {
    match kind {
        CheckinKind::Week => "week",
        CheckinKind::Month => "month",
        CheckinKind::Year => "year",
    }
}

fn result_text(result: FocusResult) -> &'static str {
    match result {
        FocusResult::Kept => "kept",
        FocusResult::Missed => "missed",
        FocusResult::Void => "void",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::accounts;
    use crate::services::transactions::{self, TransactionFields, TransactionKind};
    use crate::test_support::{db, now};
    use chrono::TimeZone;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn at(day: NaiveDate) -> DateTime<Local> {
        Local
            .from_local_datetime(&day.and_hms_opt(9, 0, 0).unwrap())
            .unwrap()
    }

    const CASH: i64 = 1;

    fn food(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT id FROM categories WHERE name = 'Food & Groceries'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn fields(amount: i64, date: &str, category: i64) -> TransactionFields {
        TransactionFields {
            kind: TransactionKind::Expense,
            amount: Money(amount),
            date: date.into(),
            account_id: CASH,
            to_account_id: None,
            category_id: Some(category),
            payee: None,
            payment_method_id: None,
            note: None,
        }
    }

    fn spend(conn: &mut Connection, amount: i64, date: &str) -> i64 {
        let category = food(conn);
        transactions::create(conn, fields(amount, date, category), now())
            .unwrap()
            .saved
            .id
    }

    fn row_count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM checkins {sql}"), [], |r| {
            r.get(0)
        })
        .unwrap()
    }

    fn logging(c: &Checkin) -> FocusFields {
        c.offers
            .iter()
            .find(|o| o.focus.kind == FocusKind::Logging)
            .unwrap()
            .focus
    }

    #[test]
    fn a_month_lists_every_active_goal_in_its_goals_step() {
        let mut conn = db();
        spend(&mut conn, 1_000, "2026-10-20");
        let goal = repo::goals::Values {
            name: "Trip",
            target_amount: 100_000,
            target_date: None,
            account_id: CASH,
        };
        let id = repo::goals::insert(&conn, &goal, "2026-11-03T00:00:00Z").unwrap();
        let c = get(&mut conn, d(2026, 11, 2)).unwrap().unwrap();
        assert!(c.kinds.contains(&CheckinKind::Month));
        let listed: Vec<i64> = c.goals.unwrap().iter().map(|g| g.goal.id).collect();
        assert_eq!(listed, [id]);
    }

    #[test]
    fn a_new_database_has_no_check_in() {
        let mut conn = db();
        assert_eq!(get(&mut conn, d(2026, 11, 16)).unwrap(), None);
        let any = logging_focus(d(2026, 11, 16), d(2026, 11, 22));
        assert!(matches!(
            complete(&mut conn, any, at(d(2026, 11, 16))),
            Err(AppError::Conflict { .. })
        ));
    }

    fn logging_focus(start: NaiveDate, end: NaiveDate) -> FocusFields {
        FocusFields {
            kind: FocusKind::Logging,
            category_id: None,
            account_id: None,
            loan_payment_id: None,
            target: Some(5),
            start,
            end,
        }
    }

    #[test]
    fn the_week_of_nov_9_to_15_is_due_on_mon_nov_16_and_not_once_its_row_exists() {
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        assert_eq!(get(&mut conn, d(2026, 11, 15)).unwrap(), None);
        let c = get(&mut conn, d(2026, 11, 16)).unwrap().unwrap();
        assert_eq!(c.kinds, [CheckinKind::Week]);
        assert_eq!(
            (c.period.start, c.period.end),
            (d(2026, 11, 9), d(2026, 11, 15))
        );
        assert_eq!(
            (c.compare.start, c.compare.end),
            (d(2026, 11, 2), d(2026, 11, 8))
        );
        // Only the week: no balance check, no year, and the goals step is skipped.
        assert_eq!(
            (c.balances.clone(), c.year.clone(), c.goals.clone()),
            (None, None, None)
        );
        assert_eq!(c.status.level_change, LevelChange::Same);
        let focus = logging(&c);
        assert_eq!(
            (focus.start, focus.end, focus.target),
            (d(2026, 11, 16), d(2026, 11, 22), Some(5))
        );
        let written = complete(&mut conn, focus, at(d(2026, 11, 16))).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(get(&mut conn, d(2026, 11, 16)).unwrap(), None);
        // A second Finish is refused.
        assert!(matches!(
            complete(&mut conn, focus, at(d(2026, 11, 16))),
            Err(AppError::Conflict { .. })
        ));
        assert_eq!(row_count(&conn, ""), 1);
    }

    #[test]
    fn a_month_and_a_week_due_together_make_one_check_in_with_two_rows_and_one_focus() {
        let mut conn = db();
        spend(&mut conn, 1_000, "2026-10-20");
        spend(&mut conn, 500, "2026-11-10");
        let today = d(2026, 11, 16);
        let before = accounts::list(&conn, today).unwrap();
        let c = get(&mut conn, today).unwrap().unwrap();
        assert_eq!(c.kinds, [CheckinKind::Week, CheckinKind::Month]);
        assert_eq!(
            (c.period.start, c.period.end),
            (d(2026, 10, 1), d(2026, 10, 31))
        );
        assert_eq!(c.money.comparison.expense.current, Money(1_000));
        assert_eq!(c.money.top_categories.len(), 1);
        let balances = c.balances.clone().unwrap();
        assert!(
            balances
                .iter()
                .any(|b| b.account_id == CASH && b.balance == Money(-1_500))
        );
        // No active goal: no Goals step, even with the month.
        assert_eq!(c.goals, None);
        let focus = logging(&c);
        // A month's focus runs for the current month. It asks for the days still reachable:
        // Nov 10, logged, and the 15 days from Nov 16 to 30.
        assert_eq!(
            (focus.start, focus.end, focus.target),
            (d(2026, 11, 1), d(2026, 11, 30), Some(16))
        );
        let written = complete(&mut conn, focus, at(today)).unwrap();
        let kinds: Vec<_> = written.iter().map(|w| w.kind).collect();
        assert_eq!(kinds, [CheckinKind::Week, CheckinKind::Month]);
        assert_eq!(row_count(&conn, "WHERE completed_on = '2026-11-16'"), 2);
        assert_eq!(
            row_count(&conn, "WHERE focus_kind IS NOT NULL AND kind = 'month'"),
            1
        );
        assert_eq!(row_count(&conn, "WHERE focus_kind IS NOT NULL"), 1);
        // Finishing changes no balance.
        assert_eq!(accounts::list(&conn, today).unwrap(), before);
    }

    #[test]
    fn a_focus_that_isnt_an_offer_is_refused_and_nothing_is_written() {
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        let today = d(2026, 11, 16);
        let c = get(&mut conn, today).unwrap().unwrap();
        let mut focus = logging(&c);
        focus.target = Some(4);
        let refused = complete(&mut conn, focus, at(today));
        assert!(
            matches!(refused, Err(AppError::Validation { ref field, .. }) if field == "focus"),
            "{refused:?}"
        );
        // A focus for another week isn't one of the offers either.
        let next_week = logging_focus(d(2026, 11, 23), d(2026, 11, 29));
        assert!(complete(&mut conn, next_week, at(today)).is_err());
        assert_eq!(row_count(&conn, ""), 0);
    }

    #[test]
    fn a_failed_finish_writes_nothing() {
        let mut conn = db();
        spend(&mut conn, 1_000, "2026-10-20");
        spend(&mut conn, 500, "2026-11-10");
        let today = d(2026, 11, 16);
        let focus = logging(&get(&mut conn, today).unwrap().unwrap());
        // The month's row, written after the week's, fails.
        conn.execute_batch(
            "CREATE TRIGGER no_months BEFORE INSERT ON checkins WHEN NEW.kind = 'month'
             BEGIN SELECT RAISE(ABORT, 'refused'); END;",
        )
        .unwrap();
        assert!(complete(&mut conn, focus, at(today)).is_err());
        assert_eq!(row_count(&conn, ""), 0);
    }

    #[test]
    fn a_focus_is_graded_once_and_keeps_its_grade_after_an_old_transaction_is_edited() {
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        let first = d(2026, 11, 16);
        let focus = logging(&get(&mut conn, first).unwrap().unwrap());
        complete(&mut conn, focus, at(first)).unwrap();
        let mut last = 0;
        for day in 17..=21 {
            last = spend(&mut conn, 100, &format!("2026-11-{day}"));
        }
        let second = d(2026, 11, 23);
        let c = get(&mut conn, second).unwrap().unwrap();
        let shown = c.start.last_focus.clone().unwrap();
        assert_eq!((shown.result, shown.actual), (FocusResult::Kept, Some(5)));
        assert_eq!(c.start.win, Some(Win::FocusKept));
        // The Start step works the grade out without storing it.
        assert_eq!(row_count(&conn, "WHERE focus_result IS NOT NULL"), 0);
        complete(&mut conn, logging(&c), at(second)).unwrap();
        assert_eq!(
            row_count(&conn, "WHERE focus_result = 'kept' AND focus_actual = 5"),
            1
        );
        // Moving a day out of the week changes the live figures, not the grade.
        let category = food(&conn);
        transactions::update(&mut conn, last, fields(100, "2026-11-24", category), now()).unwrap();
        let status = vault::get(&mut conn, d(2026, 11, 25)).unwrap();
        let record: Vec<_> = status
            .focus_record
            .iter()
            .map(|f| (f.result, f.actual))
            .collect();
        assert_eq!(record, [(FocusResult::Kept, Some(5))]);
        assert_eq!(status.checkin_streak, 2);
        // The next week's focus is active, and none of the first is shown again.
        assert_eq!(status.focuses.len(), 1);
        assert_eq!(status.focuses[0].focus.start, second);
    }

    #[test]
    fn a_weekly_budget_is_kept_or_says_how_far_over_it_went() {
        use crate::services::budgets::{self, BudgetFields, BudgetPeriod};
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        spend(&mut conn, 300, "2026-11-12");
        let food = food(&conn);
        for (category_id, amount) in [(Some(food), 700), (None, 1_000)] {
            let fields = BudgetFields {
                category_id,
                period: BudgetPeriod::Weekly,
                amount: Money(amount),
            };
            budgets::create(&mut conn, fields, now()).unwrap();
        }
        let c = get(&mut conn, d(2026, 11, 16)).unwrap().unwrap();
        let shown: Vec<_> = c
            .money
            .budgets
            .iter()
            .map(|b| (b.category_id, b.kept, b.over))
            .collect();
        assert_eq!(
            shown,
            [(None, true, Money::ZERO), (Some(food), false, Money(100))]
        );
    }

    #[test]
    fn a_loan_payment_due_this_week_is_offered_graded_and_listed_as_paid() {
        use crate::services::loans::{self, LoanFields, LoanPaymentFields, NewLoan};
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        let loan_category: i64 = conn
            .query_row("SELECT id FROM categories WHERE name = 'Loan'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let loan = loans::create(
            &mut conn,
            NewLoan {
                is_new: false,
                fields: LoanFields {
                    name: "Phone".into(),
                    account_id: CASH,
                    category_id: loan_category,
                    amount_borrowed: None,
                    borrowed_on: None,
                    payments: vec![LoanPaymentFields {
                        id: None,
                        due_date: "2026-11-20".into(),
                        amount: Money(5_000),
                    }],
                },
            },
            now(),
        )
        .unwrap()
        .saved;
        let payment = loan.payments[0].id;
        let first = d(2026, 11, 16);
        let c = get(&mut conn, first).unwrap().unwrap();
        let offer = c
            .offers
            .iter()
            .find(|o| o.focus.kind == FocusKind::LoanOnTime)
            .unwrap();
        assert_eq!(offer.focus.loan_payment_id, Some(payment));
        assert_eq!(
            (offer.loan_id, offer.due_date),
            (Some(loan.id), Some(d(2026, 11, 20)))
        );
        complete(&mut conn, offer.focus, at(first)).unwrap();
        let paid = fields(5_000, "2026-11-19", loan_category);
        loans::mark_paid(&mut conn, payment, paid, now()).unwrap();
        let c = get(&mut conn, d(2026, 11, 23)).unwrap().unwrap();
        let shown = c.start.last_focus.clone().unwrap();
        assert_eq!((shown.result, shown.actual), (FocusResult::Kept, None));
        let listed: Vec<_> = c
            .loans
            .paid
            .iter()
            .map(|p| (p.payment_id, p.paid_on))
            .collect();
        assert_eq!(listed, [(payment, d(2026, 11, 19))]);
        assert_eq!(c.loans.paid_total, Money(5_000));
        assert_eq!(c.loans.loans_left, None);
    }

    #[test]
    fn a_late_check_in_offers_only_what_can_still_be_kept() {
        let mut conn = db();
        let salary: i64 = conn
            .query_row("SELECT id FROM categories WHERE name = 'Salary'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let income = TransactionFields {
            kind: TransactionKind::Income,
            ..fields(1_000_000, "2026-11-09", salary)
        };
        transactions::create(&mut conn, income, now()).unwrap();
        // Food rose from 200 to 1,100 in the week reviewed, and 300 since Monday is already
        // over the 200 a category cap would ask for.
        spend(&mut conn, 200, "2026-11-04");
        spend(&mut conn, 500, "2026-11-10");
        spend(&mut conn, 600, "2026-11-12");
        spend(&mut conn, 300, "2026-11-17");
        // Future-dated: grading counts it on Saturday, so the spending cap does too.
        spend(&mut conn, 50, "2026-11-21");
        let loan_category: i64 = conn
            .query_row("SELECT id FROM categories WHERE name = 'Loan'", [], |r| {
                r.get(0)
            })
            .unwrap();
        // Due Tuesday and still unpaid: it can't be paid on time any more.
        add(&mut conn, loan_category, "Phone", "2026-11-17");
        // Due today: still on time.
        add(&mut conn, loan_category, "Car", "2026-11-20");
        let rule = crate::services::recurring::RecurringRuleFields {
            transaction: TransactionFields {
                kind: TransactionKind::Income,
                ..fields(420_000, "2026-12-01", salary)
            },
            frequency: crate::domain::recurrence::Frequency::Monthly,
            interval: 1,
            end_date: None,
            max_occurrences: None,
            mode: crate::services::recurring::RecurringMode::Confirm,
            is_payday: true,
        };
        crate::services::recurring::create(&mut conn, rule, now()).unwrap();

        let friday = d(2026, 11, 20);
        let c = get(&mut conn, friday).unwrap().unwrap();
        let kinds: Vec<_> = c.offers.iter().map(|o| o.focus.kind).collect();
        assert_eq!(kinds.len(), 3, "{kinds:?}");
        for kind in [
            FocusKind::LoanOnTime,
            FocusKind::SpendingCap,
            FocusKind::Logging,
        ] {
            assert!(kinds.contains(&kind), "{kinds:?}");
        }
        let loan = c
            .offers
            .iter()
            .find(|o| o.focus.kind == FocusKind::LoanOnTime)
            .unwrap();
        assert_eq!(loan.due_date, Some(friday));
        let cap = c
            .offers
            .iter()
            .find(|o| o.focus.kind == FocusKind::SpendingCap)
            .unwrap();
        // What's recorded for the week, then per day for Friday to Sunday.
        let per_day = cap.basis.unwrap();
        assert!(per_day > Money::ZERO);
        assert_eq!(cap.spent, Some(Money(350)));
        assert_eq!(cap.focus.target, Some(350 + per_day.0 * 3));
        // Nov 17 is logged, and 3 days are left: 4 at most.
        assert_eq!(logging(&c).target, Some(4));
    }

    /// A loan the user already has, with one payment of 5,000.00 due on `due`.
    fn add(conn: &mut Connection, category: i64, name: &str, due: &str) {
        use crate::services::loans::{self, LoanFields, LoanPaymentFields, NewLoan};
        let fields = LoanFields {
            name: name.into(),
            account_id: CASH,
            category_id: category,
            amount_borrowed: None,
            borrowed_on: None,
            payments: vec![LoanPaymentFields {
                id: None,
                due_date: due.into(),
                amount: Money(5_000),
            }],
        };
        let new = NewLoan {
            is_new: false,
            fields,
        };
        loans::create(conn, new, now()).unwrap();
    }

    #[test]
    fn a_debt_free_date_stored_while_everything_was_overdue_never_counts_as_moved() {
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        let category: i64 = conn
            .query_row("SELECT id FROM categories WHERE name = 'Loan'", [], |r| {
                r.get(0)
            })
            .unwrap();
        // Overdue on Nov 16, so the debt-free date stored that day is that day.
        add(&mut conn, category, "Phone", "2026-11-12");
        let first = d(2026, 11, 16);
        let c = get(&mut conn, first).unwrap().unwrap();
        complete(&mut conn, logging(&c), at(first)).unwrap();
        add(&mut conn, category, "Laptop", "2027-06-01");
        let c = get(&mut conn, d(2026, 11, 23)).unwrap().unwrap();
        assert_eq!(
            c.loans.loans_left.map(|l| l.debt_free_on),
            Some(d(2027, 6, 1))
        );
        assert_eq!(c.loans.debt_free_moved, None);
    }

    #[test]
    fn a_january_check_in_settles_the_year_too() {
        let mut conn = db();
        spend(&mut conn, 1_000, "2026-11-10");
        spend(&mut conn, 2_000, "2026-12-20");
        let today = d(2027, 1, 4);
        let c = get(&mut conn, today).unwrap().unwrap();
        assert_eq!(
            c.kinds,
            [CheckinKind::Week, CheckinKind::Month, CheckinKind::Year]
        );
        let year = c.year.clone().unwrap();
        assert_eq!(year.comparison.expense.current, Money(3_000));
        assert_eq!((year.first_level, year.last_level), (None, None));
        let written = complete(&mut conn, logging(&c), at(today)).unwrap();
        assert_eq!(written.len(), 3);
        assert_eq!(
            row_count(&conn, "WHERE kind = 'year' AND focus_kind IS NULL"),
            1
        );
    }

    #[test]
    fn finishing_counts_this_week_as_on_time_in_its_own_status() {
        let mut conn = db();
        spend(&mut conn, 500, "2026-11-10");
        let c = get(&mut conn, d(2026, 11, 16)).unwrap().unwrap();
        assert_eq!(c.status.status.checkin_streak, 1);
        // 1 of the 4 latest weeks on time: 30 × 1 ÷ 4 = 7 points, plus 1 day logged (2 points).
        assert_eq!(c.status.status.stats[5].score, Some(9));
        complete(&mut conn, logging(&c), at(d(2026, 11, 16))).unwrap();
        let habit: Option<i64> = conn
            .query_row("SELECT stat_habit FROM checkins", [], |r| r.get(0))
            .unwrap();
        assert_eq!(habit, Some(9));
    }
}
