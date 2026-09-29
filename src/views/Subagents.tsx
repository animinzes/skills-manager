import { useCallback, useEffect, useMemo, useState } from "react";
import { Users, RefreshCw, Upload, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  AgentDefinitionRecord,
  AgentDefinitionTarget,
  DiscoveredDefinition,
  SubagentAdapterInfo,
} from "../lib/tauri";
import {
  deleteAgentDefinition,
  deployAgentDefinition,
  getAgentDefinitionAdapters,
  getAgentDefinitionTargets,
  getAgentDefinitions,
  importAgentDefinition,
  scanAgentDefinitions,
  undeployAgentDefinition,
} from "../lib/tauri";
import { AgentIcon } from "../components/AgentIcon";
import { getErrorMessage } from "../lib/error";
import { toast } from "sonner";
import { cn } from "../utils";

export function Subagents() {
  const { t } = useTranslation();
  const [definitions, setDefinitions] = useState<AgentDefinitionRecord[]>([]);
  const [adapters, setAdapters] = useState<SubagentAdapterInfo[]>([]);
  const [discovered, setDiscovered] = useState<DiscoveredDefinition[]>([]);
  const [targetsByDefinition, setTargetsByDefinition] = useState<
    Record<string, AgentDefinitionTarget[]>
  >({});
  const [busy, setBusy] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);

  const refresh = useCallback(async () => {
    const [defs, adpts] = await Promise.all([
      getAgentDefinitions(),
      getAgentDefinitionAdapters(),
    ]);
    setDefinitions(defs);
    setAdapters(adpts);
    const targetEntries = await Promise.all(
      defs.map(async (d) => [d.id, await getAgentDefinitionTargets(d.id)] as const)
    );
    setTargetsByDefinition(Object.fromEntries(targetEntries));
  }, []);

  useEffect(() => {
    refresh().catch((error) => toast.error(getErrorMessage(error, t("common.error"))));
  }, [refresh, t]);

  const handleScan = async () => {
    setScanning(true);
    try {
      setDiscovered(await scanAgentDefinitions());
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setScanning(false);
    }
  };

  const handleAdopt = async (row: DiscoveredDefinition) => {
    setBusy(row.path);
    try {
      await importAgentDefinition(row.path, row.format);
      toast.success(t("subagents.adopted", { name: row.file_name }));
      await Promise.all([refresh(), handleScan()]);
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleToggle = async (definition: AgentDefinitionRecord, tool: string, deploy: boolean) => {
    setBusy(`${definition.id}:${tool}`);
    try {
      if (deploy) {
        await deployAgentDefinition(definition.id, tool);
      } else {
        await undeployAgentDefinition(definition.id, tool);
      }
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (definition: AgentDefinitionRecord) => {
    setBusy(definition.id);
    try {
      await deleteAgentDefinition(definition.id);
      toast.success(t("subagents.deleted", { name: definition.name }));
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const installedAdapters = useMemo(
    () => adapters.filter((a) => a.installed),
    [adapters]
  );
  const untracked = useMemo(
    () => discovered.filter((row) => !row.already_tracked),
    [discovered]
  );

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="border-b border-border-subtle px-6 py-3">
        <div className="flex items-center gap-2">
          <Users className="h-5 w-5 text-accent-light" />
          <h1 className="text-[15px] font-semibold text-primary">{t("subagents.title")}</h1>
          <span className="text-[12px] text-faint">{t("subagents.centralDir")}</span>
          <button
            onClick={() => void handleScan()}
            disabled={scanning}
            className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-border-subtle px-2.5 py-1 text-[13px] text-secondary hover:bg-surface-hover disabled:opacity-50"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", scanning && "animate-spin")} />
            {t("subagents.scan")}
          </button>
        </div>
      </div>

      <div className="flex-1 space-y-6 overflow-y-auto px-6 py-4">
        <section>
          <h2 className="mb-2 text-[13px] font-medium text-secondary">
            {t("subagents.library", { count: definitions.length })}
          </h2>
          {definitions.length === 0 ? (
            <p className="rounded-lg border border-border-subtle p-4 text-[13px] text-faint">
              {t("subagents.empty")}
            </p>
          ) : (
            <div className="space-y-2">
              {definitions.map((definition) => {
                const targets = targetsByDefinition[definition.id] ?? [];
                return (
                  <div
                    key={definition.id}
                    className="rounded-lg border border-border-subtle p-3"
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[13px] font-medium text-primary">
                        {definition.name}
                      </span>
                      <span className="rounded-full border border-border-subtle px-1.5 text-[11px] text-muted">
                        {definition.format}
                      </span>
                      {definition.description && (
                        <span className="truncate text-[12px] text-muted">
                          {definition.description}
                        </span>
                      )}
                      <button
                        onClick={() => void handleDelete(definition)}
                        disabled={busy === definition.id}
                        className="ml-auto inline-flex items-center gap-1 rounded-md px-2 py-1 text-[12px] text-muted hover:bg-red-500/10 hover:text-red-500 disabled:opacity-50"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                        {t("common.delete")}
                      </button>
                    </div>
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {installedAdapters.map((adapter) => {
                        const deployed = targets.some(
                          (target) => target.tool === adapter.tool_key
                        );
                        const key = `${definition.id}:${adapter.tool_key}`;
                        return (
                          <button
                            key={adapter.tool_key}
                            onClick={() =>
                              void handleToggle(definition, adapter.tool_key, !deployed)
                            }
                            disabled={busy === key}
                            className={cn(
                              "inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[12px] transition-colors disabled:opacity-50",
                              deployed
                                ? "border-accent-border bg-accent-bg text-accent-light"
                                : "border-border-subtle text-muted hover:bg-surface-hover"
                            )}
                          >
                            <AgentIcon
                              agentKey={adapter.tool_key}
                              displayName={adapter.display_name}
                              className="h-4 w-4"
                            />
                            {adapter.display_name}
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
          <h2 className="mb-2 text-[13px] font-medium text-secondary">
            {t("subagents.discovered", { count: untracked.length })}
          </h2>
          {discovered.length === 0 ? (
            <p className="text-[12px] text-faint">{t("subagents.scanHint")}</p>
          ) : untracked.length === 0 ? (
            <p className="text-[12px] text-faint">{t("subagents.allTracked")}</p>
          ) : (
            <div className="max-h-[360px] space-y-1 overflow-y-auto rounded-lg border border-border-subtle p-1.5">
              {untracked.map((row) => (
                <div key={row.path} className="flex items-center gap-2 px-2 py-1">
                  <AgentIcon
                    agentKey={row.tool}
                    displayName={row.display_name}
                    className="h-4 w-4"
                  />
                  <span className="text-[13px] text-secondary">{row.file_name}</span>
                  <span className="text-[12px] text-faint">{row.display_name}</span>
                  <button
                    onClick={() => void handleAdopt(row)}
                    disabled={busy === row.path}
                    className="ml-auto inline-flex items-center gap-1 rounded-md bg-accent-dark px-2 py-1 text-[12px] font-medium text-white hover:bg-accent disabled:opacity-50"
                  >
                    <Upload className="h-3.5 w-3.5" />
                    {t("subagents.adopt")}
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
