use anyhow::{bail, Context, Result};
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::core::audit_log::AuditDraft;
use crate::core::central_repo;
use crate::core::skill_store::SkillStore;

/// A registered agent memory / instructions file (CLAUDE.md, AGENTS.md,
/// GEMINI.md). These files are registered and snapshotted, never written
/// back: there is deliberately no deploy function. The central snapshot
/// lives in `<central>/memories/<tool>/<file_name>`.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryFileRecord {
    pub id: String,
    pub tool: String,
    pub file_name: String,
    pub source_path: String,
    pub central_path: String,
    pub content_hash: Option<String>,
    pub byte_size: i64,
    pub snapshotted_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Where each harness keeps its global memory file. ZCode reads AGENTS.md
/// from `~/.zcode` (verified on this machine; the requirements table's "—"
/// predates that finding).
pub struct MemoryAdapter {
    pub tool_key: &'static str,
    pub display_name: &'static str,
    pub file_name: &'static str,
    pub path: PathBuf,
}

pub fn memory_adapters() -> Vec<MemoryAdapter> {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    let candidates: Vec<(&'static str, &'static str, &'static str, PathBuf)> = vec![
        ("claude_code", "Claude Code", "CLAUDE.md", home.join(".claude/CLAUDE.md")),
        ("codex", "Codex", "AGENTS.md", home.join(".codex/AGENTS.md")),
        ("zcode", "ZCode", "AGENTS.md", home.join(".zcode/AGENTS.md")),
        ("gemini_cli", "AGY (Gemini)", "GEMINI.md", home.join(".gemini/GEMINI.md")),
        ("opencode", "OpenCode", "AGENTS.md", home.join(".config/opencode/AGENTS.md")),
    ];
    candidates
        .into_iter()
        .map(|(tool_key, display_name, file_name, path)| MemoryAdapter {
            tool_key,
            display_name,
            file_name,
            path,
        })
        .collect()
}

pub fn central_memories_dir() -> PathBuf {
    central_repo::base_dir().join("memories")
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

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryFileRecord> {
    Ok(MemoryFileRecord {
        id: row.get(0)?,
        tool: row.get(1)?,
        file_name: row.get(2)?,
        source_path: row.get(3)?,
        central_path: row.get(4)?,
        content_hash: row.get(5)?,
        byte_size: row.get(6)?,
        snapshotted_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const RECORD_COLS: &str = "id, tool, file_name, source_path, central_path, content_hash, \
                           byte_size, snapshotted_at, created_at, updated_at";

pub fn list_memory_files(store: &SkillStore) -> Result<Vec<MemoryFileRecord>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn
            .prepare(&format!("SELECT {RECORD_COLS} FROM memory_files ORDER BY tool"))?;
        let collected: Vec<MemoryFileRecord> =
            stmt.query_map([], row_to_record)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn get_memory_file(store: &SkillStore, id: &str) -> Result<Option<MemoryFileRecord>> {
    let row = store.with_conn(|conn| {
        let mut stmt =
            conn.prepare(&format!("SELECT {RECORD_COLS} FROM memory_files WHERE id = ?1"))?;
        let mut rows = stmt.query_map(params![id], row_to_record)?;
        let first = match rows.next() {
            Some(row) => Some(row?),
            None => None,
        };
        Ok(first)
    })?;
    Ok(row)
}

fn snapshot_at(
    tool: &str,
    file_name: &str,
    source: &Path,
) -> Result<(PathBuf, String, i64, i64)> {
    if !source.is_file() {
        bail!("{} does not exist", source.display());
    }
    let dir = central_memories_dir().join(tool);
    std::fs::create_dir_all(&dir)?;
    let central_path = dir.join(file_name);
    std::fs::copy(source, &central_path)
        .with_context(|| format!("snapshot {} -> {}", source.display(), central_path.display()))?;
    let hash = file_hash(&central_path)?;
    let size = std::fs::metadata(&central_path).map(|m| m.len() as i64).unwrap_or(0);
    let now = now_secs();
    Ok((central_path, hash, size, now))
}

/// Register one harness's memory file: copy the current content into the
/// central `memories/<tool>/` snapshot area and record it. One registration
/// per tool and file name.
pub fn register_memory_file(store: &SkillStore, tool: &str) -> Result<MemoryFileRecord> {
    let adapter = memory_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no memory file location configured"))?;
    register_memory_file_at(store, tool, adapter.file_name, &adapter.path)
}

/// Snapshot core with an explicit source path, so tests never read real
/// home-directory files.
fn register_memory_file_at(
    store: &SkillStore,
    tool: &str,
    file_name: &str,
    source: &Path,
) -> Result<MemoryFileRecord> {
    let exists: bool = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM memory_files WHERE tool = ?1 AND file_name = ?2)",
                params![tool, file_name],
                |row| row.get(0),
            )
        })
        .unwrap_or(false);
    if exists {
        bail!(
            "{} {} is already registered (refresh its snapshot instead)",
            tool,
            file_name
        );
    }

    let (central_path, hash, size, now) = snapshot_at(tool, file_name, source)?;
    let id = new_id();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO memory_files (id, tool, file_name, source_path, central_path, \
             content_hash, byte_size, snapshotted_at, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8, ?8)",
            params![
                id,
                tool,
                file_name,
                source.to_string_lossy().into_owned(),
                central_path.to_string_lossy().into_owned(),
                hash,
                size,
                now
            ],
        )
    })?;
    store.log_audit(AuditDraft::new("memory_file_register").ok());
    Ok(MemoryFileRecord {
        id,
        tool: tool.to_string(),
        file_name: file_name.to_string(),
        source_path: source.to_string_lossy().into_owned(),
        central_path: central_path.to_string_lossy().into_owned(),
        content_hash: Some(hash),
        byte_size: size,
        snapshotted_at: now,
        created_at: now,
        updated_at: now,
    })
}

/// Re-snapshot a registered memory file: overwrite the central backup with
/// the current content at the recorded source path. Never writes to the
/// source.
pub fn refresh_memory_file(store: &SkillStore, id: &str) -> Result<MemoryFileRecord> {
    let record = get_memory_file(store, id)?
        .with_context(|| format!("memory file registration {id} not found"))?;

    let (central_path, hash, size, now) =
        snapshot_at(&record.tool, &record.file_name, Path::new(&record.source_path))?;
    store.with_conn(|conn| {
        conn.execute(
            "UPDATE memory_files SET central_path = ?2, content_hash = ?3, byte_size = ?4, \
             snapshotted_at = ?5, updated_at = ?5 WHERE id = ?1",
            params![id, central_path.to_string_lossy().into_owned(), hash, size, now],
        )
    })?;
    store.log_audit(AuditDraft::new("memory_file_refresh").ok());
    Ok(MemoryFileRecord {
        central_path: central_path.to_string_lossy().into_owned(),
        content_hash: Some(hash),
        byte_size: size,
        snapshotted_at: now,
        updated_at: now,
        ..record
    })
}

/// Remove a registration and its central snapshot. The agent's live memory
/// file is never touched.
pub fn delete_memory_file(store: &SkillStore, id: &str) -> Result<()> {
    let record = get_memory_file(store, id)?;
    store.with_conn(|conn| {
        conn.execute("DELETE FROM memory_files WHERE id = ?1", params![id])
    })?;
    if let Some(record) = record {
        let central = PathBuf::from(&record.central_path);
        if central.exists() {
            std::fs::remove_file(&central).ok();
        }
    }
    store.log_audit(AuditDraft::new("memory_file_delete").ok());
    Ok(())
}

/// One row per adapter for the UI: whether the file exists on disk, whether
/// it is registered, and whether the source changed since the last snapshot.
#[derive(Debug, Serialize)]
pub struct MemorySlotDto {
    pub tool_key: String,
    pub display_name: String,
    pub file_name: String,
    pub source_path: String,
    pub exists_on_disk: bool,
    pub registered: bool,
    /// Some(true) when registered and the live file is readable and differs
    /// from the snapshot; Some(false) when it matches (or is gone — the
    /// exists_on_disk flag owns that signal); None when not registered.
    pub source_changed: Option<bool>,
    pub record: Option<MemoryFileRecord>,
}

pub fn memory_slots(store: &SkillStore) -> Result<Vec<MemorySlotDto>> {
    let records: Vec<MemoryFileRecord> = list_memory_files(store)?;
    Ok(memory_adapters()
        .into_iter()
        .map(|a| {
            let record = records.iter().find(|r| r.tool == a.tool_key).cloned();
            // Compare against the recorded source (what refresh re-snapshots),
            // so the badge and the refresh action can never disagree. A source
            // we cannot read but that is still present counts as changed; a
            // missing one is reported through exists_on_disk instead.
            let source_changed = record.as_ref().map(|r| {
                match file_hash(Path::new(&r.source_path)) {
                    Ok(h) => Some(h) != r.content_hash,
                    Err(_) => !Path::new(&r.source_path).is_file(),
                }
            });
            MemorySlotDto {
                tool_key: a.tool_key.to_string(),
                display_name: a.display_name.to_string(),
                file_name: a.file_name.to_string(),
                source_path: a.path.to_string_lossy().into_owned(),
                exists_on_disk: a.path.is_file(),
                registered: record.is_some(),
                source_changed,
                record,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in(tmp: &Path) -> SkillStore {
        SkillStore::new(&tmp.join("test.db")).unwrap()
    }

    /// Round-trips the data path without touching any real home file: the
    /// explicit-source core registers a temp file, refresh picks up an edit,
    /// delete removes the registration and the central snapshot while the
    /// source stays untouched.
    #[test]
    fn register_refresh_delete_roundtrip() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let source = tmp.path().join("CLAUDE.md");
        std::fs::write(&source, "first version\n").unwrap();

        let record =
            register_memory_file_at(&store, "claude_code", "CLAUDE.md", &source).unwrap();
        assert!(Path::new(&record.central_path).is_file());
        assert_eq!(record.byte_size, "first version\n".len() as i64);

        // Duplicate registration for the same tool + file is refused.
        assert!(
            register_memory_file_at(&store, "claude_code", "CLAUDE.md", &source).is_err()
        );

        // The agent keeps editing its memory; refresh re-snapshots it.
        std::fs::write(&source, "second version with more rules\n").unwrap();
        let refreshed = refresh_memory_file(&store, &record.id).unwrap();
        assert!(refreshed.snapshotted_at >= record.snapshotted_at);
        let snapshot_text =
            std::fs::read_to_string(Path::new(&refreshed.central_path)).unwrap();
        assert_eq!(snapshot_text, "second version with more rules\n");
        assert_eq!(refreshed.source_path, source.to_string_lossy());

        // Deleting the registration never touches the live source file.
        delete_memory_file(&store, &record.id).unwrap();
        assert!(get_memory_file(&store, &record.id).unwrap().is_none());
        assert!(!Path::new(&refreshed.central_path).exists());
        assert!(source.is_file());
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn register_refuses_missing_source() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        assert!(
            register_memory_file_at(&store, "gemini_cli", "GEMINI.md", &tmp.path().join("nope.md"))
                .is_err()
        );
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn refresh_refuses_when_source_disappeared() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let source = tmp.path().join("AGENTS.md");
        std::fs::write(&source, "v1").unwrap();
        let record =
            register_memory_file_at(&store, "codex", "AGENTS.md", &source).unwrap();
        std::fs::remove_file(&source).unwrap();
        assert!(refresh_memory_file(&store, &record.id).is_err());
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn slots_report_shape_for_all_adapters() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let slots = memory_slots(&store).unwrap();
        assert_eq!(slots.len(), memory_adapters().len());
        for slot in &slots {
            assert!(!slot.tool_key.is_empty());
            assert!(!slot.file_name.is_empty());
            assert_eq!(slot.registered, slot.record.is_some());
            if !slot.exists_on_disk {
                assert_eq!(slot.source_changed, None);
            }
        }
        central_repo::set_test_base_dir_override(None);
    }
}
