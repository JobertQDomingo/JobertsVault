use serde::Serialize;
use ts_rs::TS;

/// Every error a command can return. The frontend reads it by `code` (ARCHITECTURE §4.4).
#[derive(thiserror::Error, Debug, Clone, Serialize, TS)]
#[serde(tag = "code", rename_all = "snake_case")]
#[ts(export)]
pub enum AppError {
    #[error("{message}")]
    Validation { field: String, message: String },
    #[error("{entity} {id} not found")]
    NotFound {
        entity: String,
        #[ts(type = "number")]
        id: i64,
    },
    #[error("{message}")]
    Conflict { message: String },
    #[error("{message}")]
    Database { message: String },
    #[error("{message}")]
    Io { message: String },
    /// A failed update check or install. The message says why, and that nothing changed.
    #[error("{message}")]
    Update { message: String },
}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        let message = match err {
            // These messages can hold the value itself, which could be an amount or a name.
            rusqlite::Error::IntegralValueOutOfRange(column, _) => {
                format!("Integer out of range at index {column}")
            }
            rusqlite::Error::FromSqlConversionFailure(column, kind, _) => {
                format!("Conversion error from type {kind} at index {column}")
            }
            rusqlite::Error::ToSqlConversionFailure(_) => "Conversion error to SQL".into(),
            err => err.to_string(),
        };
        Self::Database { message }
    }
}

impl From<rusqlite_migration::Error> for AppError {
    fn from(err: rusqlite_migration::Error) -> Self {
        match err {
            // Goes through the rusqlite conversion, which drops values and the SQL text.
            rusqlite_migration::Error::RusqliteError { err, .. } => err.into(),
            err => Self::Database {
                message: err.to_string(),
            },
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_error_crosses_ipc_tagged_by_code() {
        let declaration = AppError::decl(&ts_rs::Config::default());
        assert!(
            declaration.contains(r#""code": "not_found""#),
            "{declaration}"
        );
        assert!(declaration.contains("id: number"), "{declaration}");
    }
}
