use std::sync::Arc;

use tauri::State;

use crate::core::{
    audit_log::AuditDraft,
    error::AppError,
    governance::{SkillGovernanceInput, SkillGovernanceProfile},
    skill_store::SkillStore,
    sync_metadata,
};

#[tauri::command]
pub async fn get_skill_governance(
    skill_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SkillGovernanceProfile, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .get_skill_governance(&skill_id)
            .map_err(AppError::db)?
            .ok_or_else(|| AppError::not_found(format!("Skill '{skill_id}' was not found")))
    })
    .await?
}

#[tauri::command]
pub async fn save_skill_governance(
    input: SkillGovernanceInput,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SkillGovernanceProfile, AppError> {
    let input = input
        .validate_and_normalize()
        .map_err(|error| AppError::invalid_input(error.to_string()))?;
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let skill = store
            .get_skill_by_id(&input.skill_id)
            .map_err(AppError::db)?
            .ok_or_else(|| {
                AppError::not_found(format!("Skill '{}' was not found", input.skill_id))
            })?;
        let before = store
            .get_skill_governance(&input.skill_id)
            .map_err(AppError::db)?;
        let profile = sync_metadata::with_repo_lock("update skill governance", || {
            let profile = store.replace_skill_governance(input)?;
            sync_metadata::write_all_from_db_unlocked(&store)?;
            Ok(profile)
        })
        .map_err(AppError::db)?;
        store.log_audit(
            AuditDraft::new("update_governance")
                .skill(&skill.id, &skill.name)
                .detail(
                    serde_json::json!({ "before": before, "after": &profile }).to_string(),
                )
                .ok(),
        );
        Ok(profile)
    })
    .await?
}
