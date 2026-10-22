import { useCallback, useEffect, useMemo, useState } from "react";
import { Plug, Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { McpHostInfo, McpServerRecord, McpServerTarget } from "../lib/tauri";
import {
  addMcpServer,
  deleteMcpServer,
  deployMcpServer,
  getMcpHostAdapters,
  getMcpServerTargets,
  getMcpServers,
  undeployMcpServer,
} from "../lib/tauri";
import { AgentIcon } from "../components/AgentIcon";
import { getErrorMessage } from "../lib/error";
import { toast } from "sonner";
import { cn } from "../utils";

const HTTP_TEMPLATE = `{
  "transport": "http",
  "url": "https://example.com/mcp"
}`;
const STDIO_TEMPLATE = `{
  "transport": "stdio",
  "command": "C:\\\\tools\\\\server.exe",
  "args": [],
  "env": {}
}`;

function summarizeConfig(configJson: string): string {
  try {
    const parsed = JSON.parse(configJson) as { transport?: string; url?: string; command?: string };
    if (parsed.transport === "http") return parsed.url ?? "";
    return parsed.command ?? "";
  } catch {
    return "";
  }
}

export function Mcp() {
  const { t } = useTranslation();
  const [servers, setServers] = useState<McpServerRecord[]>([]);
  const [hosts, setHosts] = useState<McpHostInfo[]>([]);
  const [targetsByServer, setTargetsByServer] = useState<Record<string, McpServerTarget[]>>({});
  const [busy, setBusy] = useState<string | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [newConfig, setNewConfig] = useState(HTTP_TEMPLATE);

  const refresh = useCallback(async () => {
    const [list, hostList] = await Promise.all([getMcpServers(), getMcpHostAdapters()]);
    setServers(list);
    setHosts(hostList);
    const entries = await Promise.all(
      list.map(async (s) => [s.id, await getMcpServerTargets(s.id)] as const)
    );
    setTargetsByServer(Object.fromEntries(entries));
  }, []);

  useEffect(() => {
    refresh().catch((error) => toast.error(getErrorMessage(error, t("common.error"))));
  }, [refresh, t]);

  const handleAdd = async () => {
    setBusy("add");
    try {
      await addMcpServer(newName, newDescription.trim() || null, newConfig);
      toast.success(t("mcp.added", { name: newName.trim() }));
      setNewName("");
      setNewDescription("");
      setNewConfig(HTTP_TEMPLATE);
      setShowAdd(false);
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleToggle = async (server: McpServerRecord, tool: string, deploy: boolean) => {
    setBusy(`${server.id}:${tool}`);
    try {
      if (deploy) {
        await deployMcpServer(server.id, tool);
      } else {
        await undeployMcpServer(server.id, tool);
      }
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (server: McpServerRecord) => {
    setBusy(server.id);
    try {
      await deleteMcpServer(server.id);
      toast.success(t("mcp.deleted", { name: server.name }));
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const availableHosts = useMemo(() => hosts.filter((h) => h.exists), [hosts]);

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="border-b border-border-subtle px-6 py-3">
        <div className="flex items-center gap-2">
          <Plug className="h-5 w-5 text-accent-light" />
          <h1 className="text-[15px] font-semibold text-primary">{t("mcp.title")}</h1>
          <span className="text-[12px] text-faint">{t("mcp.hint")}</span>
          <button
            onClick={() => setShowAdd((v) => !v)}
            className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-border-subtle px-2.5 py-1 text-[13px] text-secondary hover:bg-surface-hover"
          >
            <Plus className="h-3.5 w-3.5" />
            {t("mcp.add")}
          </button>
        </div>
      </div>

      <div className="flex-1 space-y-6 overflow-y-auto px-6 py-4">
        {showAdd && (
          <section className="space-y-2 rounded-lg border border-border-subtle p-3">
            <div className="flex flex-wrap items-center gap-2">
              <input
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                placeholder={t("mcp.namePlaceholder")}
                className="w-48 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[13px] text-primary outline-none focus:border-accent-border"
              />
              <input
                value={newDescription}
                onChange={(e) => setNewDescription(e.target.value)}
                placeholder={t("mcp.descriptionPlaceholder")}
                className="w-64 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[13px] text-primary outline-none focus:border-accent-border"
              />
              <button
                onClick={() => setNewConfig(HTTP_TEMPLATE)}
                className="rounded-md border border-border-subtle px-2 py-1 text-[12px] text-muted hover:bg-surface-hover"
              >
                {t("mcp.httpTemplate")}
              </button>
              <button
                onClick={() => setNewConfig(STDIO_TEMPLATE)}
                className="rounded-md border border-border-subtle px-2 py-1 text-[12px] text-muted hover:bg-surface-hover"
              >
                {t("mcp.stdioTemplate")}
              </button>
            </div>
            <textarea
              value={newConfig}
              onChange={(e) => setNewConfig(e.target.value)}
              rows={7}
              spellCheck={false}
              className="w-full rounded-md border border-border-subtle bg-transparent p-2 font-mono text-[12px] text-primary outline-none focus:border-accent-border"
            />
            <button
              onClick={() => void handleAdd()}
              disabled={busy === "add" || newName.trim().length === 0}
              className="inline-flex items-center gap-1 rounded-md bg-accent-dark px-3 py-1.5 text-[13px] font-medium text-white hover:bg-accent disabled:opacity-50"
            >
              {t("mcp.register")}
            </button>
          </section>
        )}

        <section>
          <h2 className="mb-2 text-[13px] font-medium text-secondary">
            {t("mcp.library", { count: servers.length })}
          </h2>
          {servers.length === 0 ? (
            <p className="rounded-lg border border-border-subtle p-4 text-[13px] text-faint">
              {t("mcp.empty")}
            </p>
          ) : (
            <div className="space-y-2">
              {servers.map((server) => {
                const targets = targetsByServer[server.id] ?? [];
                let transport = "?";
                try {
                  transport = (JSON.parse(server.config_json) as { transport?: string }).transport ?? "?";
                } catch {
                  transport = "?";
                }
                return (
                  <div
                    key={server.id}
                    className="rounded-lg border border-border-subtle p-3"
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[13px] font-medium text-primary">
                        {server.name}
                      </span>
                      <span className="rounded-full border border-border-subtle px-1.5 text-[11px] text-muted">
                        {transport}
                      </span>
                      <span className="truncate text-[12px] text-muted">
                        {summarizeConfig(server.config_json)}
                      </span>
                      {server.description && (
                        <span className="truncate text-[12px] text-faint">
                          {server.description}
                        </span>
                      )}
                      <button
                        onClick={() => void handleDelete(server)}
                        disabled={busy === server.id}
                        className="ml-auto inline-flex items-center gap-1 rounded-md px-2 py-1 text-[12px] text-muted hover:bg-red-500/10 hover:text-red-500 disabled:opacity-50"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                        {t("common.delete")}
                      </button>
                    </div>
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {availableHosts.map((host) => {
                        const deployed = targets.some((target) => target.tool === host.tool_key);
                        const key = `${server.id}:${host.tool_key}`;
                        return (
                          <button
                            key={host.tool_key}
                            onClick={() => void handleToggle(server, host.tool_key, !deployed)}
                            disabled={busy === key}
                            title={host.config_path}
                            className={cn(
                              "inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[12px] transition-colors disabled:opacity-50",
                              deployed
                                ? "border-accent-border bg-accent-bg text-accent-light"
                                : "border-border-subtle text-muted hover:bg-surface-hover"
                            )}
                          >
                            <AgentIcon
                              agentKey={host.tool_key}
                              displayName={host.display_name}
                              className="h-4 w-4"
                            />
                            {host.display_name}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </section>

        <section>
          <h2 className="mb-2 text-[13px] font-medium text-secondary">{t("mcp.hosts")}</h2>
          <div className="space-y-1 rounded-lg border border-border-subtle p-1.5">
            {hosts.map((host) => (
              <div key={host.tool_key} className="flex items-center gap-2 px-2 py-1">
                <AgentIcon
                  agentKey={host.tool_key}
                  displayName={host.display_name}
                  className="h-4 w-4"
                />
                <span className="text-[13px] text-secondary">{host.display_name}</span>
                <span className="truncate text-[12px] text-faint">{host.config_path}</span>
                <span
                  className={cn(
                    "ml-auto rounded-full border px-1.5 text-[11px]",
                    host.exists
                      ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-500"
                      : "border-border-subtle text-faint"
                  )}
                >
                  {host.exists ? t("mcp.hostReady") : t("mcp.hostMissing")}
                </span>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
