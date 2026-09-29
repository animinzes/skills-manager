use anyhow::{bail, Context, Result};
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::core::audit_log::AuditDraft;
use crate::core::central_repo;
use crate::core::skill_store::SkillStore;

/// One managed subagent file (`.md` for Claude Code / OpenCode, `.toml` for
/// Codex). Mirrors the skill model at file granularity: the central copy
/// lives in `<central>/agents/<name>.<ext>`, deployments are recorded per
/// tool in `agent_definition_targets`, and every mutation is audited.
#[derive(Debug, Clone, Serialize)]
pub struct AgentDefinitionRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub format: String,
    pub source_type: String,
    pub source_ref: Option<String>,
    pub central_path: String,
    pub content_hash: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentDefinitionTarget {
    pub tool: String,
    pub target_path: String,
    pub mode: String,
    pub synced_hash: Option<String>,
    pub synced_at: i64,
}

/// A subagent-capable harness: where its definition files live and in what
/// format. Skills adapters stay untouched; this is a parallel, smaller map
/// because only a few harnesses expose file-based subagents today.
pub struct SubagentAdapter {
    pub tool_key: &'static str,
    pub display_name: &'static str,
    /// `markdown` (Claude Code / OpenCode frontmatter) or `toml` (Codex).
    pub format: &'static str,
    pub dir: PathBuf,
}

pub fn subagent_adapters() -> Vec<SubagentAdapter> {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    let candidates: Vec<(&'static str, &'static str, &'static str, PathBuf)> = vec![
        ("claude_code", "Claude Code", "markdown", home.join(".claude/agents")),
        ("codex", "Codex", "toml", home.join(".codex/agents")),
        ("opencode", "OpenCode", "markdown", home.join(".config/opencode/agents")),
    ];
    candidates
        .into_iter()
        .map(|(tool_key, display_name, format, dir)| SubagentAdapter {
            tool_key,
            display_name,
            format,
            dir,
        })
        .collect()
}

pub fn central_agents_dir() -> PathBuf {
    central_repo::base_dir().join("agents")
}

/// Serializable adapter summary for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct SubagentAdapterDto {
    pub tool_key: String,
    pub display_name: String,
    pub format: String,
    pub dir: String,
    pub installed: bool,
}

pub fn subagent_adapter_dtos() -> Vec<SubagentAdapterDto> {
    subagent_adapters()
        .into_iter()
        .map(|a| SubagentAdapterDto {
            tool_key: a.tool_key.to_string(),
            display_name: a.display_name.to_string(),
            format: a.format.to_string(),
            dir: a.dir.to_string_lossy().into_owned(),
            installed: a.dir.exists(),
        })
        .collect()
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

/// Extract `name` and `description` from a definition file. Markdown files
/// carry YAML frontmatter; Codex TOML files carry top-level keys.
pub fn parse_definition(path: &Path, format: &str) -> Result<(String, Option<String>)> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let (name, description) = if format == "toml" {
        let name = key_value(&text, "name").unwrap_or_else(|| stem_of(path));
        (name, key_value(&text, "description"))
    } else if let Some(rest) = text.strip_prefix("---\n") {
        let front = rest.split("\n---").next().unwrap_or("");
        let name = yaml_key(front, "name").unwrap_or_else(|| stem_of(path));
        (name, yaml_key(front, "description"))
    } else {
        (stem_of(path), None)
    };
    if name.trim().is_empty() {
        bail!("definition file {} has no usable name", path.display());
    }
    Ok((name.trim().to_string(), description))
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// `name = "value"` (TOML) or `name: value` (frontmatter lines) — one small
/// scanner for both, enough for name/description extraction.
fn key_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim_start();
        for sep in [": ", " = ", ":", "="] {
            let prefix = format!("{key}{sep}");
            if let Some(rest) = trimmed.strip_prefix(&prefix) {
                let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

fn yaml_key(front: &str, key: &str) -> Option<String> {
    key_value(front, key)
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AgentDefinitionRecord> {
    Ok(AgentDefinitionRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        format: row.get(3)?,
        source_type: row.get(4)?,
        source_ref: row.get(5)?,
        central_path: row.get(6)?,
        content_hash: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const RECORD_COLS: &str = "id, name, description, format, source_type, source_ref, \
                           central_path, content_hash, created_at, updated_at";

pub fn list_definitions(store: &SkillStore) -> Result<Vec<AgentDefinitionRecord>> {
    let rows = store.with_conn(|conn| {
        let mut stmt =
            conn.prepare(&format!("SELECT {RECORD_COLS} FROM agent_definitions ORDER BY name"))?;
        let collected: Vec<AgentDefinitionRecord> =
            stmt.query_map([], row_to_record)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn get_definition(store: &SkillStore, id: &str) -> Result<Option<AgentDefinitionRecord>> {
    let row = store.with_conn(|conn| {
        let mut stmt = conn
            .prepare(&format!("SELECT {RECORD_COLS} FROM agent_definitions WHERE id = ?1"))?;
        let mut rows = stmt.query_map(params![id], row_to_record)?;
        let first = match rows.next() {
            Some(row) => Some(row?),
            None => None,
        };
        Ok(first)
    })?;
    Ok(row)
}

pub fn list_targets(store: &SkillStore, definition_id: &str) -> Result<Vec<AgentDefinitionTarget>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT tool, target_path, mode, synced_hash, synced_at \
             FROM agent_definition_targets WHERE definition_id = ?1 ORDER BY tool",
        )?;
        let collected: Vec<AgentDefinitionTarget> = stmt
            .query_map(params![definition_id], |row| {
                Ok(AgentDefinitionTarget {
                    tool: row.get(0)?,
                    target_path: row.get(1)?,
                    mode: row.get(2)?,
                    synced_hash: row.get(3)?,
                    synced_at: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

/// Import (copy) a definition file from anywhere on disk into the central
/// `agents/` directory and register it. Refuses duplicate names.
pub fn import_definition(
    store: &SkillStore,
    source: &Path,
    format: &str,
) -> Result<AgentDefinitionRecord> {
    if !source.is_file() {
        bail!("{} is not a file", source.display());
    }
    let (name, description) = parse_definition(source, format)?;
    let extension = if format == "toml" { "toml" } else { "md" };
    let dir = central_agents_dir();
    std::fs::create_dir_all(&dir)?;
    let central_path = dir.join(format!("{name}.{extension}"));

    let exists: bool = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM agent_definitions WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
        })
        .unwrap_or(false);
    if exists {
        bail!("a subagent named \"{name}\" is already registered");
    }

    std::fs::copy(source, &central_path)
        .with_context(|| format!("copy {} -> {}", source.display(), central_path.display()))?;
    let hash = file_hash(&central_path)?;
    let now = now_secs();
    let id = new_id();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO agent_definitions (id, name, description, format, source_type, \
             source_ref, central_path, content_hash, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 'import', ?5, ?6, ?7, ?8, ?8)",
            params![
                id,
                name,
                description,
                format,
                source.to_string_lossy().into_owned(),
                central_path.to_string_lossy().into_owned(),
                hash,
                now
            ],
        )
    })?;
    store.log_audit(AuditDraft::new("agent_definition_import").ok());
    Ok(AgentDefinitionRecord {
        id,
        name,
        description,
        format: format.to_string(),
        source_type: "import".into(),
        source_ref: Some(source.to_string_lossy().into_owned()),
        central_path: central_path.to_string_lossy().into_owned(),
        content_hash: Some(hash),
        created_at: now,
        updated_at: now,
    })
}

/// Deploy the central copy of a definition to one harness. Same ownership
/// rule as skills: a target that exists, differs from our last sync, and has
/// no record of ours is refused rather than overwritten.
pub fn deploy_definition(store: &SkillStore, id: &str, tool: &str) -> Result<()> {
    let record =
        get_definition(store, id)?.with_context(|| format!("subagent definition {id} not found"))?;
    let adapter = subagent_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no subagent directory configured"))?;
    std::fs::create_dir_all(&adapter.dir)?;
    let target_path = adapter
        .dir
        .join(Path::new(&record.central_path).file_name().unwrap());

    if target_path.exists() {
        let recorded = list_targets(store, id)?
            .into_iter()
            .find(|t| t.tool == tool);
        let ours = matches!(&recorded,
            Some(t) if t.target_path == target_path.to_string_lossy());
        if !ours {
            let target_hash = file_hash(&target_path).ok();
            if target_hash != record.content_hash {
                bail!(
                    "{} already exists and differs from the central copy; \
                     import it under a new name or remove it first",
                    target_path.display()
                );
            }
        }
    }

    std::fs::copy(&record.central_path, &target_path)?;
    let hash = file_hash(&target_path)?;
    let now = now_secs();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO agent_definition_targets \
             (definition_id, tool, target_path, mode, synced_hash, synced_at) \
             VALUES (?1, ?2, ?3, 'copy', ?4, ?5) \
             ON CONFLICT(definition_id, tool) DO UPDATE SET \
             target_path = ?3, synced_hash = ?4, synced_at = ?5",
            params![id, tool, target_path.to_string_lossy().into_owned(), hash, now],
        )
    })?;
    store.log_audit(AuditDraft::new("agent_definition_deploy").ok());
    Ok(())
}

/// Remove a deployed copy. Refuses to delete a file that no longer matches
/// what we deployed (the user may have replaced it with their own work).
pub fn undeploy_definition(store: &SkillStore, id: &str, tool: &str) -> Result<()> {
    let targets = list_targets(store, id)?;
    let target = targets
        .iter()
        .find(|t| t.tool == tool)
        .with_context(|| format!("subagent definition {id} is not deployed to {tool}"))?;
    let path = PathBuf::from(&target.target_path);
    if path.exists() {
        let current = file_hash(&path).ok();
        let central = get_definition(store, id)?.and_then(|d| d.content_hash);
        let matches_record = current.is_some() && current == target.synced_hash;
        let matches_central = current.is_some() && current == central;
        if !matches_record && !matches_central {
            bail!(
                "{} no longer matches the deployed copy; remove it manually",
                path.display()
            );
        }
        std::fs::remove_file(&path)?;
    }
    store.with_conn(|conn| {
        conn.execute(
            "DELETE FROM agent_definition_targets WHERE definition_id = ?1 AND tool = ?2",
            params![id, tool],
        )
    })?;
    store.log_audit(AuditDraft::new("agent_definition_undeploy").ok());
    Ok(())
}

/// Delete a definition: removes every recorded target that still matches our
/// deployed copy, then the central copy and the record.
pub fn delete_definition(store: &SkillStore, id: &str) -> Result<()> {
    for target in list_targets(store, id)? {
        let path = PathBuf::from(&target.target_path);
        if path.exists() {
            let current = file_hash(&path).ok();
            if current.is_none() || current == target.synced_hash {
                std::fs::remove_file(&path).ok();
            }
        }
    }
    let record = get_definition(store, id)?;
    store.with_conn(|conn| {
        conn.execute("DELETE FROM agent_definitions WHERE id = ?1", params![id])
    })?;
    if let Some(record) = record {
        let central = PathBuf::from(&record.central_path);
        if central.exists() {
            std::fs::remove_file(&central).ok();
        }
    }
    store.log_audit(AuditDraft::new("agent_definition_delete").ok());
    Ok(())
}

/// Files present in a harness's subagent directory that the library does not
/// track yet — the adoption surface.
#[derive(Debug, Serialize)]
pub struct DiscoveredDefinition {
    pub tool: String,
    pub display_name: String,
    pub path: String,
    pub file_name: String,
    pub format: String,
    pub already_tracked: bool,
}

pub fn scan_discovered(store: &SkillStore) -> Result<Vec<DiscoveredDefinition>> {
    let tracked: std::collections::HashSet<String> = list_definitions(store)?
        .into_iter()
        .filter_map(|d| {
            Path::new(&d.central_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .collect();
    let mut out = Vec::new();
    for adapter in subagent_adapters() {
        if !adapter.dir.exists() {
            continue;
        }
        let entries = std::fs::read_dir(&adapter.dir)?;
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default();
            let expected = if adapter.format == "toml" { "toml" } else { "md" };
            if !path.is_file() || ext != expected {
                continue;
            }
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(DiscoveredDefinition {
                tool: adapter.tool_key.to_string(),
                display_name: adapter.display_name.to_string(),
                path: path.to_string_lossy().into_owned(),
                file_name: file_name.clone(),
                format: adapter.format.to_string(),
                already_tracked: tracked.contains(&file_name),
            });
        }
    }
    out.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in(tmp: &Path) -> SkillStore {
        SkillStore::new(&tmp.join("test.db")).unwrap()
    }

    fn write_md(dir: &Path, name: &str, description: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join(format!("{name}.md"));
        std::fs::write(
            &p,
            format!("---\nname: {name}\ndescription: \"{description}\"\n---\n\nsystem prompt\n"),
        )
        .unwrap();
        p
    }

    #[test]
    fn parse_reads_frontmatter_name_and_description() {
        let tmp = tempdir().unwrap();
        let p = write_md(tmp.path(), "reviewer", "reviews code");
        let (name, desc) = parse_definition(&p, "markdown").unwrap();
        assert_eq!(name, "reviewer");
        assert_eq!(desc.as_deref(), Some("reviews code"));
    }

    #[test]
    fn parse_reads_toml_keys() {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("builder.toml");
        std::fs::write(&p, "name = \"builder\"\ndescription = \"builds things\"\n").unwrap();
        let (name, desc) = parse_definition(&p, "toml").unwrap();
        assert_eq!(name, "builder");
        assert_eq!(desc.as_deref(), Some("builds things"));
    }

    #[test]
    fn parse_falls_back_to_file_stem_without_frontmatter() {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("bare.md");
        std::fs::write(&p, "no frontmatter here").unwrap();
        let (name, desc) = parse_definition(&p, "markdown").unwrap();
        assert_eq!(name, "bare");
        assert_eq!(desc, None);
    }

    /// Round-trips the data path without touching any real harness directory:
    /// import into a temp central repo, book a target row pointing at a temp
    /// file, undeploy removes it, delete cleans everything.
    #[test]
    fn import_target_undeploy_delete_roundtrip() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let source = write_md(&tmp.path().join("src"), "tester", "tests things");

        let record = import_definition(&store, &source, "markdown").unwrap();
        assert!(Path::new(&record.central_path).is_file());
        assert_eq!(record.name, "tester");

        // Duplicate names are refused.
        assert!(import_definition(&store, &source, "markdown").is_err());

        // Book a target row directly (deploy_definition writes to the real
        // harness dir, which unit tests must not do).
        let deployed = tmp.path().join("deployed").join("tester.md");
        std::fs::create_dir_all(deployed.parent().unwrap()).unwrap();
        std::fs::copy(&record.central_path, &deployed).unwrap();
        let hash = file_hash(&deployed).unwrap();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO agent_definition_targets \
                     (definition_id, tool, target_path, mode, synced_hash, synced_at) \
                     VALUES (?1, 'claude_code', ?2, 'copy', ?3, 0)",
                    params![record.id, deployed.to_string_lossy().into_owned(), hash],
                )
            })
            .unwrap();

        undeploy_definition(&store, &record.id, "claude_code").unwrap();
        assert!(!deployed.exists());
        assert!(list_targets(&store, &record.id).unwrap().is_empty());

        delete_definition(&store, &record.id).unwrap();
        assert!(get_definition(&store, &record.id).unwrap().is_none());
        assert!(!Path::new(&record.central_path).exists());
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn undeploy_refuses_when_target_drifted() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let source = write_md(&tmp.path().join("src"), "guard", "guards things");

        let record = import_definition(&store, &source, "markdown").unwrap();
        let deployed = tmp.path().join("drifted").join("guard.md");
        std::fs::create_dir_all(deployed.parent().unwrap()).unwrap();
        std::fs::write(&deployed, "user rewrote this file").unwrap();
        let stale_hash = file_hash(Path::new(&record.central_path)).unwrap();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO agent_definition_targets \
                     (definition_id, tool, target_path, mode, synced_hash, synced_at) \
                     VALUES (?1, 'claude_code', ?2, 'copy', ?3, 0)",
                    params![record.id, deployed.to_string_lossy().into_owned(), stale_hash],
                )
            })
            .unwrap();

        // The file on disk no longer matches the recorded hash nor the
        // central copy, so removal must be refused and the file preserved.
        assert!(undeploy_definition(&store, &record.id, "claude_code").is_err());
        assert!(deployed.exists());
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn scan_reports_well_formed_rows() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        for row in scan_discovered(&store).unwrap() {
            assert!(!row.tool.is_empty());
            assert!(row.path.ends_with(&row.file_name));
        }
        central_repo::set_test_base_dir_override(None);
    }
}
