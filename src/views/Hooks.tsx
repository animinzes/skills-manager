import { useCallback, useEffect, useState } from "react";
import { Zap, Plus, Trash2, ChevronDown, ChevronRight } from "lucide-react";
import { useTranslation } from "react-i18next";
import type {
  HookGroupRecord,
  HookGroupRegistration,
  HookGroupTarget,
  HookGroupFile,
} from "../lib/tauri";
import {
  addHookGroup,
  addHookGroupFile,
  addHookGroupRegistration,
  deleteHookGroup,
  deployHookGroup,
  getHookGroupFiles,
  getHookGroupRegistrations,
  getHookGroupTargets,
  getHookGroups,
  removeHookGroupFile,
  removeHookGroupRegistration,
  undeployHookGroup,
  HOOK_EVENTS,
} from "../lib/tauri";
import { AgentIcon } from "../components/AgentIcon";
import { getErrorMessage } from "../lib/error";
import { toast } from "sonner";
import { cn } from "../utils";

interface GroupDetail {
  files: HookGroupFile[];
  registrations: HookGroupRegistration[];
  targets: HookGroupTarget[];
}

/** Claude Code is the only hook-capable host in this round. */
const HOOK_HOST = { toolKey: "claude_code", displayName: "Claude Code" };

export function Hooks() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<HookGroupRecord[]>([]);
  const [details, setDetails] = useState<Record<string, GroupDetail>>({});
  const [expanded, setExpanded] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [showAdd, setShowAdd] = useState(false);
  const [newName, setNewName] = useState("");
  const [newDescription, setNewDescription] = useState("");
  const [filePath, setFilePath] = useState<Record<string, string>>({});
  const [regForm, setRegForm] = useState<Record<string, { event: string; matcher: string; command: string }>>({});

  const refresh = useCallback(async () => {
    const list = await getHookGroups();
    setGroups(list);
    const entries = await Promise.all(
      list.map(async (g) => {
        const [files, registrations, targets] = await Promise.all([
          getHookGroupFiles(g.id),
          getHookGroupRegistrations(g.id),
          getHookGroupTargets(g.id),
        ]);
        return [g.id, { files, registrations, targets } as GroupDetail] as const;
      })
    );
    setDetails(Object.fromEntries(entries));
  }, []);

  useEffect(() => {
    refresh().catch((error) => toast.error(getErrorMessage(error, t("common.error"))));
  }, [refresh, t]);

  const run = async (key: string, action: () => Promise<void>, done?: string) => {
    setBusy(key);
    try {
      await action();
      if (done) toast.success(done);
      await refresh();
    } catch (error: unknown) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setBusy(null);
    }
  };

  const handleAddGroup = () =>
    run(
      "add-group",
      async () => {
        await addHookGroup(newName, newDescription.trim() || null);
        setNewName("");
        setNewDescription("");
        setShowAdd(false);
      },
      t("hooks.groupAdded", { name: newName.trim() })
    );

  const handleDeploy = (group: HookGroupRecord, deploy: boolean) =>
    run(
      `${group.id}:deploy`,
      () =>
        deploy
          ? deployHookGroup(group.id, HOOK_HOST.toolKey)
          : undeployHookGroup(group.id, HOOK_HOST.toolKey),
      t(deploy ? "hooks.deployed" : "hooks.undeployed", { name: group.name })
    );

  const detail = (id: string): GroupDetail => details[id] ?? { files: [], registrations: [], targets: [] };
  const form = (id: string) =>
    regForm[id] ?? { event: "PreToolUse", matcher: "", command: "" };

  return (
    <div className="flex h-full flex-col overflow-hidden">
      <div className="border-b border-border-subtle px-6 py-3">
        <div className="flex items-center gap-2">
          <Zap className="h-5 w-5 text-accent-light" />
          <h1 className="text-[15px] font-semibold text-primary">{t("hooks.title")}</h1>
          <span className="text-[12px] text-faint">{t("hooks.hint")}</span>
          <button
            onClick={() => setShowAdd((v) => !v)}
            className="ml-auto inline-flex items-center gap-1.5 rounded-md border border-border-subtle px-2.5 py-1 text-[13px] text-secondary hover:bg-surface-hover"
          >
            <Plus className="h-3.5 w-3.5" />
            {t("hooks.addGroup")}
          </button>
        </div>
      </div>

      <div className="flex-1 space-y-4 overflow-y-auto px-6 py-4">
        {showAdd && (
          <section className="flex flex-wrap items-center gap-2 rounded-lg border border-border-subtle p-3">
            <input
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              placeholder={t("hooks.namePlaceholder")}
              className="w-44 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[13px] text-primary outline-none focus:border-accent-border"
            />
            <input
              value={newDescription}
              onChange={(e) => setNewDescription(e.target.value)}
              placeholder={t("hooks.descriptionPlaceholder")}
              className="w-64 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[13px] text-primary outline-none focus:border-accent-border"
            />
            <button
              onClick={() => void handleAddGroup()}
              disabled={busy === "add-group" || newName.trim().length === 0}
              className="rounded-md bg-accent-dark px-3 py-1.5 text-[13px] font-medium text-white hover:bg-accent disabled:opacity-50"
            >
              {t("hooks.create")}
            </button>
          </section>
        )}

        {groups.length === 0 && (
          <p className="rounded-lg border border-border-subtle p-4 text-[13px] text-faint">
            {t("hooks.empty")}
          </p>
        )}

        {groups.map((group) => {
          const d = detail(group.id);
          const deployed = d.targets.some((target) => target.tool === HOOK_HOST.toolKey);
          const open = expanded === group.id;
          const f = form(group.id);
          return (
            <div key={group.id} className="rounded-lg border border-border-subtle p-3">
              <div className="flex items-center gap-2">
                <button
                  onClick={() => setExpanded(open ? null : group.id)}
                  className="text-muted hover:text-primary"
                >
                  {open ? (
                    <ChevronDown className="h-4 w-4" />
                  ) : (
                    <ChevronRight className="h-4 w-4" />
                  )}
                </button>
                <span className="text-[13px] font-medium text-primary">{group.name}</span>
                <span className="text-[12px] text-faint">
                  {d.files.length} {t("hooks.filesUnit")} · {d.registrations.length}{" "}
                  {t("hooks.regsUnit")}
                </span>
                <button
                  onClick={() => void handleDeploy(group, !deployed)}
                  disabled={busy === `${group.id}:deploy`}
                  className={cn(
                    "ml-auto inline-flex items-center gap-1.5 rounded-md border px-2 py-1 text-[12px] transition-colors disabled:opacity-50",
                    deployed
                      ? "border-accent-border bg-accent-bg text-accent-light"
                      : "border-border-subtle text-muted hover:bg-surface-hover"
                  )}
                >
                  <AgentIcon
                    agentKey={HOOK_HOST.toolKey}
                    displayName={HOOK_HOST.displayName}
                    className="h-4 w-4"
                  />
                  {HOOK_HOST.displayName}
                </button>
                <button
                  onClick={() =>
                    void run(group.id, () => deleteHookGroup(group.id), t("hooks.deleted", { name: group.name }))
                  }
                  disabled={busy === group.id}
                  className="inline-flex items-center gap-1 rounded-md px-2 py-1 text-[12px] text-muted hover:bg-red-500/10 hover:text-red-500 disabled:opacity-50"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                  {t("common.delete")}
                </button>
              </div>

              {open && (
                <div className="mt-3 space-y-3 border-t border-border-subtle pt-3">
                  <div className="space-y-1">
                    {d.files.map((file) => (
                      <div key={file.file_name} className="flex items-center gap-2">
                        <span className="text-[12px] text-secondary">{file.file_name}</span>
                        <button
                          onClick={() =>
                            void run(
                              `${group.id}:file:${file.file_name}`,
                              () => removeHookGroupFile(group.id, file.file_name)
                            )
                          }
                          disabled={busy === `${group.id}:file:${file.file_name}`}
                          className="ml-auto text-[11px] text-muted hover:text-red-500"
                        >
                          {t("hooks.remove")}
                        </button>
                      </div>
                    ))}
                    <div className="flex items-center gap-2">
                      <input
                        value={filePath[group.id] ?? ""}
                        onChange={(e) => setFilePath({ ...filePath, [group.id]: e.target.value })}
                        placeholder={t("hooks.filePlaceholder")}
                        className="flex-1 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[12px] text-primary outline-none focus:border-accent-border"
                      />
                      <button
                        onClick={() =>
                          void run(`${group.id}:addfile`, async () => {
                            await addHookGroupFile(group.id, (filePath[group.id] ?? "").trim());
                          })
                        }
                        disabled={busy === `${group.id}:addfile` || !(filePath[group.id] ?? "").trim()}
                        className="rounded-md border border-border-subtle px-2 py-1 text-[12px] text-secondary hover:bg-surface-hover disabled:opacity-50"
                      >
                        {t("hooks.addFile")}
                      </button>
                    </div>
                  </div>

                  <div className="space-y-1">
                    {d.registrations.map((reg) => (
                      <div key={`${reg.event}:${reg.command}`} className="flex items-center gap-2">
                        <span className="rounded-full border border-border-subtle px-1.5 text-[11px] text-muted">
                          {reg.event}
                        </span>
                        {reg.matcher && (
                          <span className="text-[11px] text-faint">{reg.matcher}</span>
                        )}
                        <span className="truncate text-[12px] text-secondary">{reg.command}</span>
                        <button
                          onClick={() =>
                            void run(`${group.id}:reg:${reg.event}`, () =>
                              removeHookGroupRegistration(group.id, reg.event, reg.command)
                            )
                          }
                          disabled={busy === `${group.id}:reg:${reg.event}`}
                          className="ml-auto text-[11px] text-muted hover:text-red-500"
                        >
                          {t("hooks.remove")}
                        </button>
                      </div>
                    ))}
                    <div className="flex flex-wrap items-center gap-2">
                      <select
                        value={f.event}
                        onChange={(e) =>
                          setRegForm({ ...regForm, [group.id]: { ...f, event: e.target.value } })
                        }
                        className="rounded-md border border-border-subtle bg-transparent px-1.5 py-1 text-[12px] text-primary outline-none"
                      >
                        {HOOK_EVENTS.map((event) => (
                          <option key={event} value={event}>
                            {event}
                          </option>
                        ))}
                      </select>
                      <input
                        value={f.matcher}
                        onChange={(e) =>
                          setRegForm({ ...regForm, [group.id]: { ...f, matcher: e.target.value } })
                        }
                        placeholder={t("hooks.matcherPlaceholder")}
                        className="w-36 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[12px] text-primary outline-none focus:border-accent-border"
                      />
                      <input
                        value={f.command}
                        onChange={(e) =>
                          setRegForm({ ...regForm, [group.id]: { ...f, command: e.target.value } })
                        }
                        placeholder={t("hooks.commandPlaceholder")}
                        className="flex-1 rounded-md border border-border-subtle bg-transparent px-2 py-1 text-[12px] text-primary outline-none focus:border-accent-border"
                      />
                      <button
                        onClick={() =>
                          void run(`${group.id}:addreg`, async () => {
                            await addHookGroupRegistration(
                              group.id,
                              f.event,
                              f.matcher.trim() || null,
                              f.command
                            );
                          })
                        }
                        disabled={busy === `${group.id}:addreg` || !f.command.trim()}
                        className="rounded-md border border-border-subtle px-2 py-1 text-[12px] text-secondary hover:bg-surface-hover disabled:opacity-50"
                      >
                        {t("hooks.addRegistration")}
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
