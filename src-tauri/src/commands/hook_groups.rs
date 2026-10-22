use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::core::error::AppError;
use crate::core::hook_groups::{
    self, HookGroupFile, HookGroupRecord, HookGroupRegistration, HookGroupTarget,
};
use crate::core::skill_store::SkillStore;

#[tauri::command]
pub async fn get_hook_groups(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<HookGroupRecord>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::list_groups(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_hook_group_files(
    group_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<HookGroupFile>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::list_group_files(&store, &group_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_hook_group_registrations(
    group_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<HookGroupRegistration>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::list_group_registrations(&store, &group_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_hook_group_targets(
    group_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<HookGroupTarget>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::list_group_targets(&store, &group_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn add_hook_group(
    name: String,
    description: Option<String>,
    store: State<'_, Arc<SkillStore>>,
) -> Result<HookGroupRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::add_group(&store, &name, description.as_deref()).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn add_hook_group_file(
    group_id: String,
    source_path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<HookGroupFile, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::add_group_file(&store, &group_id, &PathBuf::from(&source_path))
            .map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn add_hook_group_registration(
    group_id: String,
    event: String,
    matcher: Option<String>,
    command: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<HookGroupRegistration, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::add_group_registration(
            &store,
            &group_id,
            &event,
            matcher.as_deref(),
            &command,
        )
        .map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn remove_hook_group_registration(
    group_id: String,
    event: String,
    command: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::remove_group_registration(&store, &group_id, &event, &command)
            .map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn remove_hook_group_file(
    group_id: String,
    file_name: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::remove_group_file(&store, &group_id, &file_name).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn deploy_hook_group(
    group_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::deploy_group(&store, &group_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn undeploy_hook_group(
    group_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::undeploy_group(&store, &group_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn delete_hook_group(
    group_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        hook_groups::delete_group(&store, &group_id).map_err(AppError::db)
    })
    .await?
}
