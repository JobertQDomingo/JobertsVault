use std::collections::HashSet;

use chrono::{DateTime, Local};

use crate::error::AppError;

pub mod accounts;
pub mod backup;
pub mod budgets;
pub mod categories;
pub mod dashboard;
pub mod export;
pub mod goals;
pub mod loans;
pub mod payment_methods;
#[cfg(test)]
mod perf;
pub mod period;
pub mod recurring;
pub mod reports;
pub mod settings;
pub mod system;
pub mod transactions;

/// A Validation error on `field`, the camelCase name the form uses for `setError`.
fn invalid(field: &str, message: &str) -> AppError {
    AppError::Validation {
        field: field.into(),
        message: message.into(),
    }
}

/// An `io` error that starts with `what` and ends with the cause.
fn failed<E: std::fmt::Display>(what: &str) -> impl Fn(E) -> AppError + '_ {
    move |err| AppError::Io {
        message: format!("{what} {err}"),
    }
}

fn not_found(entity: &str, id: i64) -> AppError {
    AppError::NotFound {
        entity: entity.into(),
        id,
    }
}

/// `created_at` / `updated_at` text (DATA_MODEL §1).
fn stamp(now: DateTime<Local>) -> String {
    now.to_utc().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// The order a reorder stores: `ids` first, then the rest of `existing` in its current order, so
/// records a list leaves out (e.g. archived ones it doesn't drag) never share a position.
fn new_order(ids: Vec<i64>, existing: impl IntoIterator<Item = i64>) -> Result<Vec<i64>, AppError> {
    let mut seen = HashSet::new();
    if !ids.iter().all(|id| seen.insert(*id)) {
        return Err(invalid("ids", "Each one can appear only once"));
    }
    let rest = existing.into_iter().filter(|id| !seen.contains(id));
    Ok(ids.into_iter().chain(rest).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_order_puts_the_given_ids_first() {
        assert_eq!(new_order(vec![3, 1], [1, 2, 3, 4]).unwrap(), [3, 1, 2, 4]);
        assert!(matches!(
            new_order(vec![1, 1], [1, 2]),
            Err(AppError::Validation { field, .. }) if field == "ids"
        ));
    }
}
