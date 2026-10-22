use std::sync::Arc;

use tauri::State;

use crate::core::error::AppError;
use crate::core::mcp_config::{self, McpHostAdapterDto, McpServerRecord, McpServerTarget};
use crate::core::skill_store::SkillStore;

#[tauri::command]
pub async fn get_mcp_servers(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<McpServerRecord>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::list_servers(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_mcp_server_targets(
    server_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<McpServerTarget>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::list_server_targets(&store, &server_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_mcp_host_adapters() -> Vec<McpHostAdapterDto> {
    mcp_config::mcp_host_adapter_dtos()
}

#[tauri::command]
pub async fn add_mcp_server(
    name: String,
    description: Option<String>,
    config_json: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<McpServerRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::add_server(&store, &name, description.as_deref(), &config_json)
            .map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn update_mcp_server_config(
    server_id: String,
    config_json: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::update_server_config(&store, &server_id, &config_json).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn deploy_mcp_server(
    server_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::deploy_server(&store, &server_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn undeploy_mcp_server(
    server_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::undeploy_server(&store, &server_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn delete_mcp_server(
    server_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        mcp_config::delete_server(&store, &server_id).map_err(AppError::db)
    })
    .await?
}
