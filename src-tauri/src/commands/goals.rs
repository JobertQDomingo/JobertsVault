use tauri::State;

use crate::error::AppError;
use crate::services::goals::{Goal, GoalDraft, GoalFields, GoalProgress, GoalSuggestion};
use crate::state::AppState;
use crate::{log, services};

#[tauri::command(async)]
pub fn list_goals(state: State<'_, AppState>) -> Result<Vec<GoalProgress>, AppError> {
    services::goals::list(&mut *state.conn()?, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn create_goal(state: State<'_, AppState>, goal: GoalFields) -> Result<Goal, AppError> {
    services::goals::create(&mut *state.conn()?, goal, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

/// GOL-5: what the goal dialog suggests as the user types; none with no payday rule.
#[tauri::command(async)]
pub fn suggest_goal(
    state: State<'_, AppState>,
    draft: GoalDraft,
) -> Result<Option<GoalSuggestion>, AppError> {
    services::goals::suggest(&*state.conn()?, draft, chrono::Local::now().date_naive())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn update_goal(
    state: State<'_, AppState>,
    id: i64,
    goal: GoalFields,
) -> Result<Goal, AppError> {
    services::goals::update(&mut *state.conn()?, id, goal, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn archive_goal(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::goals::archive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn unarchive_goal(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::goals::unarchive(&mut *state.conn()?, id, chrono::Local::now())
        .inspect_err(|err| log::record(&state.log_path, err))
}

#[tauri::command(async)]
pub fn delete_goal(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    services::goals::delete(&mut *state.conn()?, id)
        .inspect_err(|err| log::record(&state.log_path, err))
}
