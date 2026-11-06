use chrono::{NaiveDate, Weekday};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::period::{Period, PeriodKind, PeriodSettings};
use crate::error::AppError;
use crate::services::invalid;
use crate::services::settings::{self, WeekStart};

/// Which period a screen shows (ARCHITECTURE §4.5). No anchor means the one containing today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum PeriodQuery {
    Day { anchor: Option<NaiveDate> },
    Week { anchor: Option<NaiveDate> },
    Month { anchor: Option<NaiveDate> },
    Year { anchor: Option<NaiveDate> },
    Custom { start: NaiveDate, end: NaiveDate },
}

/// A period's dates and the queries its ◀, ▶ and Today buttons use (DATA_MODEL §5.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeriodView {
    pub start: NaiveDate,
    pub end: NaiveDate,
    /// `None` only past the supported date range.
    pub previous: Option<PeriodQuery>,
    /// `None` when the next period would start after today.
    pub next: Option<PeriodQuery>,
    /// The period of the same kind that contains today, as ▶ would reach it. A custom range
    /// moves by its length.
    pub today_query: PeriodQuery,
    /// Whether today falls inside this period.
    pub is_current: bool,
    pub today: NaiveDate,
}

/// The dates `query` covers, using the week and month start settings (DATA_MODEL §5.3).
pub fn resolve(
    conn: &Connection,
    query: &PeriodQuery,
    today: NaiveDate,
) -> Result<Period, AppError> {
    resolve_with(query, today, &period_settings(conn)?)
}

pub fn get(
    conn: &Connection,
    query: &PeriodQuery,
    today: NaiveDate,
) -> Result<PeriodView, AppError> {
    let settings = period_settings(conn)?;
    let period = resolve_with(query, today, &settings)?;
    let today_query = query_for(
        &period
            .in_sequence(today, &settings)
            .ok_or_else(out_of_range)?,
    );
    Ok(PeriodView {
        start: period.start(),
        end: period.end(),
        previous: period.previous(&settings).as_ref().map(query_for),
        next: period.next(&settings, today).as_ref().map(query_for),
        today_query,
        is_current: (period.start()..=period.end()).contains(&today),
        today,
    })
}

/// Where weeks and months start (SET-2).
pub fn period_settings(conn: &Connection) -> Result<PeriodSettings, AppError> {
    let s = settings::stored(conn)?;
    Ok(PeriodSettings {
        week_start: match s.week_start {
            WeekStart::Monday => Weekday::Mon,
            WeekStart::Sunday => Weekday::Sun,
        },
        month_start_day: s.month_start_day.into(),
    })
}

fn resolve_with(
    query: &PeriodQuery,
    today: NaiveDate,
    settings: &PeriodSettings,
) -> Result<Period, AppError> {
    let (kind, anchor) = match *query {
        PeriodQuery::Day { anchor } => (PeriodKind::Day, anchor),
        PeriodQuery::Week { anchor } => (PeriodKind::Week, anchor),
        PeriodQuery::Month { anchor } => (PeriodKind::Month, anchor),
        PeriodQuery::Year { anchor } => (PeriodKind::Year, anchor),
        PeriodQuery::Custom { start, end } => {
            return Period::custom(start, end)
                .ok_or_else(|| invalid("period", "The end date can't be before the start"));
        }
    };
    Period::containing(kind, anchor.unwrap_or(today), settings).ok_or_else(out_of_range)
}

fn out_of_range() -> AppError {
    invalid("period", "That date is outside the supported range")
}

/// The query that shows exactly `period`: an anchored kind is anchored on its first day.
fn query_for(period: &Period) -> PeriodQuery {
    let anchor = Some(period.start());
    match period.kind() {
        PeriodKind::Day => PeriodQuery::Day { anchor },
        PeriodKind::Week => PeriodQuery::Week { anchor },
        PeriodKind::Month => PeriodQuery::Month { anchor },
        PeriodKind::Year => PeriodQuery::Year { anchor },
        PeriodKind::Custom => PeriodQuery::Custom {
            start: period.start(),
            end: period.end(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::db;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("a real test date")
    }

    fn range(conn: &Connection, query: PeriodQuery) -> (NaiveDate, NaiveDate) {
        let period = resolve(conn, &query, d(2026, 11, 3)).unwrap();
        (period.start(), period.end())
    }

    #[test]
    fn month_without_an_anchor_is_todays_month() {
        let conn = db();
        assert_eq!(
            range(&conn, PeriodQuery::Month { anchor: None }),
            (d(2026, 11, 1), d(2026, 11, 30))
        );
    }

    #[test]
    fn week_and_month_follow_the_settings() {
        let mut conn = db();
        let mut s = settings::get(&conn).unwrap().settings;
        s.week_start = WeekStart::Sunday;
        s.month_start_day = 15;
        settings::update(&mut conn, s).unwrap();
        let anchor = Some(d(2026, 11, 3));
        assert_eq!(
            range(&conn, PeriodQuery::Week { anchor }),
            (d(2026, 11, 1), d(2026, 11, 7))
        );
        assert_eq!(
            range(&conn, PeriodQuery::Month { anchor }),
            (d(2026, 10, 15), d(2026, 11, 14))
        );
    }

    fn custom(start: NaiveDate, end: NaiveDate) -> PeriodQuery {
        PeriodQuery::Custom { start, end }
    }

    #[test]
    fn the_current_month_is_current_and_has_no_next() {
        let conn = db();
        let view = get(&conn, &PeriodQuery::Month { anchor: None }, d(2026, 11, 3)).unwrap();
        assert_eq!((view.start, view.end), (d(2026, 11, 1), d(2026, 11, 30)));
        assert!(view.is_current);
        assert_eq!(view.next, None);
        assert_eq!(
            view.previous,
            Some(PeriodQuery::Month {
                anchor: Some(d(2026, 10, 1))
            })
        );
        assert_eq!(view.today, d(2026, 11, 3));
    }

    #[test]
    fn an_older_month_steps_by_anchor_and_today_matches_where_next_stops() {
        let conn = db();
        let query = PeriodQuery::Month {
            anchor: Some(d(2026, 9, 14)),
        };
        let view = get(&conn, &query, d(2026, 11, 3)).unwrap();
        assert_eq!((view.start, view.end), (d(2026, 9, 1), d(2026, 9, 30)));
        assert!(!view.is_current);
        assert_eq!(
            view.next,
            Some(PeriodQuery::Month {
                anchor: Some(d(2026, 10, 1))
            })
        );
        // The same query ▶ reaches, so one period has one cache key.
        let october = get(&conn, &view.next.unwrap(), d(2026, 11, 3)).unwrap();
        assert_eq!(view.today_query, october.next.unwrap());
        assert_eq!(
            view.today_query,
            PeriodQuery::Month {
                anchor: Some(d(2026, 11, 1))
            }
        );
    }

    #[test]
    fn a_custom_range_steps_by_its_length_and_today_finds_the_range_with_today() {
        let conn = db();
        let view = get(
            &conn,
            &custom(d(2026, 10, 1), d(2026, 10, 10)),
            d(2026, 11, 3),
        )
        .unwrap();
        assert!(!view.is_current);
        assert_eq!(view.previous, Some(custom(d(2026, 9, 21), d(2026, 9, 30))));
        assert_eq!(view.next, Some(custom(d(2026, 10, 11), d(2026, 10, 20))));
        assert_eq!(view.today_query, custom(d(2026, 10, 31), d(2026, 11, 9)));
        let today = get(&conn, &view.today_query, d(2026, 11, 3)).unwrap();
        assert!(today.is_current);
        assert_eq!(today.next, None);
    }

    #[test]
    fn a_custom_range_today_cannot_reach_is_a_validation_error() {
        let conn = db();
        let query = custom(d(2026, 10, 2), d(2026, 10, 11));
        assert!(matches!(
            get(&conn, &query, NaiveDate::MAX),
            Err(AppError::Validation { field, .. }) if field == "period"
        ));
    }

    #[test]
    fn custom_needs_end_on_or_after_start() {
        let conn = db();
        assert_eq!(
            range(
                &conn,
                PeriodQuery::Custom {
                    start: d(2026, 1, 5),
                    end: d(2026, 1, 5)
                }
            ),
            (d(2026, 1, 5), d(2026, 1, 5))
        );
        let backwards = PeriodQuery::Custom {
            start: d(2026, 1, 5),
            end: d(2026, 1, 4),
        };
        assert!(matches!(
            resolve(&conn, &backwards, d(2026, 11, 3)),
            Err(AppError::Validation { field, .. }) if field == "period"
        ));
    }
}
