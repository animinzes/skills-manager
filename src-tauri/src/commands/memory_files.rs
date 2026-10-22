use std::sync::Arc;

use tauri::State;

use crate::core::error::AppError;
use crate::core::memory_files::{self, MemoryFileRecord, MemorySlotDto};
use crate::core::skill_store::SkillStore;

#[tauri::command]
pub async fn get_memory_slots(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<MemorySlotDto>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        memory_files::memory_slots(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn get_memory_files(
    store: State<'_, Arc<SkillStore>>,
) -> Result<Vec<MemoryFileRecord>, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        memory_files::list_memory_files(&store).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn register_memory_file(
    tool: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<MemoryFileRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        memory_files::register_memory_file(&store, &tool).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn refresh_memory_file(
    registration_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<MemoryFileRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        memory_files::refresh_memory_file(&store, &registration_id).map_err(AppError::db)
    })
    .await?
}

#[tauri::command]
pub async fn delete_memory_file(
    registration_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        memory_files::delete_memory_file(&store, &registration_id).map_err(AppError::db)
    })
    .await?
}
