use anyhow::{bail, Context, Result};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::core::audit_log::AuditDraft;
use crate::core::central_repo;
use crate::core::mcp_config::pretty_write;
use crate::core::skill_store::SkillStore;

/// A hook group: a set of script files plus the registration entries that
/// wire them into a host. Deploy copies the scripts under the host's hooks
/// directory (`~/.claude/hooks/<group>/` for Claude Code) and appends one
/// matcher-group per (event, matcher) to the `hooks` key of the host's
/// settings file. Undeploy removes exactly the entries that were written
/// and the script directory, refusing when anything drifted underneath us.
#[derive(Debug, Clone, Serialize)]
pub struct HookGroupRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HookGroupFile {
    pub file_name: String,
    pub central_path: String,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HookGroupRegistration {
    pub event: String,
    pub matcher: Option<String>,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HookGroupTarget {
    pub tool: String,
    pub hooks_dir: String,
    pub settings_path: String,
    pub synced_at: i64,
}

pub const HOOK_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "UserPromptSubmit",
    "Notification",
    "Stop",
    "SubagentStop",
    "PreCompact",
    "SessionStart",
    "SessionEnd",
];

/// Hook-capable host. Claude Code only in this round — its layout is the
/// template; AGY's hook system needs its own adapter once its file
/// conventions are pinned down.
pub struct HookHostAdapter {
    pub tool_key: &'static str,
    pub display_name: &'static str,
    pub hooks_dir: PathBuf,
    pub settings_path: PathBuf,
}

pub fn hook_host_adapters() -> Vec<HookHostAdapter> {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    vec![HookHostAdapter {
        tool_key: "claude_code",
        display_name: "Claude Code",
        hooks_dir: home.join(".claude/hooks"),
        settings_path: home.join(".claude/settings.json"),
    }]
}

pub fn central_hooks_dir() -> PathBuf {
    central_repo::base_dir().join("hooks")
}

fn file_hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn new_id() -> String {
    let seed = format!("{:?}-{}", std::time::SystemTime::now(), std::process::id());
    let digest = Sha256::digest(seed.as_bytes());
    digest[..8].iter().map(|b| format!("{b:02x}")).collect()
}

// ── Registry CRUD ──

pub fn list_groups(store: &SkillStore) -> Result<Vec<HookGroupRecord>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, name, description, created_at, updated_at \
                      FROM hook_groups ORDER BY name")?;
        let collected: Vec<HookGroupRecord> = stmt
            .query_map([], |row| {
                Ok(HookGroupRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn get_group(store: &SkillStore, id: &str) -> Result<Option<HookGroupRecord>> {
    let row = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at \
             FROM hook_groups WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |row| {
            Ok(HookGroupRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        Ok(match rows.next() {
            Some(row) => Some(row?),
            None => None,
        })
    })?;
    Ok(row)
}

pub fn list_group_files(store: &SkillStore, group_id: &str) -> Result<Vec<HookGroupFile>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT file_name, central_path, content_hash FROM hook_group_files \
             WHERE group_id = ?1 ORDER BY file_name",
        )?;
        let collected: Vec<HookGroupFile> = stmt
            .query_map(params![group_id], |row| {
                Ok(HookGroupFile {
                    file_name: row.get(0)?,
                    central_path: row.get(1)?,
                    content_hash: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn list_group_registrations(
    store: &SkillStore,
    group_id: &str,
) -> Result<Vec<HookGroupRegistration>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT event, matcher, command FROM hook_group_registrations \
             WHERE group_id = ?1 ORDER BY event, command",
        )?;
        let collected: Vec<HookGroupRegistration> = stmt
            .query_map(params![group_id], |row| {
                Ok(HookGroupRegistration {
                    event: row.get(0)?,
                    matcher: row.get(1)?,
                    command: row.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn list_group_targets(store: &SkillStore, group_id: &str) -> Result<Vec<HookGroupTarget>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT tool, hooks_dir, settings_path, synced_at FROM hook_group_targets \
             WHERE group_id = ?1 ORDER BY tool",
        )?;
        let collected: Vec<HookGroupTarget> = stmt
            .query_map(params![group_id], |row| {
                Ok(HookGroupTarget {
                    tool: row.get(0)?,
                    hooks_dir: row.get(1)?,
                    settings_path: row.get(2)?,
                    synced_at: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn add_group(store: &SkillStore, name: &str, description: Option<&str>) -> Result<HookGroupRecord> {
    let name = name.trim();
    if name.is_empty() {
        bail!("hook group name cannot be empty");
    }
    let exists: bool = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM hook_groups WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
        })
        .unwrap_or(false);
    if exists {
        bail!("a hook group named \"{name}\" already exists");
    }
    let now = now_secs();
    let id = new_id();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO hook_groups (id, name, description, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![id, name, description, now],
        )
    })?;
    store.log_audit(AuditDraft::new("hook_group_add").ok());
    Ok(HookGroupRecord {
        id,
        name: name.to_string(),
        description: description.map(|d| d.to_string()),
        created_at: now,
        updated_at: now,
    })
}

/// Copy one script file into the group's central directory and register it.
pub fn add_group_file(store: &SkillStore, group_id: &str, source: &Path) -> Result<HookGroupFile> {
    get_group(store, group_id)?.with_context(|| format!("hook group {group_id} not found"))?;
    if !source.is_file() {
        bail!("{} is not a file", source.display());
    }
    let file_name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .with_context(|| format!("{} has no file name", source.display()))?;
    let dir = central_hooks_dir().join(group_id);
    std::fs::create_dir_all(&dir)?;
    let central_path = dir.join(&file_name);

    let exists: bool = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM hook_group_files WHERE group_id = ?1 AND file_name = ?2)",
                params![group_id, file_name],
                |row| row.get(0),
            )
        })
        .unwrap_or(false);
    if exists {
        bail!("the group already has a file named \"{file_name}\"");
    }

    std::fs::copy(source, &central_path)
        .with_context(|| format!("copy {} -> {}", source.display(), central_path.display()))?;
    let hash = file_hash(&central_path)?;
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO hook_group_files (group_id, file_name, central_path, content_hash) \
             VALUES (?1, ?2, ?3, ?4)",
            params![group_id, file_name, central_path.to_string_lossy().into_owned(), hash],
        )
    })?;
    store.log_audit(AuditDraft::new("hook_group_file_add").ok());
    Ok(HookGroupFile {
        file_name,
        central_path: central_path.to_string_lossy().into_owned(),
        content_hash: Some(hash),
    })
}

pub fn add_group_registration(
    store: &SkillStore,
    group_id: &str,
    event: &str,
    matcher: Option<&str>,
    command: &str,
) -> Result<HookGroupRegistration> {
    get_group(store, group_id)?.with_context(|| format!("hook group {group_id} not found"))?;
    if !HOOK_EVENTS.contains(&event) {
        bail!(
            "unknown hook event {event:?}; expected one of {}",
            HOOK_EVENTS.join(", ")
        );
    }
    let command = command.trim();
    if command.is_empty() {
        bail!("hook command cannot be empty");
    }
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO hook_group_registrations (group_id, event, matcher, command) \
             VALUES (?1, ?2, ?3, ?4)",
            params![group_id, event, matcher, command],
        )
    })?;
    store.log_audit(AuditDraft::new("hook_group_registration_add").ok());
    Ok(HookGroupRegistration {
        event: event.to_string(),
        matcher: matcher.map(|m| m.to_string()),
        command: command.to_string(),
    })
}

pub fn remove_group_registration(
    store: &SkillStore,
    group_id: &str,
    event: &str,
    command: &str,
) -> Result<()> {
    store.with_conn(|conn| {
        conn.execute(
            "DELETE FROM hook_group_registrations \
             WHERE group_id = ?1 AND event = ?2 AND command = ?3",
            params![group_id, event, command],
        )
    })?;
    store.log_audit(AuditDraft::new("hook_group_registration_remove").ok());
    Ok(())
}

pub fn remove_group_file(store: &SkillStore, group_id: &str, file_name: &str) -> Result<()> {
    let central: Option<String> = store.with_conn(|conn| {
        conn.query_row(
            "SELECT central_path FROM hook_group_files WHERE group_id = ?1 AND file_name = ?2",
            params![group_id, file_name],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|e| if e == rusqlite::Error::QueryReturnedNoRows { Ok(None) } else { Err(e) })
    })?;
    store.with_conn(|conn| {
        conn.execute(
            "DELETE FROM hook_group_files WHERE group_id = ?1 AND file_name = ?2",
            params![group_id, file_name],
        )
    })?;
    if let Some(central) = central {
        let p = PathBuf::from(&central);
        if p.exists() {
            std::fs::remove_file(&p).ok();
        }
    }
    store.log_audit(AuditDraft::new("hook_group_file_remove").ok());
    Ok(())
}

// ── Settings merge (Claude Code hooks key) ──

/// Build the matcher-group entries we will append, one per (event, matcher):
/// `{matcher?, hooks: [{type: "command", command}, ...]}`.
pub fn build_written_json(
    registrations: &[HookGroupRegistration],
    script_dir: &Path,
) -> Map<String, Value> {
    // (event, matcher) -> commands, in first-seen order
    let mut order: Vec<(String, Option<String>)> = Vec::new();
    let mut grouped: std::collections::BTreeMap<(String, Option<String>), Vec<String>> =
        Default::default();
    for reg in registrations {
        let command = if Path::new(&reg.command).is_absolute() || reg.command.contains('/') || reg.command.contains('\\') {
            reg.command.clone()
        } else {
            // bare script name: run our copy out of the deployed group dir
            script_dir
                .join(&reg.command)
                .to_string_lossy()
                .into_owned()
        };
        let key = (reg.event.clone(), reg.matcher.clone());
        if !grouped.contains_key(&key) {
            order.push(key.clone());
        }
        grouped.entry(key).or_default().push(command);
    }
    let mut out = Map::new();
    for key in order {
        let commands = grouped.remove(&key).unwrap_or_default();
        let mut group = Map::new();
        if let Some(matcher) = &key.1 {
            group.insert("matcher".into(), Value::String(matcher.clone()));
        }
        group.insert(
            "hooks".into(),
            Value::Array(
                commands
                    .into_iter()
                    .map(|c| {
                        Value::Object({
                            let mut h = Map::new();
                            h.insert("type".into(), Value::String("command".into()));
                            h.insert("command".into(), Value::String(c));
                            h
                        })
                    })
                    .collect(),
            ),
        );
        out.insert(key.0.clone(), Value::Array(vec![Value::Object(group)]));
    }
    out
}

/// Append our entries to the `hooks` map of a settings document and write it
/// back (merge-only: existing keys and array elements are untouched).
pub fn merge_written_into_settings(
    settings_path: &Path,
    written: &Map<String, Value>,
) -> Result<()> {
    let text = std::fs::read_to_string(settings_path)
        .with_context(|| format!("cannot read {}", settings_path.display()))?;
    let mut doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", settings_path.display()))?;
    if !doc.is_object() {
        bail!("{} does not contain a JSON object", settings_path.display());
    }
    let obj = doc.as_object_mut().unwrap();
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks_map = hooks
        .as_object_mut()
        .with_context(|| "the \"hooks\" key is not an object")?;
    for (event, entries) in written {
        let arr = hooks_map
            .entry(event.clone())
            .or_insert_with(|| Value::Array(Vec::new()));
        let list = arr
            .as_array_mut()
            .with_context(|| format!("hooks.{event} is not an array"))?;
        list.extend(
            entries
                .as_array()
                .context("written entries must be an array")?
                .iter()
                .cloned(),
        );
    }
    pretty_write(settings_path, &doc, &text)
}

/// Remove exactly the entries `written` describes from the settings file.
/// Returns the number removed; an entry that is missing or was modified is
/// counted as a drift and reported as an error.
pub fn remove_written_from_settings(
    settings_path: &Path,
    written: &Map<String, Value>,
) -> Result<usize> {
    let text = std::fs::read_to_string(settings_path)
        .with_context(|| format!("cannot read {}", settings_path.display()))?;
    let mut doc: Value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON", settings_path.display()))?;
    let obj = doc
        .as_object_mut()
        .with_context(|| format!("{} does not contain a JSON object", settings_path.display()))?;
    let mut removed = 0usize;
    let mut emptied_events: Vec<String> = Vec::new();
    let hooks = match obj.get_mut("hooks") {
        Some(h) => h,
        None => bail!("the settings file has no \"hooks\" key; nothing to roll back"),
    };
    let hooks_map = hooks
        .as_object_mut()
        .with_context(|| "the \"hooks\" key is not an object")?;
    for (event, entries) in written {
        let expected = entries
            .as_array()
            .context("written entries must be an array")?;
        let arr = hooks_map
            .get_mut(event.as_str())
            .with_context(|| format!("hooks.{event} is missing; was the registration edited?"))?;
        let list = arr
            .as_array_mut()
            .with_context(|| format!("hooks.{event} is not an array"))?;
        for entry in expected {
            let before = list.len();
            list.retain(|item| item != entry);
            if list.len() == before {
                bail!(
                    "an entry written by this deployment is gone or modified in hooks.{event}; \
                     restore it or clean up manually"
                );
            }
            removed += 1;
        }
        if list.is_empty() {
            emptied_events.push(event.clone());
        }
    }
    for event in emptied_events {
        hooks_map.shift_remove(&event);
    }
    pretty_write(settings_path, &doc, &text)?;
    Ok(removed)
}

// ── Deploy / undeploy ──

pub fn deploy_group(store: &SkillStore, group_id: &str, tool: &str) -> Result<()> {
    let group =
        get_group(store, group_id)?.with_context(|| format!("hook group {group_id} not found"))?;
    let adapter = hook_host_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no hook location configured"))?;
    if !adapter.settings_path.is_file() {
        bail!(
            "{} does not exist; launch the agent once before deploying hooks",
            adapter.settings_path.display()
        );
    }
    let files = list_group_files(store, group_id)?;
    let registrations = list_group_registrations(store, group_id)?;
    if files.is_empty() || registrations.is_empty() {
        bail!("add at least one script file and one registration before deploying");
    }

    let script_dir = adapter.hooks_dir.join(&group.name);
    if !script_dir.exists() {
        std::fs::create_dir_all(&script_dir)?;
    }
    for file in &files {
        let target = script_dir.join(&file.file_name);
        if target.exists() {
            let target_hash = file_hash(&target).ok();
            if target_hash != file.content_hash && list_group_targets(store, group_id)?
                .into_iter()
                .all(|t| t.tool != tool)
            {
                bail!(
                    "{} exists and differs from the central copy; remove it first",
                    target.display()
                );
            }
        }
        let central = PathBuf::from(&file.central_path);
        std::fs::copy(&central, &target)
            .with_context(|| format!("copy {} -> {}", central.display(), target.display()))?;
    }

    let written = build_written_json(&registrations, &script_dir);
    let backup_dir = central_repo::base_dir().join("backups").join("hooks").join(tool);
    std::fs::create_dir_all(&backup_dir)?;
    let backup = backup_dir.join(format!("{}-{}.json", now_secs(), group.name));
    std::fs::copy(&adapter.settings_path, &backup).with_context(|| {
        format!(
            "backup {} -> {}",
            adapter.settings_path.display(),
            backup.display()
        )
    })?;
    merge_written_into_settings(&adapter.settings_path, &written)?;

    let now = now_secs();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO hook_group_targets \
             (group_id, tool, hooks_dir, settings_path, backup_path, written_json, synced_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(group_id, tool) DO UPDATE SET \
             hooks_dir = ?3, settings_path = ?4, backup_path = ?5, \
             written_json = ?6, synced_at = ?7",
            params![
                group_id,
                tool,
                script_dir.to_string_lossy().into_owned(),
                adapter.settings_path.to_string_lossy().into_owned(),
                backup.to_string_lossy().into_owned(),
                Value::Object(written.clone()).to_string(),
                now
            ],
        )
    })?;
    store.log_audit(AuditDraft::new("hook_group_deploy").ok());
    Ok(())
}

pub fn undeploy_group(store: &SkillStore, group_id: &str, tool: &str) -> Result<()> {
    let group =
        get_group(store, group_id)?.with_context(|| format!("hook group {group_id} not found"))?;
    let adapter = hook_host_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no hook location configured"))?;

    let row: Option<(String, String)> = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT written_json, hooks_dir FROM hook_group_targets \
             WHERE group_id = ?1 AND tool = ?2",
        )?;
        let mut rows = stmt.query_map(params![group_id, tool], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        Ok(match rows.next() {
            Some(r) => Some(r?),
            None => None,
        })
    })?;
    let (written_json, hooks_dir) = row
        .with_context(|| format!("hook group {} is not deployed to {tool}", group.name))?;
    let written: Map<String, Value> = serde_json::from_str::<Value>(&written_json)
        .context("stored registration is not valid JSON")?
        .as_object()
        .context("stored registration is not an object")?
        .clone();

    remove_written_from_settings(&adapter.settings_path, &written)?;

    // Remove the script dir only when every file still matches what we
    // deployed; user-edited scripts are left in place.
    let dir = PathBuf::from(&hooks_dir);
    if dir.is_dir() {
        let files = list_group_files(store, group_id)?;
        let mut safe = true;
        for file in &files {
            let target = dir.join(&file.file_name);
            if let Ok(hash) = file_hash(&target) {
                if Some(hash) != file.content_hash {
                    safe = false;
                }
            }
        }
        if safe {
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    store.with_conn(|conn| {
        conn.execute(
            "DELETE FROM hook_group_targets WHERE group_id = ?1 AND tool = ?2",
            params![group_id, tool],
        )
    })?;
    store.log_audit(AuditDraft::new("hook_group_undeploy").ok());
    Ok(())
}

/// Delete a group after undeploying everywhere and clearing the central
/// script directory.
pub fn delete_group(store: &SkillStore, group_id: &str) -> Result<()> {
    for target in list_group_targets(store, group_id)? {
        undeploy_group(store, group_id, &target.tool)?;
    }
    store.with_conn(|conn| {
        conn.execute("DELETE FROM hook_groups WHERE id = ?1", params![group_id])
    })?;
    let central = central_hooks_dir().join(group_id);
    if central.is_dir() {
        std::fs::remove_dir_all(&central).ok();
    }
    store.log_audit(AuditDraft::new("hook_group_delete").ok());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in(tmp: &Path) -> SkillStore {
        SkillStore::new(&tmp.join("test.db")).unwrap()
    }

    fn setup_group(store: &SkillStore, tmp: &Path) -> (HookGroupRecord, PathBuf) {
        let group = add_group(store, "guard", Some("test hooks")).unwrap();
        let script = tmp.join("guard.js");
        std::fs::write(&script, "console.log('guard')\n").unwrap();
        add_group_file(store, &group.id, &script).unwrap();
        add_group_registration(
            store,
            &group.id,
            "PreToolUse",
            Some("Bash"),
            "node guard.js",
        )
        .unwrap();
        (group, script)
    }

    #[test]
    fn written_json_shapes_matcher_groups() {
        let regs = vec![
            HookGroupRegistration {
                event: "PreToolUse".into(),
                matcher: Some("Bash".into()),
                command: "guard.js".into(),
            },
            HookGroupRegistration {
                event: "SessionStart".into(),
                matcher: None,
                command: "C:\\abs\\start.js".into(),
            },
        ];
        let written = build_written_json(&regs, Path::new("C:\\h\\guard"));
        assert_eq!(written.len(), 2);
        let pre = written["PreToolUse"][0].as_object().unwrap();
        assert_eq!(pre["matcher"], "Bash");
        assert_eq!(pre["hooks"][0]["command"], "C:\\h\\guard\\guard.js");
        let session = written["SessionStart"][0].as_object().unwrap();
        assert!(session.get("matcher").is_none());
        assert_eq!(session["hooks"][0]["command"], "C:\\abs\\start.js");
    }

    /// Success + rollback against a temp settings file: entries are appended
    /// without touching existing keys, and removed exactly on rollback.
    #[test]
    fn settings_merge_and_rollback_roundtrip() {
        let tmp = tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        std::fs::write(
            &settings,
            "{\n  \"model\": \"sonnet\",\n  \"hooks\": {\n    \"Stop\": [\n      {\"hooks\": [{\"type\": \"command\", \"command\": \"keep.sh\"}]}\n    ]\n  }\n}\n",
        )
        .unwrap();

        let written: Map<String, Value> = serde_json::from_str(
            r#"{"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "C:\\h\\guard\\guard.js"}]}]}"#,
        )
        .unwrap();
        merge_written_into_settings(&settings, &written).unwrap();

        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert_eq!(doc["model"], "sonnet");
        assert_eq!(doc["hooks"]["Stop"][0]["hooks"][0]["command"], "keep.sh");
        assert_eq!(doc["hooks"]["PreToolUse"][0]["matcher"], "Bash");
        assert!(std::fs::read_to_string(&settings).unwrap().ends_with('\n'));

        let removed = remove_written_from_settings(&settings, &written).unwrap();
        assert_eq!(removed, 1);
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        assert!(after["hooks"].get("PreToolUse").is_none());
        assert_eq!(after["hooks"]["Stop"][0]["hooks"][0]["command"], "keep.sh");
        assert_eq!(after["model"], "sonnet");
    }

    /// Rollback refuses when our written entry was modified on disk.
    #[test]
    fn rollback_refuses_on_drift() {
        let tmp = tempdir().unwrap();
        let settings = tmp.path().join("settings.json");
        std::fs::write(&settings, "{\"hooks\": {}}").unwrap();
        let written: Map<String, Value> = serde_json::from_str(
            r#"{"Stop": [{"hooks": [{"type": "command", "command": "a.sh"}]}]}"#,
        )
        .unwrap();
        merge_written_into_settings(&settings, &written).unwrap();
        // user edits our entry
        std::fs::write(
            &settings,
            r#"{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "edited.sh"}]}]}}"#,
        )
        .unwrap();
        assert!(remove_written_from_settings(&settings, &written).is_err());
    }

    /// Registry round-trip with a temp central repo: group CRUD refuses
    /// duplicates, registrations validate events.
    #[test]
    fn group_registry_crud() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());

        let (group, script) = setup_group(&store, tmp.path());
        assert!(add_group(&store, "guard", None).is_err());
        assert!(add_group_file(&store, &group.id, &script).is_err());
        assert!(
            add_group_registration(&store, &group.id, "NotAnEvent", None, "x")
                .is_err()
        );

        let files = list_group_files(&store, &group.id).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].central_path.contains("guard"));

        let regs = list_group_registrations(&store, &group.id).unwrap();
        assert_eq!(regs.len(), 1);
        assert_eq!(regs[0].event, "PreToolUse");

        remove_group_registration(&store, &group.id, "PreToolUse", "node guard.js").unwrap();
        remove_group_file(&store, &group.id, "guard.js").unwrap();
        assert!(list_group_files(&store, &group.id).unwrap().is_empty());
        delete_group(&store, &group.id).unwrap();
        assert!(get_group(&store, &group.id).unwrap().is_none());
        central_repo::set_test_base_dir_override(None);
    }
}
