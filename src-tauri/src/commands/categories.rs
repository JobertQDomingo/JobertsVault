use tauri::State;

use crate::error::AppError;
use crate::services::categories::{Category, CategoryFields, CategoryList, NewCategory};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_categories(state: State<'_, AppState>) -> Result<CategoryList, AppError> {
    services::categories::list(&*state.conn()?).inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_category(
    state: State<'_, AppState>,
    category: NewCategory,
) -> Result<Category, AppError> {
    services::categories::create(&mut *state.conn()?, category, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_category(
    state: State<'_, AppState>,
    id: i64,
    category: CategoryFields,
) -> Result<Category, AppError> {
    services::categories::update(&mut *state.conn()?, id, category, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn reorder_categories(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), AppError> {
    services::categories::reorder(&mut *state.conn()?, ids, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn archive_category(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::categories::archive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn unarchive_category(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::categories::unarchive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}
