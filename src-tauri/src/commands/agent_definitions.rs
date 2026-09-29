use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use crate::core::agent_definitions::{
    self, AgentDefinitionRecord, AgentDefinitionTarget, DiscoveredDefinition,
};
use crate::core::error::AppError;
use crate::core::skill_store::SkillStore;

#[tauri::command]
pub async fn get_agent_definitions(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<AgentDefinitionRecord>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::list_definitions(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_agent_definition_targets(
    definition_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<AgentDefinitionTarget>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::list_targets(&store, &definition_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_agent_definition_adapters() -> Vec<agent_definitions::SubagentAdapterDto> {
    agent_definitions::subagent_adapter_dtos()
}

#[tauri::command]
pub async fn scan_agent_definitions(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<DiscoveredDefinition>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::scan_discovered(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn import_agent_definition(
    source_path: String,
    format: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<AgentDefinitionRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::import_definition(&store, &PathBuf::from(&source_path), &format)
            .map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn deploy_agent_definition(
    definition_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::deploy_definition(&store, &definition_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn undeploy_agent_definition(
    definition_id: String,
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::undeploy_definition(&store, &definition_id, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn delete_agent_definition(
    definition_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        agent_definitions::delete_definition(&store, &definition_id).map_err(AppError::db)
    })
    .await?
}
