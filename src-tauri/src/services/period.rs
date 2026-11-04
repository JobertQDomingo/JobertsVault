use chrono::{NaiveDate, Weekday};
use rusqlite::Connection;
use serde::Deserialize;
use ts_rs::TS;

use crate::domain::period::{Period, PeriodKind, PeriodSettings};
use crate::error::AppError;
use crate::services::invalid;
use crate::services::settings::{self, WeekStart};

/// Which period a screen shows (ARCHITECTURE §4.5). No anchor means the one containing today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum PeriodQuery {
    Day { anchor: Option<NaiveDate> },
    Week { anchor: Option<NaiveDate> },
    Month { anchor: Option<NaiveDate> },
    Year { anchor: Option<NaiveDate> },
    Custom { start: NaiveDate, end: NaiveDate },
}

/// The dates `query` covers, using the week and month start settings (DATA_MODEL §5.3).
pub fn resolve(
    conn: &Connection,
    query: &PeriodQuery,
    today: NaiveDate,
) -> Result<Period, AppError> {
    let s = settings::get(conn)?.settings;
    let period_settings = PeriodSettings {
        week_start: match s.week_start {
            WeekStart::Monday => Weekday::Mon,
            WeekStart::Sunday => Weekday::Sun,
        },
        month_start_day: s.month_start_day.into(),
    };
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
    Period::containing(kind, anchor.unwrap_or(today), &period_settings)
        .ok_or_else(|| invalid("period", "That date is outside the supported range"))
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
