import { useCallback, useEffect, useMemo, useState } from "react";
import { Terminal, RefreshCw, Upload, Trash2, Camera, History } from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  CommandAdapterInfo,
  DiscoveredCommand,
  MemorySlotInfo,
  SlashCommandRecord,
  SlashCommandTarget,
} from "../lib/tauri";
import {
  deleteMemoryFile,
  deleteSlashCommand,
  deploySlashCommand,
  getMemorySlots,
  getSlashCommandAdapters,
  getSlashCommandTargets,
  getSlashCommands,
  importSlashCommand,
  refreshMemoryFile,
  registerMemoryFile,
  scanSlashCommands,
  undeploySlashCommand,
} from "../lib/tauri";
import { AgentIcon } from "../components/AgentIcon";
import { getErrorMessage } from "../lib/error";
import { toast } from "sonner";
import { cn } from "../utils";

export function Commands() {
  const { t } = useTranslation();
  const [commands, setCommands] = useState<SlashCommandRecord[]>([]);
  const [adapters, setAdapters] = useState<CommandAdapterInfo[]>([]);
  const [discovered, setDiscovered] = useState<DiscoveredCommand[]>([]);
  const [memorySlots, setMemorySlots] = useState<MemorySlotInfo[]>([]);
  const [targetsByCommand, setTargetsByCommand] = useState<
    Record<string, SlashCommandTarget[]>
  >({});
  const [busy, setBusy] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);

  const refresh = useCallback(async () => {
    const [cmds, adpts, slots] = await Promise.all([
      getSlashCommands(),
      getSlashCommandAdapters(),
      getMemorySlots(),
    ]);
    setCommands(cmds);
    setAdapters(adpts);
    setMemorySlots(slots);
    const targetEntries = await Promise.all(
      cmds.map(async (c) => [c.id, await getSlashCommandTargets(c.id)] as const)
    );
    setTargetsByCommand(Object.fromEntries(targetEntries));
  }, []);

  useEffect(() => {
    refresh().catch((error) => toast.error(getErrorMessage(error, t("common.error"))));
  }, [refresh, t]);

  const handleScan = async () => {
    setScanning(true);
    try {
      setDiscovered(await scanSlashCommands());
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setScanning(false);
    }
  };

  const handleAdopt = async (row: DiscoveredCommand) => {
    setBusy(row.path);
    try {
      await importSlashCommand(row.path);
      toast.success(t("commands.adopted", { name: row.file_name }));
      await Promise.all([refresh(), handleScan()]);
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleToggle = async (command: SlashCommandRecord, tool: string, deploy: boolean) => {
    setBusy(`${command.id}:${tool}`);
    try {
      if (deploy) {
        await deploySlashCommand(command.id, tool);
      } else {
        await undeploySlashCommand(command.id, tool);
      }
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleDelete = async (command: SlashCommandRecord) => {
    setBusy(command.id);
    try {
      await deleteSlashCommand(command.id);
      toast.success(t("commands.deleted", { name: command.name }));
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleRegisterMemory = async (slot: MemorySlotInfo) => {
    setBusy(`memory:${slot.tool_key}`);
    try {
      await registerMemoryFile(slot.tool_key);
      toast.success(t("memories.registeredToast", { tool: slot.display_name, name: slot.file_name }));
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleRefreshMemory = async (slot: MemorySlotInfo) => {
    if (!slot.record) return;
    setBusy(`memory:${slot.tool_key}`);
    try {
      await refreshMemoryFile(slot.record.id);
      toast.success(t("memories.refreshedToast", { name: slot.file_name }));
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleDeleteMemory = async (slot: MemorySlotInfo) => {
    if (!slot.record) return;
    setBusy(`memory:${slot.tool_key}`);
    try {
      await deleteMemoryFile(slot.record.id);
      toast.success(t("memories.deletedToast", { name: slot.file_name }));
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
          <Terminal className="h-5 w-5 text-accent-light" />
          <h1 className="text-[15px] font-semibold text-primary">{t("commands.title")}</h1>
          <span className="text-[12px] text-faint">{t("commands.centralDir")}</span>
          <button
            onClick={() => void handleScan()}
            disabled={scanning}
            className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-border-subtle px-2.5 py-1 text-[13px] text-secondary hover:bg-surface-hover disabled:opacity-50"
          >
            <RefreshCw className={cn("h-3.5 w-3.5", scanning && "animate-spin")} />
            {t("commands.scan")}
          </button>
        </div>
      </div>

      <div className="flex-1 space-y-6 overflow-y-auto px-6 py-4">
        <section>
          <h2 className="mb-2 text-[13px] font-medium text-secondary">
            {t("commands.library", { count: commands.length })}
          </h2>
          {commands.length === 0 ? (
            <p className="rounded-lg border border-border-subtle p-4 text-[13px] text-faint">
              {t("commands.empty")}
            </p>
          ) : (
            <div className="space-y-2">
              {commands.map((command) => {
                const targets = targetsByCommand[command.id] ?? [];
                return (
                  <div
                    key={command.id}
                    className="rounded-lg border border-border-subtle p-3"
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[13px] font-medium text-primary">
                        /{command.name}
                      </span>
                      {command.description && (
                        <span className="truncate text-[12px] text-muted">
                          {command.description}
                        </span>
                      )}
                      <button
                        onClick={() => void handleDelete(command)}
                        disabled={busy === command.id}
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
                        const key = `${command.id}:${adapter.tool_key}`;
                        return (
                          <button
                            key={adapter.tool_key}
                            onClick={() =>
                              void handleToggle(command, adapter.tool_key, !deployed)
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
            {t("commands.discovered", { count: untracked.length })}
          </h2>
          {discovered.length === 0 ? (
            <p className="text-[12px] text-faint">{t("commands.scanHint")}</p>
          ) : untracked.length === 0 ? (
            <p className="text-[12px] text-faint">{t("commands.allTracked")}</p>
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
                    {t("commands.adopt")}
                  </button>
                </div>
              ))}
            </div>
          )}
        </section>

        <section>
          <h2 className="mb-2 text-[13px] font-medium text-secondary">
            {t("memories.title")}
          </h2>
          <p className="mb-2 text-[12px] text-faint">{t("memories.hint")}</p>
          <div className="space-y-1 rounded-lg border border-border-subtle p-1.5">
            {memorySlots.map((slot) => {
              const memoryKey = `memory:${slot.tool_key}`;
              const record = slot.record;
              return (
                <div key={slot.tool_key} className="flex items-center gap-2 px-2 py-1">
                  <AgentIcon
                    agentKey={slot.tool_key}
                    displayName={slot.display_name}
                    className="h-4 w-4"
                  />
                  <span className="text-[13px] text-secondary">{slot.file_name}</span>
                  <span className="text-[12px] text-faint">{slot.display_name}</span>
                  {record ? (
                    <>
                      <span className="text-[12px] text-faint">
                        {new Date(record.snapshotted_at * 1000).toLocaleString()} ·{" "}
                        {record.byte_size} B
                      </span>
                      {slot.source_changed === true && (
                        <span className="rounded-full border border-amber-500/40 bg-amber-500/10 px-1.5 text-[11px] text-amber-500">
                          {t("memories.sourceChanged")}
                        </span>
                      )}
                      {!slot.exists_on_disk && (
                        <span className="rounded-full border border-red-500/40 bg-red-500/10 px-1.5 text-[11px] text-red-500">
                          {t("memories.notFound")}
                        </span>
                      )}
                      <button
                        onClick={() => void handleRefreshMemory(slot)}
                        disabled={busy === memoryKey || !slot.exists_on_disk}
                        className="ml-auto inline-flex items-center gap-1 rounded-md border border-border-subtle px-2 py-1 text-[12px] text-secondary hover:bg-surface-hover disabled:opacity-50"
                      >
                        <History className="h-3.5 w-3.5" />
                        {t("memories.refresh")}
                      </button>
                      <button
                        onClick={() => void handleDeleteMemory(slot)}
                        disabled={busy === memoryKey}
                        className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[12px] text-muted hover:bg-red-500/10 hover:text-red-500 disabled:opacity-50"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                        {t("memories.deleteRegistration")}
                      </button>
                    </>
                  ) : slot.exists_on_disk ? (
                    <button
                      onClick={() => void handleRegisterMemory(slot)}
                      disabled={busy === memoryKey}
                      className="ml-auto inline-flex items-center gap-1 rounded-md bg-accent-dark px-2 py-1 text-[12px] font-medium text-white hover:bg-accent disabled:opacity-50"
                    >
                      <Camera className="h-3.5 w-3.5" />
                      {t("memories.register")}
                    </button>
                  ) : (
                    <span className="ml-auto text-[12px] text-faint">
                      {t("memories.absent")}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </section>
      </div>
    </div>
  );
}
