use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::core::{
    audit_log::{self, AuditDraft, AuditEntry},
    error::AppError,
    governance::SkillGovernanceProfile,
    skill_store::{SkillRecord, SkillStore, SkillTargetRecord},
    suite::{SuiteInput, SuiteRecord},
    sync_engine, sync_metadata,
};

#[derive(Debug, Serialize)]
pub struct RegistrySkill {
    pub skill: SkillRecord,
    pub governance: SkillGovernanceProfile,
    pub tags: Vec<String>,
    pub targets: Vec<SkillTargetRecord>,
    pub suite_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistryResource {
    pub resource_type: String,
    pub name: String,
    pub skill_ids: Vec<String>,
    pub required_by: usize,
}

#[derive(Debug, Serialize)]
pub struct RegistrySnapshot {
    pub skills: Vec<RegistrySkill>,
    pub suites: Vec<SuiteRecord>,
    pub resources: Vec<RegistryResource>,
    pub history: Vec<AuditEntry>,
    pub registry_path: String,
    pub history_path: String,
}

#[tauri::command]
pub async fn get_registry(store: State<'_, Arc<SkillStore>>) -> Result<RegistrySnapshot, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || build_registry(&store)).await?
}

fn build_registry(store: &SkillStore) -> Result<RegistrySnapshot, AppError> {
    let suites = store.get_all_suites().map_err(AppError::db)?;
    let tags = store.get_tags_map().map_err(AppError::db)?;
    let targets = store.get_all_targets().map_err(AppError::db)?;
    let mut resources: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut skills = Vec::new();

    for skill in store.get_all_skills().map_err(AppError::db)? {
        let governance = store
            .get_skill_governance(&skill.id)
            .map_err(AppError::db)?
            .ok_or_else(|| AppError::not_found(format!("Skill '{}' was not found", skill.id)))?;
        for dependency in &governance.dependencies {
            resources
                .entry((dependency.dependency_type.clone(), dependency.name.clone()))
                .or_default()
                .insert(skill.id.clone());
        }
        let suite_ids = suites
            .iter()
            .filter(|suite| {
                suite
                    .members
                    .iter()
                    .any(|member| member.skill_id == skill.id)
            })
            .map(|suite| suite.id.clone())
            .collect();
        skills.push(RegistrySkill {
            tags: tags.get(&skill.id).cloned().unwrap_or_default(),
            targets: targets
                .iter()
                .filter(|target| target.skill_id == skill.id)
                .cloned()
                .collect(),
            skill,
            governance,
            suite_ids,
        });
    }

    let resources = resources
        .into_iter()
        .map(|((resource_type, name), skill_ids)| {
            let skill_ids = skill_ids.into_iter().collect::<Vec<_>>();
            RegistryResource {
                resource_type,
                name,
                required_by: skill_ids.len(),
                skill_ids,
            }
        })
        .collect();
    let history = audit_log::read_persistent(Some(500))
        .unwrap_or_default()
        .into_iter()
        .chain(store.list_audit(Some(500)).unwrap_or_default())
        .fold(BTreeMap::new(), |mut unique, entry| {
            let key = (
                entry.ts,
                entry.id,
                entry.action.clone(),
                entry.skill_id.clone(),
                entry.tool.clone(),
                entry.detail.clone(),
            );
            unique.entry(key).or_insert(entry);
            unique
        })
        .into_values()
        .rev()
        .take(500)
        .collect();
    let registry_path = sync_metadata::registry_dir();
    let history_path = audit_log::history_path();

    Ok(RegistrySnapshot {
        skills,
        suites,
        resources,
        history,
        registry_path: registry_path.to_string_lossy().to_string(),
        history_path: history_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn save_suite(
    input: SuiteInput,
    store: State<'_, Arc<SkillStore>>,
) -> Result<SuiteRecord, AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let before = input
            .id
            .as_deref()
            .map(|id| store.get_suite(id))
            .transpose()
            .map_err(AppError::db)?
            .flatten();
        let suite = sync_metadata::with_repo_lock("save suite", || {
            let suite = store.replace_suite(input)?;
            sync_metadata::write_all_from_db_unlocked(&store)?;
            Ok(suite)
        })
        .map_err(AppError::db)?;
        store.log_audit(
            AuditDraft::new("save_suite")
                .detail(serde_json::json!({ "before": before, "after": &suite }).to_string())
                .ok(),
        );
        Ok(suite)
    })
    .await?
}

#[tauri::command]
pub async fn delete_suite(
    suite_id: String,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let suite = store
            .get_suite(&suite_id)
            .map_err(AppError::db)?
            .ok_or_else(|| AppError::not_found(format!("Suite '{suite_id}' was not found")))?;
        sync_metadata::with_repo_lock("delete suite", || {
            store.delete_suite(&suite_id)?;
            sync_metadata::write_all_from_db_unlocked(&store)
        })
        .map_err(AppError::db)?;
        store.log_audit(
            AuditDraft::new("delete_suite")
                .detail(serde_json::json!({ "before": suite, "after": null }).to_string())
                .ok(),
        );
        Ok(())
    })
    .await?
}

#[tauri::command]
pub async fn set_suite_deployed(
    suite_id: String,
    tool: String,
    enabled: bool,
    store: State<'_, Arc<SkillStore>>,
) -> Result<(), AppError> {
    let store = store.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let suite = store
            .get_suite(&suite_id)
            .map_err(AppError::db)?
            .ok_or_else(|| AppError::not_found(format!("Suite '{suite_id}' was not found")))?;
        let previously_deployed = store
            .get_all_targets()
            .map_err(AppError::db)?
            .into_iter()
            .filter(|target| target.tool == tool)
            .map(|target| target.skill_id)
            .collect::<HashSet<_>>();

        let outcome = if enabled {
            let mut newly_deployed = Vec::new();
            let mut result = Ok(());
            for member in &suite.members {
                if let Err(error) =
                    super::sync::sync_skill_to_tool_internal(&store, &member.skill_id, &tool)
                {
                    result = Err(error);
                    break;
                }
                if !previously_deployed.contains(&member.skill_id) {
                    newly_deployed.push(member.skill_id.clone());
                }
            }
            if result.is_err() {
                for skill_id in newly_deployed.into_iter().rev() {
                    let _ = remove_suite_member_target(&store, &skill_id, &tool);
                }
            }
            result
        } else {
            let mut removed_members = Vec::new();
            let mut result = Ok(());
            for member in &suite.members {
                if let Err(error) = remove_suite_member_target(&store, &member.skill_id, &tool) {
                    result = Err(error);
                    break;
                }
                if previously_deployed.contains(&member.skill_id) {
                    removed_members.push(member.skill_id.clone());
                }
            }
            if result.is_err() {
                for skill_id in removed_members.into_iter().rev() {
                    let _ = super::sync::sync_skill_to_tool_internal(&store, &skill_id, &tool);
                }
            }
            result
        };

        let mut audit = AuditDraft::new(if enabled {
            "deploy_suite"
        } else {
            "undeploy_suite"
        })
        .tool(tool.clone())
        .detail(
            serde_json::json!({
                "suite_id": suite_id,
                "suite_name": suite.name,
                "member_skill_ids": suite
                    .members
                    .iter()
                    .map(|member| &member.skill_id)
                    .collect::<Vec<_>>(),
                "enabled": enabled,
            })
            .to_string(),
        );
        audit = match &outcome {
            Ok(()) => audit.ok(),
            Err(error) => audit.fail(error.to_string()),
        };
        store.log_audit(audit);
        outcome
    })
    .await?
}

fn remove_suite_member_target(
    store: &SkillStore,
    skill_id: &str,
    tool: &str,
) -> Result<(), AppError> {
    if let Some(target) = store
        .get_targets_for_skill(skill_id)
        .map_err(AppError::db)?
        .into_iter()
        .find(|target| target.tool == tool)
    {
        let target_path = PathBuf::from(&target.target_path);
        let shared = store
            .get_all_targets()
            .map_err(AppError::db)?
            .into_iter()
            .any(|other| {
                other.target_path == target.target_path
                    && !(other.skill_id == skill_id && other.tool == tool)
            });
        if !shared {
            sync_engine::remove_recorded_target(&target_path, &target.mode)
                .map_err(AppError::io)?;
        }
    }
    store.delete_target(skill_id, tool).map_err(AppError::db)
}
