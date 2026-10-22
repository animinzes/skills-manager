use anyhow::{bail, Context, Result};
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::core::audit_log::AuditDraft;
use crate::core::central_repo;
use crate::core::skill_store::SkillStore;

/// One managed slash-command / prompt file (`.md`). Same file-granularity
/// model as subagents: the central copy lives in `<central>/commands/`, and
/// deployments are recorded per tool in `slash_command_targets`.
#[derive(Debug, Clone, Serialize)]
pub struct SlashCommandRecord {
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
pub struct SlashCommandTarget {
    pub tool: String,
    pub target_path: String,
    pub mode: String,
    pub synced_hash: Option<String>,
    pub synced_at: i64,
}

/// A command-capable harness: which directory holds its command / prompt
/// files. Claude Code reads `~/.claude/commands`, Codex reads
/// `~/.codex/prompts`, OpenCode reads `~/.config/opencode/commands`.
pub struct CommandAdapter {
    pub tool_key: &'static str,
    pub display_name: &'static str,
    pub dir: PathBuf,
}

pub fn command_adapters() -> Vec<CommandAdapter> {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    let candidates: Vec<(&'static str, &'static str, PathBuf)> = vec![
        ("claude_code", "Claude Code", home.join(".claude/commands")),
        ("codex", "Codex", home.join(".codex/prompts")),
        ("opencode", "OpenCode", home.join(".config/opencode/commands")),
    ];
    candidates
        .into_iter()
        .map(|(tool_key, display_name, dir)| CommandAdapter {
            tool_key,
            display_name,
            dir,
        })
        .collect()
}

pub fn central_commands_dir() -> PathBuf {
    central_repo::base_dir().join("commands")
}

/// Serializable adapter summary for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct CommandAdapterDto {
    pub tool_key: String,
    pub display_name: String,
    pub dir: String,
    pub installed: bool,
}

pub fn command_adapter_dtos() -> Vec<CommandAdapterDto> {
    command_adapters()
        .into_iter()
        .map(|a| CommandAdapterDto {
            tool_key: a.tool_key.to_string(),
            display_name: a.display_name.to_string(),
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

/// Extract `description` from a command file's YAML frontmatter. The command
/// name is always the file stem — Claude Code derives `/name` from it.
pub fn parse_command(path: &Path) -> Result<(String, Option<String>)> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.trim().is_empty() {
        bail!("command file {} has no usable name", path.display());
    }
    let description = text
        .strip_prefix("---\n")
        .and_then(|rest| rest.split("\n---").next())
        .and_then(|front| frontmatter_value(front, "description"));
    Ok((name.trim().to_string(), description))
}

/// `key: value` on a frontmatter line, quoted or bare.
fn frontmatter_value(front: &str, key: &str) -> Option<String> {
    for line in front.lines() {
        let trimmed = line.trim_start();
        for sep in [": ", ":"] {
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

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<SlashCommandRecord> {
    Ok(SlashCommandRecord {
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

pub fn list_commands(store: &SkillStore) -> Result<Vec<SlashCommandRecord>> {
    let rows = store.with_conn(|conn| {
        let mut stmt =
            conn.prepare(&format!("SELECT {RECORD_COLS} FROM slash_commands ORDER BY name"))?;
        let collected: Vec<SlashCommandRecord> =
            stmt.query_map([], row_to_record)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn get_command(store: &SkillStore, id: &str) -> Result<Option<SlashCommandRecord>> {
    let row = store.with_conn(|conn| {
        let mut stmt =
            conn.prepare(&format!("SELECT {RECORD_COLS} FROM slash_commands WHERE id = ?1"))?;
        let mut rows = stmt.query_map(params![id], row_to_record)?;
        let first = match rows.next() {
            Some(row) => Some(row?),
            None => None,
        };
        Ok(first)
    })?;
    Ok(row)
}

pub fn list_targets(store: &SkillStore, command_id: &str) -> Result<Vec<SlashCommandTarget>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT tool, target_path, mode, synced_hash, synced_at \
             FROM slash_command_targets WHERE command_id = ?1 ORDER BY tool",
        )?;
        let collected: Vec<SlashCommandTarget> = stmt
            .query_map(params![command_id], |row| {
                Ok(SlashCommandTarget {
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

/// Import (copy) a command file from anywhere on disk into the central
/// `commands/` directory and register it. Refuses duplicate names.
pub fn import_command(store: &SkillStore, source: &Path) -> Result<SlashCommandRecord> {
    if !source.is_file() {
        bail!("{} is not a file", source.display());
    }
    let (name, description) = parse_command(source)?;
    let dir = central_commands_dir();
    std::fs::create_dir_all(&dir)?;
    let central_path = dir.join(format!("{name}.md"));

    let exists: bool = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM slash_commands WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
        })
        .unwrap_or(false);
    if exists {
        bail!("a slash command named \"{name}\" is already registered");
    }

    std::fs::copy(source, &central_path)
        .with_context(|| format!("copy {} -> {}", source.display(), central_path.display()))?;
    let hash = file_hash(&central_path)?;
    let now = now_secs();
    let id = new_id();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO slash_commands (id, name, description, format, source_type, \
             source_ref, central_path, content_hash, created_at, updated_at) \
             VALUES (?1, ?2, ?3, 'markdown', 'import', ?4, ?5, ?6, ?7, ?7)",
            params![
                id,
                name,
                description,
                source.to_string_lossy().into_owned(),
                central_path.to_string_lossy().into_owned(),
                hash,
                now
            ],
        )
    })?;
    store.log_audit(AuditDraft::new("slash_command_import").ok());
    Ok(SlashCommandRecord {
        id,
        name,
        description,
        format: "markdown".into(),
        source_type: "import".into(),
        source_ref: Some(source.to_string_lossy().into_owned()),
        central_path: central_path.to_string_lossy().into_owned(),
        content_hash: Some(hash),
        created_at: now,
        updated_at: now,
    })
}

/// Deploy the central copy of a command to one harness. Same ownership rule
/// as skills and subagents: a target that exists, differs from our last
/// sync, and has no record of ours is refused rather than overwritten.
pub fn deploy_command(store: &SkillStore, id: &str, tool: &str) -> Result<()> {
    let record =
        get_command(store, id)?.with_context(|| format!("slash command {id} not found"))?;
    let adapter = command_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no command directory configured"))?;
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
            "INSERT INTO slash_command_targets \
             (command_id, tool, target_path, mode, synced_hash, synced_at) \
             VALUES (?1, ?2, ?3, 'copy', ?4, ?5) \
             ON CONFLICT(command_id, tool) DO UPDATE SET \
             target_path = ?3, synced_hash = ?4, synced_at = ?5",
            params![id, tool, target_path.to_string_lossy().into_owned(), hash, now],
        )
    })?;
    store.log_audit(AuditDraft::new("slash_command_deploy").ok());
    Ok(())
}

/// Remove a deployed copy. Refuses to delete a file that no longer matches
/// what we deployed (the user may have replaced it with their own work).
pub fn undeploy_command(store: &SkillStore, id: &str, tool: &str) -> Result<()> {
    let targets = list_targets(store, id)?;
    let target = targets
        .iter()
        .find(|t| t.tool == tool)
        .with_context(|| format!("slash command {id} is not deployed to {tool}"))?;
    let path = PathBuf::from(&target.target_path);
    if path.exists() {
        let current = file_hash(&path).ok();
        let central = get_command(store, id)?.and_then(|c| c.content_hash);
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
            "DELETE FROM slash_command_targets WHERE command_id = ?1 AND tool = ?2",
            params![id, tool],
        )
    })?;
    store.log_audit(AuditDraft::new("slash_command_undeploy").ok());
    Ok(())
}

/// Delete a command: removes every recorded target that still matches our
/// deployed copy, then the central copy and the record.
pub fn delete_command(store: &SkillStore, id: &str) -> Result<()> {
    for target in list_targets(store, id)? {
        let path = PathBuf::from(&target.target_path);
        if path.exists() {
            let current = file_hash(&path).ok();
            if current.is_none() || current == target.synced_hash {
                std::fs::remove_file(&path).ok();
            }
        }
    }
    let record = get_command(store, id)?;
    store.with_conn(|conn| {
        conn.execute("DELETE FROM slash_commands WHERE id = ?1", params![id])
    })?;
    if let Some(record) = record {
        let central = PathBuf::from(&record.central_path);
        if central.exists() {
            std::fs::remove_file(&central).ok();
        }
    }
    store.log_audit(AuditDraft::new("slash_command_delete").ok());
    Ok(())
}

/// Files present in a harness's command directory that the library does not
/// track yet — the adoption surface.
#[derive(Debug, Serialize)]
pub struct DiscoveredCommand {
    pub tool: String,
    pub display_name: String,
    pub path: String,
    pub file_name: String,
    pub format: String,
    pub already_tracked: bool,
}

pub fn scan_discovered(store: &SkillStore) -> Result<Vec<DiscoveredCommand>> {
    let tracked: std::collections::HashSet<String> = list_commands(store)?
        .into_iter()
        .filter_map(|c| {
            Path::new(&c.central_path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .collect();
    let mut out = Vec::new();
    for adapter in command_adapters() {
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
            if !path.is_file() || ext != "md" {
                continue;
            }
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(DiscoveredCommand {
                tool: adapter.tool_key.to_string(),
                display_name: adapter.display_name.to_string(),
                path: path.to_string_lossy().into_owned(),
                file_name: file_name.clone(),
                format: "markdown".to_string(),
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

    fn write_command(dir: &Path, name: &str, description: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join(format!("{name}.md"));
        std::fs::write(
            &p,
            format!("---\ndescription: \"{description}\"\n---\n\nprompt body\n"),
        )
        .unwrap();
        p
    }

    #[test]
    fn parse_reads_stem_and_frontmatter_description() {
        let tmp = tempdir().unwrap();
        let p = write_command(tmp.path(), "commit", "commits staged work");
        let (name, desc) = parse_command(&p).unwrap();
        assert_eq!(name, "commit");
        assert_eq!(desc.as_deref(), Some("commits staged work"));
    }

    #[test]
    fn parse_survives_without_frontmatter() {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("bare.md");
        std::fs::write(&p, "plain prompt").unwrap();
        let (name, desc) = parse_command(&p).unwrap();
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
        let source = write_command(&tmp.path().join("src"), "review", "reviews code");

        let record = import_command(&store, &source).unwrap();
        assert!(Path::new(&record.central_path).is_file());
        assert_eq!(record.name, "review");

        // Duplicate names are refused.
        assert!(import_command(&store, &source).is_err());

        // Book a target row directly (deploy_command writes to the real
        // harness dir, which unit tests must not do).
        let deployed = tmp.path().join("deployed").join("review.md");
        std::fs::create_dir_all(deployed.parent().unwrap()).unwrap();
        std::fs::copy(&record.central_path, &deployed).unwrap();
        let hash = file_hash(&deployed).unwrap();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO slash_command_targets \
                     (command_id, tool, target_path, mode, synced_hash, synced_at) \
                     VALUES (?1, 'claude_code', ?2, 'copy', ?3, 0)",
                    params![record.id, deployed.to_string_lossy().into_owned(), hash],
                )
            })
            .unwrap();

        undeploy_command(&store, &record.id, "claude_code").unwrap();
        assert!(!deployed.exists());
        assert!(list_targets(&store, &record.id).unwrap().is_empty());

        delete_command(&store, &record.id).unwrap();
        assert!(get_command(&store, &record.id).unwrap().is_none());
        assert!(!Path::new(&record.central_path).exists());
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn undeploy_refuses_when_target_drifted() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let source = write_command(&tmp.path().join("src"), "guard", "guards things");

        let record = import_command(&store, &source).unwrap();
        let deployed = tmp.path().join("drifted").join("guard.md");
        std::fs::create_dir_all(deployed.parent().unwrap()).unwrap();
        std::fs::write(&deployed, "user rewrote this file").unwrap();
        let stale_hash = file_hash(Path::new(&record.central_path)).unwrap();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO slash_command_targets \
                     (command_id, tool, target_path, mode, synced_hash, synced_at) \
                     VALUES (?1, 'claude_code', ?2, 'copy', ?3, 0)",
                    params![record.id, deployed.to_string_lossy().into_owned(), stale_hash],
                )
            })
            .unwrap();

        // The file on disk no longer matches the recorded hash nor the
        // central copy, so removal must be refused and the file preserved.
        assert!(undeploy_command(&store, &record.id, "claude_code").is_err());
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
