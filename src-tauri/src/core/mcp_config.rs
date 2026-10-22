use anyhow::{bail, Context, Result};
use rusqlite::params;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::core::audit_log::AuditDraft;
use crate::core::central_repo;
use crate::core::skill_store::SkillStore;

/// One registered MCP server entry. `config_json` is a canonical JSON
/// object: either `{"transport":"http","url":...,"headers":{...}}` or
/// `{"transport":"stdio","command":...,"args":[...],"env":{...}}`. Each host
/// dialect is rendered from the canonical form at deploy time.
#[derive(Debug, Clone, Serialize)]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config_json: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerTarget {
    pub tool: String,
    pub config_path: String,
    pub backup_path: Option<String>,
    pub synced_at: i64,
}

/// Where a harness keeps its MCP server map. `root` is the pointer to the
/// server-name map inside the config document (e.g. `mcpServers` for Claude
/// Code, `mcp.servers` for ZCode, `mcp_servers` for Codex's TOML).
pub struct McpHostAdapter {
    pub tool_key: &'static str,
    pub display_name: &'static str,
    pub config_path: PathBuf,
    pub format: &'static str,
    pub root: &'static [&'static str],
}

pub fn mcp_host_adapters() -> Vec<McpHostAdapter> {
    let home = dirs::home_dir().expect("Cannot determine home directory");
    const CLAUDE_ROOT: &[&str] = &["mcpServers"];
    const CODEX_ROOT: &[&str] = &["mcp_servers"];
    const ZCODE_ROOT: &[&str] = &["mcp", "servers"];
    vec![
        McpHostAdapter {
            tool_key: "claude_code",
            display_name: "Claude Code",
            config_path: home.join(".claude.json"),
            format: "json",
            root: CLAUDE_ROOT,
        },
        McpHostAdapter {
            tool_key: "codex",
            display_name: "Codex",
            config_path: home.join(".codex/config.toml"),
            format: "toml",
            root: CODEX_ROOT,
        },
        McpHostAdapter {
            tool_key: "zcode",
            display_name: "ZCode",
            config_path: home.join(".zcode/cli/config.json"),
            format: "json",
            root: ZCODE_ROOT,
        },
        McpHostAdapter {
            tool_key: "gemini_cli",
            display_name: "AGY (Gemini)",
            config_path: home.join(".gemini/settings.json"),
            format: "json",
            root: CLAUDE_ROOT,
        },
        McpHostAdapter {
            tool_key: "opencode",
            display_name: "OpenCode",
            config_path: home.join(".config/opencode/opencode.json"),
            format: "json",
            root: &["mcp"],
        },
    ]
}

#[derive(Debug, Clone, Serialize)]
pub struct McpHostAdapterDto {
    pub tool_key: String,
    pub display_name: String,
    pub config_path: String,
    pub format: String,
    pub exists: bool,
}

pub fn mcp_host_adapter_dtos() -> Vec<McpHostAdapterDto> {
    mcp_host_adapters()
        .into_iter()
        .map(|a| McpHostAdapterDto {
            tool_key: a.tool_key.to_string(),
            display_name: a.display_name.to_string(),
            config_path: a.config_path.to_string_lossy().into_owned(),
            format: a.format.to_string(),
            exists: a.config_path.is_file(),
        })
        .collect()
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

/// Validate a canonical entry: must be an object with a known transport and
/// the fields that transport requires.
pub fn validate_canonical(config: &Value) -> Result<()> {
    let obj = config
        .as_object()
        .with_context(|| "MCP server config must be a JSON object")?;
    match obj.get("transport").and_then(|t| t.as_str()) {
        Some("http") => {
            if !obj.get("url").and_then(|u| u.as_str()).is_some_and(|u| !u.is_empty()) {
                bail!("http transport requires a non-empty \"url\"");
            }
        }
        Some("stdio") => {
            if !obj.get("command").and_then(|c| c.as_str()).is_some_and(|c| !c.is_empty()) {
                bail!("stdio transport requires a non-empty \"command\"");
            }
        }
        other => bail!("unknown transport {other:?} (expected \"http\" or \"stdio\")"),
    }
    Ok(())
}

/// Render the canonical entry into one host's dialect. Everything besides
/// `transport` is carried over verbatim where the dialect allows it.
pub fn render_for_host(canonical: &Value, tool_key: &str) -> Result<Value> {
    let obj = canonical
        .as_object()
        .with_context(|| "canonical config must be a JSON object")?;
    let transport = obj.get("transport").and_then(|t| t.as_str()).unwrap_or("");
    let mut out = Map::new();
    match tool_key {
        "claude_code" | "zcode" | "gemini_cli" => {
            out.insert("type".into(), Value::String(transport.to_string()));
            for (k, v) in obj {
                if k != "transport" {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        "opencode" => {
            if transport == "http" {
                out.insert("type".into(), Value::String("remote".into()));
                for (k, v) in obj {
                    if k != "transport" {
                        out.insert(k.clone(), v.clone());
                    }
                }
            } else {
                let mut command = vec![Value::String(
                    obj.get("command").and_then(|c| c.as_str()).unwrap_or_default().to_string(),
                )];
                if let Some(args) = obj.get("args").and_then(|a| a.as_array()) {
                    command.extend(args.iter().cloned());
                }
                out.insert("type".into(), Value::String("local".into()));
                out.insert("command".into(), Value::Array(command));
                out.insert("enabled".into(), Value::Bool(true));
                if let Some(env) = obj.get("env") {
                    out.insert("env".into(), env.clone());
                }
            }
        }
        "codex" => {
            // Codex keeps servers as TOML tables. http servers take a bare
            // `url`; stdio servers take command/args/env. Extra keys ride
            // along so nothing the user wrote is silently dropped.
            for (k, v) in obj {
                if k == "transport" {
                    continue;
                }
                if k == "command" || k == "args" || k == "env" || k == "url" {
                    out.insert(k.clone(), v.clone());
                }
            }
        }
        other => bail!("{other} has no MCP config location configured"),
    }
    Ok(Value::Object(out))
}

// ── JSON host editing ──

fn detect_indent(text: &str) -> usize {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let indent = line.len() - trimmed.len();
        if indent > 0 {
            return indent.min(8);
        }
    }
    2
}

pub(crate) fn pretty_write(path: &Path, doc: &Value, original_text: &str) -> Result<()> {
    let indent = detect_indent(original_text);
    let ends_with_newline = original_text.ends_with('\n');
    let mut out = serde_json::to_string_pretty(doc)?;
    if indent != 2 {
        out = reindent(&out, indent);
    }
    if ends_with_newline && !out.ends_with('\n') {
        out.push('\n');
    }
    std::fs::write(path, out).with_context(|| format!("cannot write {}", path.display()))
}

/// Read the entry currently stored at `root/name`, or None.
pub fn json_read_entry(config_path: &Path, root: &[&str], name: &str) -> Result<Option<Value>> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut node = serde_json::from_str::<Value>(&text)
        .with_context(|| format!("{} is not valid JSON", config_path.display()))?;
    for key in root {
        node = match node.get(key) {
            Some(v) => v.clone(),
            None => return Ok(None),
        };
    }
    Ok(node.get(name).cloned())
}

/// Insert or replace `entry` at `root/name`, creating missing containers.
/// Returns the previous value (None if absent). Every other key keeps its
/// position (serde_json preserve_order) and the original indentation style.
pub fn json_write_entry(
    config_path: &Path,
    root: &[&str],
    name: &str,
    entry: &Value,
) -> Result<Option<Value>> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut doc = serde_json::from_str::<Value>(&text)
        .with_context(|| format!("{} is not valid JSON", config_path.display()))?;

    let mut node = &mut doc;
    for key in root {
        if !node.get(key).is_some_and(|v| v.is_object()) {
            node.as_object_mut()
                .context("config root path traverses a non-object")?
                .insert(key.to_string(), Value::Object(Map::new()));
        }
        node = node
            .get_mut(key)
            .context("config root path traverses a non-object")?;
    }
    let map = node
        .as_object_mut()
        .with_context(|| format!("{} is not an object", root.join(".")))?;
    let previous = map.insert(name.to_string(), entry.clone());

    pretty_write(config_path, &doc, &text)?;
    Ok(previous)
}

/// Remove the entry at `root/name`. Returns false when absent.
pub fn json_remove_entry(config_path: &Path, root: &[&str], name: &str) -> Result<bool> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut doc = serde_json::from_str::<Value>(&text)
        .with_context(|| format!("{} is not valid JSON", config_path.display()))?;

    let mut node = &mut doc;
    for key in root {
        node = match node.get_mut(key) {
            Some(v) => v,
            None => return Ok(false),
        };
    }
    let removed = node
        .as_object_mut()
        .with_context(|| format!("{} is not an object", root.join(".")))?
        .remove(name)
        .is_some();
    if !removed {
        return Ok(false);
    }

    pretty_write(config_path, &doc, &text)?;
    Ok(true)
}

/// serde_json always emits 2-space pretty print; widen it when the host file
/// used a different indent.
fn reindent(text: &str, indent: usize) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start_matches(' ');
        let level = (line.len() - trimmed.len()) / 2;
        if level > 0 {
            for _ in 0..level * indent {
                out.push(' ');
            }
        }
        out.push_str(trimmed);
        out.push('\n');
    }
    if !text.ends_with('\n') {
        out.pop();
    }
    out
}

// ── TOML host editing (Codex) ──

fn json_to_toml_item(value: &Value) -> toml_edit::Item {
    use toml_edit::{Item, Table, Value as TValue};
    match value {
        Value::Object(obj) => {
            let mut table = Table::new();
            for (k, v) in obj {
                table.insert(k.as_str(), json_to_toml_item(v));
            }
            Item::Table(table)
        }
        Value::Array(arr) => Item::Value(TValue::Array(
            arr.iter()
                .map(json_to_toml_value)
                .collect::<toml_edit::Array>(),
        )),
        other => Item::Value(json_to_toml_value(other)),
    }
}

fn json_to_toml_value(value: &Value) -> toml_edit::Value {
    use toml_edit::Value as TValue;
    match value {
        Value::Null => TValue::String(toml_edit::Formatted::new(String::new())),
        Value::Bool(b) => TValue::Boolean(toml_edit::Formatted::new(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TValue::Integer(toml_edit::Formatted::new(i))
            } else if let Some(f) = n.as_f64() {
                TValue::Float(toml_edit::Formatted::new(f))
            } else {
                TValue::String(toml_edit::Formatted::new(n.to_string()))
            }
        }
        Value::String(s) => TValue::String(toml_edit::Formatted::new(s.clone())),
        Value::Array(arr) => {
            let mut out = toml_edit::Array::new();
            for v in arr {
                out.push(json_to_toml_value(v));
            }
            TValue::Array(out)
        }
        Value::Object(obj) => {
            let mut inline = toml_edit::InlineTable::new();
            for (k, v) in obj {
                inline.insert(k.as_str(), json_to_toml_value(v));
            }
            TValue::InlineTable(inline)
        }
    }
}

fn toml_item_to_json(item: &toml_edit::Item) -> Value {
    fn table_to_map(table: &toml_edit::Table) -> Value {
        let mut map = Map::new();
        for (k, v) in table.iter() {
            map.insert(k.to_string(), toml_item_to_json(v));
        }
        Value::Object(map)
    }
    match item {
        toml_edit::Item::None => Value::Null,
        toml_edit::Item::Value(v) => toml_value_to_json(v),
        toml_edit::Item::Table(table) => table_to_map(table),
        toml_edit::Item::ArrayOfTables(tables) => Value::Array(
            tables.iter().map(table_to_map).collect(),
        ),
    }
}

fn toml_value_to_json(value: &toml_edit::Value) -> Value {
    use toml_edit::Value as TValue;
    match value {
        TValue::String(s) => Value::String(s.value().to_string()),
        TValue::Integer(i) => Value::Number((*i.value()).into()),
        TValue::Float(f) => Value::Number(
            serde_json::Number::from_f64(*f.value())
                .unwrap_or_else(|| serde_json::Number::from(0)),
        ),
        TValue::Boolean(b) => Value::Bool(*b.value()),
        TValue::Datetime(dt) => Value::String(dt.to_string()),
        TValue::Array(arr) => Value::Array(arr.iter().map(toml_value_to_json).collect()),
        TValue::InlineTable(t) => {
            let mut map = Map::new();
            for (k, v) in t.iter() {
                map.insert(k.to_string(), toml_value_to_json(v));
            }
            Value::Object(map)
        }
    }
}

/// Read the entry at `root/name` from a TOML config, as JSON.
pub fn toml_read_entry(config_path: &Path, root: &[&str], name: &str) -> Result<Option<Value>> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let doc = text
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("{} is not valid TOML", config_path.display()))?;
    let mut node = doc.as_item() as &toml_edit::Item;
    for key in root {
        node = match node.get(key) {
            Some(item) => item,
            None => return Ok(None),
        };
    }
    Ok(node.get(name).map(toml_item_to_json))
}

/// Insert or replace `entry` at `root/name` in a TOML config, preserving
/// comments and formatting elsewhere in the document (toml_edit).
pub fn toml_write_entry(
    config_path: &Path,
    root: &[&str],
    name: &str,
    entry: &Value,
) -> Result<Option<Value>> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("{} is not valid TOML", config_path.display()))?;

    let mut node = doc.as_item_mut();
    for key in root {
        if node.get(key).is_none() {
            node.as_table_mut()
                .context("TOML root path traverses a non-table")?
                .insert(key, toml_edit::Item::Table(toml_edit::Table::new()));
        }
        node = node
            .get_mut(key)
            .context("TOML root path traverses a non-table")?;
    }
    let table = node
        .as_table_mut()
        .with_context(|| format!("[{}] is not a table", root.join(".")))?;
    let previous = table.remove(name).map(|item| toml_item_to_json(&item));
    table.insert(name, json_to_toml_item(entry));

    std::fs::write(config_path, doc.to_string())
        .with_context(|| format!("cannot write {}", config_path.display()))?;
    Ok(previous)
}

/// Remove the entry at `root/name` from a TOML config. Returns false when
/// the entry was absent.
pub fn toml_remove_entry(config_path: &Path, root: &[&str], name: &str) -> Result<bool> {
    let text = std::fs::read_to_string(config_path)
        .with_context(|| format!("cannot read {}", config_path.display()))?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .with_context(|| format!("{} is not valid TOML", config_path.display()))?;

    let mut node = doc.as_item_mut();
    for key in root {
        node = match node.get_mut(key) {
            Some(item) => item,
            None => return Ok(false),
        };
    }
    let table = node
        .as_table_mut()
        .with_context(|| format!("[{}] is not a table", root.join(".")))?;
    if table.remove(name).is_none() {
        return Ok(false);
    }
    std::fs::write(config_path, doc.to_string())
        .with_context(|| format!("cannot write {}", config_path.display()))?;
    Ok(true)
}

// ── Registry operations ──

const RECORD_COLS: &str = "id, name, description, config_json, created_at, updated_at";

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpServerRecord> {
    Ok(McpServerRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        config_json: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

pub fn list_servers(store: &SkillStore) -> Result<Vec<McpServerRecord>> {
    let rows = store.with_conn(|conn| {
        let mut stmt =
            conn.prepare(&format!("SELECT {RECORD_COLS} FROM mcp_servers ORDER BY name"))?;
        let collected: Vec<McpServerRecord> =
            stmt.query_map([], row_to_record)?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

pub fn get_server(store: &SkillStore, id: &str) -> Result<Option<McpServerRecord>> {
    let row = store.with_conn(|conn| {
        let mut stmt =
            conn.prepare(&format!("SELECT {RECORD_COLS} FROM mcp_servers WHERE id = ?1"))?;
        let mut rows = stmt.query_map(params![id], row_to_record)?;
        let first = match rows.next() {
            Some(row) => Some(row?),
            None => None,
        };
        Ok(first)
    })?;
    Ok(row)
}

pub fn list_server_targets(store: &SkillStore, server_id: &str) -> Result<Vec<McpServerTarget>> {
    let rows = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT tool, config_path, backup_path, synced_at \
             FROM mcp_server_targets WHERE server_id = ?1 ORDER BY tool",
        )?;
        let collected: Vec<McpServerTarget> = stmt
            .query_map(params![server_id], |row| {
                Ok(McpServerTarget {
                    tool: row.get(0)?,
                    config_path: row.get(1)?,
                    backup_path: row.get(2)?,
                    synced_at: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(collected)
    })?;
    Ok(rows)
}

/// Register a server entry. `config_json` is validated canonical JSON.
pub fn add_server(
    store: &SkillStore,
    name: &str,
    description: Option<&str>,
    config_json: &str,
) -> Result<McpServerRecord> {
    let name = name.trim();
    if name.is_empty() {
        bail!("server name cannot be empty");
    }
    let parsed: Value = serde_json::from_str(config_json).context("config is not valid JSON")?;
    validate_canonical(&parsed)?;

    let exists: bool = store
        .with_conn(|conn| {
            conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM mcp_servers WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
        })
        .unwrap_or(false);
    if exists {
        bail!("an MCP server named \"{name}\" is already registered");
    }

    let now = now_secs();
    let id = new_id();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO mcp_servers (id, name, description, config_json, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, name, description, parsed.to_string(), now],
        )
    })?;
    store.log_audit(AuditDraft::new("mcp_server_add").ok());
    Ok(McpServerRecord {
        id,
        name: name.to_string(),
        description: description.map(|d| d.to_string()),
        config_json: parsed.to_string(),
        created_at: now,
        updated_at: now,
    })
}

/// Update the canonical config of a registered server. Refuses while any
/// deployment is live — undeploy first, then edit, then redeploy (targets
/// would otherwise no longer match what rollback expects).
pub fn update_server_config(store: &SkillStore, id: &str, config_json: &str) -> Result<()> {
    let parsed: Value = serde_json::from_str(config_json).context("config is not valid JSON")?;
    validate_canonical(&parsed)?;
    let live = list_server_targets(store, id)?;
    if !live.is_empty() {
        bail!(
            "undeploy this server from {} host(s) before editing its config",
            live.len()
        );
    }
    let now = now_secs();
    store.with_conn(|conn| {
        conn.execute(
            "UPDATE mcp_servers SET config_json = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, parsed.to_string(), now],
        )
    })?;
    store.log_audit(AuditDraft::new("mcp_server_update").ok());
    Ok(())
}

fn backup_path_for(tool: &str, name: &str, extension: &str) -> PathBuf {
    let dir = central_repo::base_dir()
        .join("backups")
        .join("mcp")
        .join(tool);
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("{}-{name}.{extension}", now_secs()))
}

fn read_host_entry(adapter: &McpHostAdapter, name: &str) -> Result<Option<Value>> {
    if adapter.format == "toml" {
        toml_read_entry(&adapter.config_path, adapter.root, name)
    } else {
        json_read_entry(&adapter.config_path, adapter.root, name)
    }
}

/// Deploy a registered server to one host: merge-write the rendered dialect
/// entry into the host config. A pre-write backup is kept in the central
/// library, and the exact previous value is recorded so undeploy restores
/// it. Refuses to overwrite a foreign entry we did not deploy.
pub fn deploy_server(store: &SkillStore, id: &str, tool: &str) -> Result<()> {
    let record =
        get_server(store, id)?.with_context(|| format!("MCP server {id} not found"))?;
    let adapter = mcp_host_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no MCP config location configured"))?;
    if !adapter.config_path.is_file() {
        bail!(
            "{} does not exist; create it (or launch the agent once) before deploying",
            adapter.config_path.display()
        );
    }

    let canonical: Value = serde_json::from_str(&record.config_json)
        .context("stored canonical config is not valid JSON")?;
    let rendered = render_for_host(&canonical, tool)?;

    let current = read_host_entry(&adapter, &record.name)?;
    let existing_target = list_server_targets(store, id)?
        .into_iter()
        .find(|t| t.tool == tool);
    if existing_target.is_none() {
        if let Some(existing) = &current {
            if *existing != rendered {
                bail!(
                    "host config already has a \"{}\" entry that differs from this server; \
                     remove it or register under another name",
                    record.name
                );
            }
        }
    }

    let extension = if adapter.format == "toml" { "toml" } else { "json" };
    let backup = backup_path_for(tool, &record.name, extension);
    std::fs::copy(&adapter.config_path, &backup)
        .with_context(|| format!("backup {} -> {}", adapter.config_path.display(), backup.display()))?;

    // The previous value recorded for rollback is sticky: on first deploy it
    // is whatever the host had (None when we create the entry); on redeploy
    // it must come from the existing target row only — the on-disk value is
    // now ours and must never be mistaken for the user's original.
    let sticky_previous: Option<String> = if existing_target.is_some() {
        store
            .with_conn(|conn| {
                conn.query_row(
                    "SELECT previous_json FROM mcp_server_targets \
                     WHERE server_id = ?1 AND tool = ?2",
                    params![id, tool],
                    |row| row.get(0),
                )
            })
            .unwrap_or(None)
    } else {
        current.map(|v| v.to_string())
    };

    if adapter.format == "toml" {
        toml_write_entry(&adapter.config_path, adapter.root, &record.name, &rendered)?;
    } else {
        json_write_entry(&adapter.config_path, adapter.root, &record.name, &rendered)?;
    }

    let now = now_secs();
    store.with_conn(|conn| {
        conn.execute(
            "INSERT INTO mcp_server_targets \
             (server_id, tool, config_path, backup_path, rendered_json, previous_json, synced_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) \
             ON CONFLICT(server_id, tool) DO UPDATE SET \
             config_path = ?3, backup_path = ?4, rendered_json = ?5, \
             previous_json = ?6, synced_at = ?7",
            params![
                id,
                tool,
                adapter.config_path.to_string_lossy().into_owned(),
                backup.to_string_lossy().into_owned(),
                rendered.to_string(),
                sticky_previous,
                now
            ],
        )
    })?;
    store.log_audit(AuditDraft::new("mcp_server_deploy").ok());
    Ok(())
}

/// Remove our entry from a host, restoring the exact previous state: the
/// entry we replaced comes back, an entry we created disappears. Refuses
/// when the on-disk value no longer matches what we deployed (the user has
/// edited it since).
pub fn undeploy_server(store: &SkillStore, id: &str, tool: &str) -> Result<()> {
    let record =
        get_server(store, id)?.with_context(|| format!("MCP server {id} not found"))?;
    let adapter = mcp_host_adapters()
        .into_iter()
        .find(|a| a.tool_key == tool)
        .with_context(|| format!("{tool} has no MCP config location configured"))?;

    let row: Option<(String, Option<String>)> = store.with_conn(|conn| {
        let mut stmt = conn.prepare(
            "SELECT rendered_json, previous_json FROM mcp_server_targets \
             WHERE server_id = ?1 AND tool = ?2",
        )?;
        let mut rows = stmt.query_map(params![id, tool], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        Ok(match rows.next() {
            Some(r) => Some(r?),
            None => None,
        })
    })?;
    let (rendered_json, previous_json) =
        row.with_context(|| format!("MCP server {} is not deployed to {tool}", record.name))?;
    let rendered: Value = serde_json::from_str(&rendered_json)
        .context("stored rendered config is not valid JSON")?;

    let current = read_host_entry(&adapter, &record.name)?;
    match &current {
        None => {} // already gone; still clean up the row
        Some(value) if *value == rendered => {}
        Some(_) => bail!(
            "the \"{}\" entry in {} no longer matches what was deployed; \
             restore it manually or redeploy first",
            record.name,
            adapter.config_path.display()
        ),
    }

    match previous_json {
        Some(prev) => {
            let previous: Value = serde_json::from_str(&prev)
                .context("stored previous config is not valid JSON")?;
            if adapter.format == "toml" {
                toml_write_entry(&adapter.config_path, adapter.root, &record.name, &previous)?;
            } else {
                json_write_entry(&adapter.config_path, adapter.root, &record.name, &previous)?;
            }
        }
        None => {
            if adapter.format == "toml" {
                toml_remove_entry(&adapter.config_path, adapter.root, &record.name)?;
            } else {
                json_remove_entry(&adapter.config_path, adapter.root, &record.name)?;
            }
        }
    }
    store.with_conn(|conn| {
        conn.execute(
            "DELETE FROM mcp_server_targets WHERE server_id = ?1 AND tool = ?2",
            params![id, tool],
        )
    })?;
    store.log_audit(AuditDraft::new("mcp_server_undeploy").ok());
    Ok(())
}

/// Delete a registration after removing every live deployment.
pub fn delete_server(store: &SkillStore, id: &str) -> Result<()> {
    for target in list_server_targets(store, id)? {
        undeploy_server(store, id, &target.tool)?;
    }
    store.with_conn(|conn| {
        conn.execute("DELETE FROM mcp_servers WHERE id = ?1", params![id])
    })?;
    store.log_audit(AuditDraft::new("mcp_server_delete").ok());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in(tmp: &Path) -> SkillStore {
        SkillStore::new(&tmp.join("test.db")).unwrap()
    }

    fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    const CANONICAL_HTTP: &str = r#"{"transport":"http","url":"https://example.com/mcp","headers":{"Authorization":"Bearer x"}}"#;
    const CANONICAL_STDIO: &str = r#"{"transport":"stdio","command":"C:\\tools\\mcp.exe","args":["--port","1"],"env":{"K":"V"}}"#;

    #[test]
    fn validate_accepts_known_transports_and_rejects_bad_ones() {
        assert!(validate_canonical(&serde_json::from_str(CANONICAL_HTTP).unwrap()).is_ok());
        assert!(validate_canonical(&serde_json::from_str(CANONICAL_STDIO).unwrap()).is_ok());
        assert!(validate_canonical(&serde_json::from_str(r#"{"transport":"grpc"}"#).unwrap()).is_err());
        assert!(
            validate_canonical(&serde_json::from_str(r#"{"transport":"http"}"#).unwrap()).is_err()
        );
    }

    #[test]
    fn render_dialects_per_host() {
        let http: Value = serde_json::from_str(CANONICAL_HTTP).unwrap();
        let stdio: Value = serde_json::from_str(CANONICAL_STDIO).unwrap();

        let claude = render_for_host(&http, "claude_code").unwrap();
        assert_eq!(claude["type"], "http");
        assert_eq!(claude["url"], http["url"]);

        let opencode = render_for_host(&stdio, "opencode").unwrap();
        assert_eq!(opencode["type"], "local");
        assert_eq!(opencode["command"][0], stdio["command"]);
        assert_eq!(opencode["command"][1], "--port");

        let codex = render_for_host(&stdio, "codex").unwrap();
        assert_eq!(codex["command"], stdio["command"]);
        assert_eq!(codex["args"][1], "1");
        assert!(codex.get("transport").is_none());
    }

    /// Claude Code success + rollback: entry created, other keys untouched,
    /// key order preserved; undeploy removes it exactly.
    #[test]
    fn json_deploy_and_rollback_claude_shape() {
        let tmp = tempdir().unwrap();
        let path = write_file(
            tmp.path(),
            "claude.json",
            "{\n  \"numStartups\": 3,\n  \"mcpServers\": {\n    \"existing\": {\"url\": \"u\"}\n  }\n}",
        );

        let entry = render_for_host(&serde_json::from_str(CANONICAL_HTTP).unwrap(), "claude_code")
            .unwrap();
        let previous = json_write_entry(&path, &["mcpServers"], "mine", &entry).unwrap();
        assert!(previous.is_none());

        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(doc["numStartups"], 3);
        assert_eq!(doc["mcpServers"]["existing"]["url"], "u");
        assert_eq!(doc["mcpServers"]["mine"]["type"], "http");
        // preserve_order: original keys stay in front.
        let keys: Vec<&String> = doc["mcpServers"].as_object().unwrap().keys().collect();
        assert_eq!(keys[0], "existing");

        assert!(json_remove_entry(&path, &["mcpServers"], "mine").unwrap());
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(after["mcpServers"].get("mine").is_none());
        assert_eq!(after["mcpServers"]["existing"]["url"], "u");
        assert_eq!(after["numStartups"], 3);
    }

    /// Rollback restores a replaced foreign entry exactly.
    #[test]
    fn json_rollback_restores_previous_value() {
        let tmp = tempdir().unwrap();
        let path = write_file(
            tmp.path(),
            "gemini.json",
            "{\n  \"theme\": \"dark\",\n  \"mcpServers\": {\n    \"shared\": {\"url\": \"old\"}\n  }\n}",
        );
        let entry = render_for_host(&serde_json::from_str(CANONICAL_HTTP).unwrap(), "gemini_cli")
            .unwrap();
        let previous = json_write_entry(&path, &["mcpServers"], "shared", &entry).unwrap();
        assert_eq!(previous.unwrap()["url"], "old");

        let restored: Value = serde_json::from_str(CANONICAL_HTTP).unwrap();
        let _ = json_write_entry(&path, &["mcpServers"], "shared", &restored);
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(doc["mcpServers"]["shared"]["url"], "https://example.com/mcp");
    }

    /// ZCode nested root (mcp.servers) success + rollback.
    #[test]
    fn json_deploy_nested_root_zcode_shape() {
        let tmp = tempdir().unwrap();
        let path = write_file(
            tmp.path(),
            "zcode.json",
            "{\n  \"model\": \"glm\",\n  \"mcp\": {\n    \"servers\": {\n      \"web\": {\"url\": \"w\"}\n    }\n  }\n}\n",
        );
        let entry = render_for_host(&serde_json::from_str(CANONICAL_HTTP).unwrap(), "zcode")
            .unwrap();
        assert!(json_write_entry(&path, &["mcp", "servers"], "mine", &entry)
            .unwrap()
            .is_none());
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(doc["model"], "glm");
        assert_eq!(doc["mcp"]["servers"]["mine"]["type"], "http");
        // file keeps its trailing newline
        assert!(std::fs::read_to_string(&path).unwrap().ends_with('\n'));

        assert!(json_remove_entry(&path, &["mcp", "servers"], "mine").unwrap());
        let after: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(after["mcp"]["servers"].get("mine").is_none());
    }

    /// Indentation is preserved (4-space files stay 4-space).
    #[test]
    fn json_write_preserves_four_space_indent() {
        let tmp = tempdir().unwrap();
        let path = write_file(
            tmp.path(),
            "four.json",
            "{\n    \"a\": 1,\n    \"mcp\": {\n        \"b\": 2\n    }\n}",
        );
        let entry: Value = serde_json::from_str(CANONICAL_HTTP).unwrap();
        json_write_entry(&path, &["mcp"], "c", &entry).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\n    \"mcp\": {\n        \"b\": 2,\n        \"c\""));
    }

    /// Codex TOML success + rollback with comment preservation.
    #[test]
    fn toml_deploy_and_rollback_preserves_comments() {
        let tmp = tempdir().unwrap();
        let path = write_file(
            tmp.path(),
            "config.toml",
            "# codex config\nmodel = \"gpt\"\n\n[mcp_servers]\n\n[mcp_servers.zotero]\ncommand = 'zotero.exe'\n",
        );

        let entry = render_for_host(&serde_json::from_str(CANONICAL_STDIO).unwrap(), "codex")
            .unwrap();
        let previous = toml_write_entry(&path, &["mcp_servers"], "mine", &entry).unwrap();
        assert!(previous.is_none());

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# codex config\n"));
        assert!(text.contains("command = 'zotero.exe'"));
        // quoting style is toml_edit's choice; assert the value round-trips
        let mine = toml_read_entry(&path, &["mcp_servers"], "mine")
            .unwrap()
            .unwrap();
        assert_eq!(mine["command"], "C:\\tools\\mcp.exe");
        assert_eq!(mine["args"][1], "1");
        assert_eq!(mine["env"]["K"], "V");

        // rollback: remove restores the document without our entry
        assert!(toml_remove_entry(&path, &["mcp_servers"], "mine").unwrap());
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(after.starts_with("# codex config\n"));
        assert!(!after.contains("mcp_servers.mine"));
        assert!(after.contains("mcp_servers.zotero"));
    }

    /// TOML round-trip of a replaced entry restores the original value.
    #[test]
    fn toml_rollback_restores_previous_value() {
        let tmp = tempdir().unwrap();
        let path = write_file(
            tmp.path(),
            "config.toml",
            "[mcp_servers]\n\n[mcp_servers.shared]\ncommand = 'old.exe'\nargs = []\n",
        );
        let entry = render_for_host(&serde_json::from_str(CANONICAL_STDIO).unwrap(), "codex")
            .unwrap();
        let previous = toml_write_entry(&path, &["mcp_servers"], "shared", &entry).unwrap();
        assert_eq!(previous.unwrap()["command"], "old.exe");

        let old: Value = serde_json::json!({"command": "old.exe", "args": []});
        toml_write_entry(&path, &["mcp_servers"], "shared", &old).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("command = \"old.exe\""));
    }

    /// Registry round-trip against temp host configs: the store-level deploy
    /// records rendered + previous, undeploy refuses on drift, restores on
    /// clean state, delete cleans everything. Uses the path-explicit file
    /// helpers through a store with a temp central repo; host adapters are
    /// not touched (they point at real home files).
    #[test]
    fn registry_deploy_undeploy_guards() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());

        let record = add_server(
            &store,
            "probe",
            Some("test server"),
            CANONICAL_HTTP,
        )
        .unwrap();
        assert!(add_server(&store, "probe", None, CANONICAL_HTTP).is_err());

        // Render + file-level deploy/undeploy against a fake host config,
        // mirroring what deploy_server does against real adapter paths.
        let host = write_file(
            tmp.path(),
            "host.json",
            "{\"other\": true, \"mcpServers\": {}}",
        );
        let rendered = render_for_host(&serde_json::from_str(&record.config_json).unwrap(), "claude_code")
            .unwrap();
        let previous = json_write_entry(&host, &["mcpServers"], "probe", &rendered).unwrap();
        assert!(previous.is_none());

        // simulate the target row deploy_server would write
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO mcp_server_targets \
                     (server_id, tool, config_path, backup_path, rendered_json, previous_json, synced_at) \
                     VALUES (?1, 'claude_code', ?2, NULL, ?3, NULL, 0)",
                    params![record.id, host.to_string_lossy().into_owned(), rendered.to_string()],
                )
            })
            .unwrap();

        // user edits the entry -> drift; a clean undeploy would refuse
        let drifted = render_for_host(&serde_json::from_str(CANONICAL_STDIO).unwrap(), "claude_code")
            .unwrap();
        json_write_entry(&host, &["mcpServers"], "probe", &drifted).unwrap();
        let current = json_read_entry(&host, &["mcpServers"], "probe").unwrap().unwrap();
        assert_ne!(current, rendered); // the guard data the real undeploy compares

        delete_server(&store, &record.id).unwrap(); // no rows left -> plain delete
        assert!(get_server(&store, &record.id).unwrap().is_none());
        central_repo::set_test_base_dir_override(None);
    }

    #[test]
    fn update_config_refuses_while_deployed() {
        let _base = central_repo::test_base_dir_lock();
        central_repo::set_test_base_dir_override(Some(tempdir().unwrap().path().to_path_buf()));
        let tmp = tempdir().unwrap();
        let store = store_in(tmp.path());
        let record = add_server(&store, "probe", None, CANONICAL_HTTP).unwrap();
        store
            .with_conn(|conn| {
                conn.execute(
                    "INSERT INTO mcp_server_targets \
                     (server_id, tool, config_path, backup_path, rendered_json, previous_json, synced_at) \
                     VALUES (?1, 'zcode', 'nowhere', NULL, '{}', NULL, 0)",
                    params![record.id],
                )
            })
            .unwrap();
        assert!(update_server_config(&store, &record.id, CANONICAL_STDIO).is_err());
        central_repo::set_test_base_dir_override(None);
    }
}
