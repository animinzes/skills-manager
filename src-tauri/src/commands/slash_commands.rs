use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::core::error::AppError;
use crate::core::skill_store::SkillStore;
use crate::core::slash_commands::{
    self, CommandAdapterDto, DiscoveredCommand, SlashCommandRecord, SlashCommandTarget,
};

#[tauri::command]
pub async fn get_slash_commands(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<SlashCommandRecord>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::list_commands(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_slash_command_targets(
    command_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<SlashCommandTarget>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::list_targets(&store, &command_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_slash_command_adapters() -> Vec<CommandAdapterDto> {
    slash_commands::command_adapter_dtos()
}

#[tauri::command]
pub async fn scan_slash_commands(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<DiscoveredCommand>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::scan_discovered(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn import_slash_command(
    source_path: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SlashCommandRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::import_command(&store, &PathBuf::from(&source_path)).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn deploy_slash_command(
    command_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::deploy_command(&store, &command_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn undeploy_slash_command(
    command_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::undeploy_command(&store, &command_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn delete_slash_command(
    command_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        slash_commands::delete_command(&store, &command_id).map_err(AppError::db)
    })
    .await?
}
