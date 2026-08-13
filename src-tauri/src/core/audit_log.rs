//! Append-only audit log of user/system actions.
//!
//! Every audited modification is appended to
//! `<central-repository>/history/changes.jsonl`. This file is never pruned or
//! rewritten. SQLite table `audit_log` is a capped read cache for the UI.
//! Writes are best-effort so a history I/O error never rolls back the user
//! action it accompanies. Reads return newest-first.

use anyhow::Result;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

/// Hard cap on the SQLite read cache. The JSONL history keeps every row.
pub const MAX_ENTRIES: i64 = 10_000;

/// One audit log entry as exposed to the frontend / exports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: i64,
    /// Unix timestamp in seconds.
    pub ts: i64,
    /// Short action verb, e.g. "install", "remove", "enable", "sync".
    pub action: String,
    pub skill_id: Option<String>,
    pub skill_name: Option<String>,
    /// Affected tool/agent key when the action targets one, e.g. "claude_code".
    pub tool: Option<String>,
    pub success: bool,
    /// Free-form detail. Error message on failure, optional context otherwise.
    pub detail: Option<String>,
}

pub fn append_persistent(entry: &AuditEntry) -> Result<()> {
    append_to_path(&history_path(), entry)
}

pub(crate) fn history_path() -> std::path::PathBuf {
    super::central_repo::base_dir()
        .join("history")
        .join("changes.jsonl")
}

fn append_to_path(path: &std::path::Path, entry: &AuditEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let lock_path = path.with_extension("lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, entry)?;
    file.write_all(b"\n")?;
    file.sync_data()?;
    FileExt::unlock(&lock)?;
    Ok(())
}

pub fn read_persistent(limit: Option<usize>) -> Result<Vec<AuditEntry>> {
    read_from_path(&history_path(), limit)
}

fn read_from_path(path: &std::path::Path, limit: Option<usize>) -> Result<Vec<AuditEntry>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let mut entries = BufReader::new(file)
        .lines()
        .map_while(|line| line.ok())
        .filter_map(|line| serde_json::from_str::<AuditEntry>(&line).ok())
        .collect::<Vec<_>>();
    entries.reverse();
    if let Some(limit) = limit {
        entries.truncate(limit);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn persistent_history_appends_without_overwriting() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("changes.jsonl");
        for id in 1..=2 {
            append_to_path(
                &path,
                &AuditEntry {
                    id,
                    ts: id,
                    action: "save_suite".into(),
                    skill_id: None,
                    skill_name: None,
                    tool: None,
                    success: true,
                    detail: Some(format!("version {id}")),
                },
            )
            .unwrap();
        }
        let entries = read_from_path(&path, None).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].detail.as_deref(), Some("version 2"));
        assert_eq!(entries[1].detail.as_deref(), Some("version 1"));
    }
}

/// Payload used when recording a new entry.
#[derive(Debug, Default, Clone)]
pub struct AuditDraft {
    pub action: String,
    pub skill_id: Option<String>,
    pub skill_name: Option<String>,
    pub tool: Option<String>,
    pub success: bool,
    pub detail: Option<String>,
}

impl AuditDraft {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            ..Default::default()
        }
    }

    pub fn ok(mut self) -> Self {
        self.success = true;
        self
    }

    pub fn fail(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.detail = Some(error.into());
        self
    }

    pub fn skill(mut self, id: impl Into<String>, name: impl Into<String>) -> Self {
        self.skill_id = Some(id.into());
        self.skill_name = Some(name.into());
        self
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}
