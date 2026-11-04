use crate::error::AppError;

pub mod accounts;
pub mod settings;
pub mod system;

/// A Validation error on `field`, the camelCase name the form uses for `setError`.
fn invalid(field: &str, message: &str) -> AppError {
    AppError::Validation {
        field: field.into(),
        message: message.into(),
    }
}
