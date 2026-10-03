import { useCallback, useEffect, useMemo, useState } from "react";
import { Boxes, History, Loader2, Package, Pencil, Plus, RefreshCw, Save, Search, Tag, Trash2, Wrench, X, PackagePlus, Grid3x3 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useApp } from "../context/AppContext";
import { getErrorMessage } from "../lib/error";
import {  getRegistry,
  deleteSuite,
  saveSkillGovernance,
  saveSuite,
  setSkillTags,
  setSuiteDeployed,
  type RegistrySkill,
  type RegistrySnapshot,
  type SkillKind,
  type SuiteInput,
  type SuiteRecord,
  SUITE_ASSET_TYPES,
  syncSkillToTool,
  unsyncSkillFromTool,
} from "../lib/tauri";
import { cn } from "../utils";
import { AgentIcon } from "../components/AgentIcon";

type Tab = "skills" | "suites" | "resources" | "history" | "matrix";

const KINDS: SkillKind[] = ["capability", "tool_guide", "integration", "workflow", "governance"];
const inputClass = "rounded-lg border border-border-subtle bg-background px-3 py-2 text-[12.5px] text-secondary outline-none focus:border-border";

function emptySuite(): SuiteInput {
  return {
    id: null,
    name: "",
    description: null,
    version: null,
    category: "uncategorized",
    lifecycle: "active",
    tags: [],
    members: [],
    assets: [],
  };
}

export function Registry() {
  const { t } = useTranslation();
  const { tools } = useApp();
  const [snapshot, setSnapshot] = useState<RegistrySnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<Tab>("skills");
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState<SkillKind | "all">("all");
  const [tag, setTag] = useState("all");
  const [editingSkill, setEditingSkill] = useState<RegistrySkill | null>(null);
  const [skillTags, setSkillTagsDraft] = useState("");
  const [skillKind, setSkillKind] = useState<SkillKind>("capability");
  const [suiteDraft, setSuiteDraft] = useState<SuiteInput | null>(null);
  const [saving, setSaving] = useState(false);
  const [selectedAgent, setSelectedAgent] = useState("");
  const [matrixPage, setMatrixPage] = useState(0);
  const [matrixBusy, setMatrixBusy] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setSnapshot(await getRegistry());
    } catch (error) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { void load(); }, [load]);
  const installedAgents = useMemo(() => tools.filter((tool) => tool.installed && tool.enabled), [tools]);
  useEffect(() => {
    if (!selectedAgent && installedAgents[0]) setSelectedAgent(installedAgents[0].key);
  }, [installedAgents, selectedAgent]);

  const allTags = useMemo(
    () => Array.from(new Set(snapshot?.skills.flatMap((skill) => skill.tags) ?? [])).sort(),
    [snapshot]
  );
  const visibleSkills = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return (snapshot?.skills ?? []).filter((item) => {
      if (kind !== "all" && item.governance.kind !== kind) return false;
      if (tag !== "all" && !item.tags.includes(tag)) return false;
      return !needle || item.skill.name.toLowerCase().includes(needle) || item.tags.some((value) => value.toLowerCase().includes(needle));
    });
  }, [kind, search, snapshot, tag]);

  const beginSkillEdit = (skill: RegistrySkill) => {
    setEditingSkill(skill);
    setSkillKind(skill.governance.kind);
    setSkillTagsDraft(skill.tags.join(", "));
  };

  const saveSkillClassification = async () => {
    if (!editingSkill) return;
    setSaving(true);
    try {
      await Promise.all([
        saveSkillGovernance({ ...editingSkill.governance, kind: skillKind }),
        setSkillTags(editingSkill.skill.id, skillTags.split(",").map((value) => value.trim()).filter(Boolean)),
      ]);
      setEditingSkill(null);
      await load();
      toast.success(t("registry.saved"));
    } catch (error) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setSaving(false);
    }
  };

  const beginSuiteEdit = (suite?: SuiteRecord) => {
    setSuiteDraft(suite ? {
      id: suite.id,
      name: suite.name,
      description: suite.description,
      version: suite.version,
      category: suite.category,
      lifecycle: suite.lifecycle,
      tags: suite.tags,
      members: suite.members,
      assets: suite.assets.map((asset) => ({ asset_type: asset.asset_type, name: asset.name, path: asset.path, tool: asset.tool, notes: asset.notes, sort_order: asset.sort_order })),
    } : emptySuite());
  };

  const toggleSuiteMember = (skillId: string) => {
    if (!suiteDraft) return;
    const exists = suiteDraft.members.some((member) => member.skill_id === skillId);
    setSuiteDraft({
      ...suiteDraft,
      members: exists
        ? suiteDraft.members.filter((member) => member.skill_id !== skillId)
        : [...suiteDraft.members, { skill_id: skillId, required: true, role: null, version_requirement: null, sort_order: suiteDraft.members.length }],
    });
  };

  const persistSuite = async () => {
    if (!suiteDraft || !suiteDraft.name.trim() || suiteDraft.members.length === 0) {
      toast.error(t("registry.suiteRequired"));
      return;
    }
    setSaving(true);
    try {
      await saveSuite(suiteDraft);
      setSuiteDraft(null);
      await load();
      toast.success(t("registry.suiteSaved"));
    } catch (error) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setSaving(false);
    }
  };

  const setSuiteState = async (suite: SuiteRecord, enabled: boolean) => {
    if (!selectedAgent) return;
    setSaving(true);
    try {
      await setSuiteDeployed(suite.id, selectedAgent, enabled);
      await load();
      toast.success(t(enabled ? "registry.suiteDeployed" : "registry.suiteUndeployed"));
    } catch (error) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setSaving(false);
    }
  };

  const removeSuite = async (suite: SuiteRecord) => {
    if (!window.confirm(t("registry.deleteSuiteConfirm", { name: suite.name }))) return;
    setSaving(true);
    try {
      await deleteSuite(suite.id);
      await load();
      toast.success(t("registry.suiteDeleted"));
    } catch (error) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setSaving(false);
    }
  };

  const tabs: { id: Tab; icon: typeof Boxes; count: number }[] = [
    { id: "skills", icon: Boxes, count: snapshot?.skills.length ?? 0 },
    { id: "suites", icon: Package, count: snapshot?.suites.length ?? 0 },
    { id: "resources", icon: Wrench, count: snapshot?.resources.length ?? 0 },
    { id: "history", icon: History, count: snapshot?.history.length ?? 0 },
    { id: "matrix", icon: Grid3x3, count: snapshot?.skills.length ?? 0 },
  ];

  return (
    <div className="h-full overflow-y-auto px-7 py-6">
      <div className="mx-auto max-w-6xl space-y-5">
        <header className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-xl font-semibold text-primary">{t("registry.title")}</h1>
            <p className="mt-1 text-[13px] text-muted">{t("registry.subtitle")}</p>
            {snapshot && <p className="mt-1 text-[11px] text-faint">{snapshot.registry_path}</p>}
          </div>
          <button onClick={() => void load()} className="rounded-lg border border-border-subtle p-2 text-muted hover:bg-surface-hover" title={t("common.refresh")}>
            <RefreshCw className={cn("h-4 w-4", loading && "animate-spin")} />
          </button>
        </header>

        <nav className="flex flex-wrap gap-2">
          {tabs.map(({ id, icon: Icon, count }) => (
            <button key={id} onClick={() => setTab(id)} className={cn("inline-flex items-center gap-2 rounded-lg border px-3 py-2 text-[12.5px]", tab === id ? "border-accent/40 bg-accent/10 text-accent" : "border-border-subtle text-muted hover:bg-surface-hover")}>
              <Icon className="h-3.5 w-3.5" />{t(`registry.tabs.${id}`)}<span className="rounded-full bg-surface px-1.5 text-[11px]">{count}</span>
            </button>
          ))}
        </nav>

        {loading && !snapshot ? <div className="flex justify-center py-20"><Loader2 className="h-5 w-5 animate-spin text-muted" /></div> : null}

        {snapshot && tab === "skills" && (
          <section className="space-y-3">
            <div className="flex flex-wrap gap-2 rounded-xl border border-border-subtle bg-surface p-3">
              <label className="flex min-w-56 flex-1 items-center gap-2 rounded-lg border border-border-subtle bg-background px-3"><Search className="h-3.5 w-3.5 text-faint" /><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={t("registry.search")} className="w-full bg-transparent py-2 text-[12.5px] outline-none" /></label>
              <select value={kind} onChange={(event) => setKind(event.target.value as SkillKind | "all")} className={inputClass}><option value="all">{t("registry.allCategories")}</option>{KINDS.map((value) => <option key={value} value={value}>{t(`governance.options.kind.${value}`)}</option>)}</select>
              <select value={tag} onChange={(event) => setTag(event.target.value)} className={inputClass}><option value="all">{t("registry.allTags")}</option>{allTags.map((value) => <option key={value} value={value}>{value}</option>)}</select>
            </div>
            <div className="overflow-hidden rounded-xl border border-border-subtle bg-surface">
              {visibleSkills.map((item) => (
                <div key={item.skill.id} className="flex flex-wrap items-center gap-3 border-b border-border-subtle px-4 py-3 last:border-b-0">
                  <div className="min-w-48 flex-1"><div className="text-[13px] font-medium text-secondary">{item.skill.name}</div><div className="mt-1 flex flex-wrap gap-1">{item.tags.map((value) => <span key={value} className="rounded bg-background px-1.5 py-0.5 text-[10.5px] text-muted">{value}</span>)}</div></div>
                  <span className="rounded-full border border-border-subtle px-2 py-1 text-[11px] text-muted">{t(`governance.options.kind.${item.governance.kind}`)}</span>
                  <span className="text-[11px] text-faint">{t("registry.suiteCount", { count: item.suite_ids.length })}</span>
                  <button onClick={() => beginSkillEdit(item)} className="rounded-lg p-2 text-muted hover:bg-surface-hover"><Pencil className="h-3.5 w-3.5" /></button>
                </div>
              ))}
            </div>
          </section>
        )}

        {snapshot && tab === "suites" && (
          <section className="space-y-3">
            <div className="flex items-center justify-between gap-3"><p className="text-[12.5px] text-muted">{t("registry.suiteRule")}</p><button onClick={() => beginSuiteEdit()} className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-[12px] font-medium text-white"><Plus className="h-3.5 w-3.5" />{t("registry.newSuite")}</button></div>
            <div className="flex justify-end"><select value={selectedAgent} onChange={(event) => setSelectedAgent(event.target.value)} className={inputClass}><option value="">{t("registry.selectAgent")}</option>{installedAgents.map((tool) => <option key={tool.key} value={tool.key}>{tool.display_name}</option>)}</select></div>
            <div className="grid gap-3 md:grid-cols-2">
              {snapshot.suites.map((suite) => {
                const memberNames = suite.members.map((member) => snapshot.skills.find((item) => item.skill.id === member.skill_id)?.skill.name ?? member.skill_id);
                const deployed = Boolean(selectedAgent) && suite.members.every((member) => snapshot.skills.find((item) => item.skill.id === member.skill_id)?.targets.some((target) => target.tool === selectedAgent));
                return <article key={suite.id} className="rounded-xl border border-border-subtle bg-surface p-4"><div className="flex items-start justify-between gap-3"><div><h3 className="text-[14px] font-semibold text-secondary">{suite.name}</h3><p className="mt-1 text-[11.5px] text-muted">{suite.category}{suite.version ? ` · ${suite.version}` : ""}</p></div><div className="flex items-center"><button onClick={() => beginSuiteEdit(suite)} className="rounded-lg p-2 text-muted hover:bg-surface-hover"><Pencil className="h-3.5 w-3.5" /></button><button disabled={saving} onClick={() => void removeSuite(suite)} className="rounded-lg p-2 text-muted hover:bg-red-500/10 hover:text-red-500"><Trash2 className="h-3.5 w-3.5" /></button></div></div><div className="mt-3 flex flex-wrap gap-1">{suite.tags.map((value) => <span key={value} className="rounded bg-background px-1.5 py-0.5 text-[10.5px] text-muted">{value}</span>)}</div><div className="mt-3 text-[12px] text-tertiary">{memberNames.join(" · ")}</div><div className="mt-4 flex items-center justify-between"><span className={cn("text-[11.5px]", deployed ? "text-emerald-500" : "text-faint")}>{deployed ? t("registry.deployed") : t("registry.notDeployed")}</span><button disabled={!selectedAgent || saving} onClick={() => void setSuiteState(suite, !deployed)} className="rounded-lg border border-border-subtle px-3 py-1.5 text-[11.5px] text-secondary disabled:opacity-40">{t(deployed ? "registry.undeployWhole" : "registry.deployWhole")}</button></div></article>;
              })}
            </div>
          </section>
        )}

        {snapshot && tab === "resources" && <section className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">{snapshot.resources.map((resource) => <article key={`${resource.resource_type}:${resource.name}`} className="rounded-xl border border-border-subtle bg-surface p-4"><div className="flex items-center gap-2"><Wrench className="h-4 w-4 text-accent" /><h3 className="text-[13px] font-medium text-secondary">{resource.name}</h3></div><p className="mt-2 text-[11.5px] uppercase tracking-wide text-faint">{resource.resource_type}</p><p className="mt-2 text-[12px] text-muted">{t("registry.requiredBy", { count: resource.required_by })}</p></article>)}</section>}

        {snapshot && tab === "history" && <section className="overflow-hidden rounded-xl border border-border-subtle bg-surface"><div className="border-b border-border-subtle px-4 py-3 text-[11px] text-faint">{snapshot.history_path}</div>{snapshot.history.map((entry) => <div key={`${entry.ts}-${entry.id}`} className="flex items-start gap-3 border-b border-border-subtle px-4 py-3 last:border-b-0"><span className={cn("mt-1 h-2 w-2 rounded-full", entry.success ? "bg-emerald-500" : "bg-red-500")} /><div className="min-w-0 flex-1"><div className="text-[12.5px] font-medium text-secondary">{entry.action}{entry.skill_name ? ` · ${entry.skill_name}` : ""}</div><div className="mt-1 break-all text-[11.5px] text-muted">{entry.detail || entry.tool || "—"}</div></div><time className="text-[11px] text-faint">{new Date(entry.ts * 1000).toLocaleString()}</time></div>)}</section>}
        {snapshot && tab === "matrix" && (() => {
          const columns = installedAgents;
          const PAGE = 40;
          const totalPages = Math.max(1, Math.ceil(snapshot.skills.length / PAGE));
          const page = Math.min(matrixPage, totalPages - 1);
          const rowsPage = snapshot.skills.slice(page * PAGE, (page + 1) * PAGE);
          const toggleCell = async (skillId: string, tool: string, deployed: boolean) => {
            setMatrixBusy(`${skillId}:${tool}`);
            try {
              if (deployed) await unsyncSkillFromTool(skillId, tool);
              else await syncSkillToTool(skillId, tool);
              await load();
            } catch (error) {
              toast.error(getErrorMessage(error, t("common.error")));
            } finally {
              setMatrixBusy(null);
            }
          };
          return (
            <section className="overflow-hidden rounded-xl border border-border-subtle bg-surface">
              <div className="overflow-x-auto">
                <table className="w-full border-collapse text-[12px]">
                  <thead>
                    <tr className="border-b border-border-subtle">
                      <th className="sticky left-0 z-10 bg-surface px-3 py-2 text-left font-medium text-muted">{t("registry.matrixSkill")}</th>
                      {columns.map((tool) => (
                        <th key={tool.key} className="px-2 py-2" title={tool.display_name}>
                          <div className="flex flex-col items-center gap-1">
                            <AgentIcon agentKey={tool.key} displayName={tool.display_name} className="h-5 w-5 rounded-[4px]" />
                          </div>
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {rowsPage.map((item) => (
                      <tr key={item.skill.id} className="border-b border-border-subtle last:border-b-0 hover:bg-surface-hover/50">
                        <td className="sticky left-0 z-10 bg-surface px-3 py-1.5">
                          <span className="block max-w-[220px] truncate text-secondary" title={item.skill.name}>{item.skill.name}</span>
                        </td>
                        {columns.map((tool) => {
                          const deployed = item.targets.some((target) => target.tool === tool.key);
                          const busyKey = `${item.skill.id}:${tool.key}`;
                          return (
                            <td key={tool.key} className="px-2 py-1.5 text-center">
                              <button
                                onClick={() => void toggleCell(item.skill.id, tool.key, deployed)}
                                disabled={matrixBusy === busyKey}
                                title={`${item.skill.name} · ${tool.display_name} — ${deployed ? t("registry.matrixUndeploy") : t("registry.matrixDeploy")}`}
                                className={cn(
                                  "inline-flex h-4 w-4 items-center justify-center rounded-full border transition-colors",
                                  deployed
                                    ? "border-emerald-500/40 bg-emerald-500/70 hover:bg-emerald-500"
                                    : "border-border-subtle bg-transparent hover:border-emerald-500/40",
                                  matrixBusy === busyKey && "animate-pulse"
                                )}
                              />
                            </td>
                          );
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="flex items-center justify-between border-t border-border-subtle px-4 py-2 text-[11.5px] text-muted">
                <span>{t("registry.matrixSummary", { count: snapshot.skills.length, agents: columns.length })}</span>
                <span className="flex items-center gap-2">
                  <button onClick={() => setMatrixPage(Math.max(0, page - 1))} disabled={page === 0} className="rounded px-2 py-0.5 hover:bg-surface-hover disabled:opacity-40">{t("registry.matrixPrev")}</button>
                  <span className="tabular-nums">{page + 1} / {totalPages}</span>
                  <button onClick={() => setMatrixPage(Math.min(totalPages - 1, page + 1))} disabled={page >= totalPages - 1} className="rounded px-2 py-0.5 hover:bg-surface-hover disabled:opacity-40">{t("registry.matrixNext")}</button>
                </span>
              </div>
            </section>
          );
        })()}
      </div>

      {editingSkill && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"><div className="w-full max-w-lg rounded-2xl border border-border bg-surface p-5 shadow-2xl"><div className="flex items-center justify-between"><h2 className="text-[15px] font-semibold text-primary">{editingSkill.skill.name}</h2><button onClick={() => setEditingSkill(null)}><X className="h-4 w-4 text-muted" /></button></div><div className="mt-4 grid gap-3"><label className="text-[12px] text-muted">{t("registry.category")}<select value={skillKind} onChange={(event) => setSkillKind(event.target.value as SkillKind)} className={`${inputClass} mt-1 w-full`}>{KINDS.map((value) => <option key={value} value={value}>{t(`governance.options.kind.${value}`)}</option>)}</select></label><label className="text-[12px] text-muted">{t("registry.tagsComma")}<input value={skillTags} onChange={(event) => setSkillTagsDraft(event.target.value)} className={`${inputClass} mt-1 w-full`} /></label></div><div className="mt-5 flex justify-end"><button disabled={saving} onClick={() => void saveSkillClassification()} className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-[12px] text-white"><Save className="h-3.5 w-3.5" />{t("common.save")}</button></div></div></div>}

      {suiteDraft && snapshot && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"><div className="max-h-[88vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-border bg-surface p-5 shadow-2xl"><div className="flex items-center justify-between"><h2 className="text-[15px] font-semibold text-primary">{t(suiteDraft.id ? "registry.editSuite" : "registry.newSuite")}</h2><button onClick={() => setSuiteDraft(null)}><X className="h-4 w-4 text-muted" /></button></div><div className="mt-4 grid gap-3 md:grid-cols-2"><input value={suiteDraft.name} onChange={(event) => setSuiteDraft({ ...suiteDraft, name: event.target.value })} placeholder={t("registry.suiteName")} className={inputClass} /><input value={suiteDraft.version ?? ""} onChange={(event) => setSuiteDraft({ ...suiteDraft, version: event.target.value || null })} placeholder={t("registry.version")} className={inputClass} /><input value={suiteDraft.category} onChange={(event) => setSuiteDraft({ ...suiteDraft, category: event.target.value })} placeholder={t("registry.category")} className={inputClass} /><input value={suiteDraft.tags.join(", ")} onChange={(event) => setSuiteDraft({ ...suiteDraft, tags: event.target.value.split(",").map((value) => value.trim()).filter(Boolean) })} placeholder={t("registry.tagsComma")} className={inputClass} /><textarea value={suiteDraft.description ?? ""} onChange={(event) => setSuiteDraft({ ...suiteDraft, description: event.target.value || null })} placeholder={t("registry.description")} className={`${inputClass} min-h-20 md:col-span-2`} /></div><h3 className="mt-5 flex items-center gap-2 text-[12.5px] font-semibold text-secondary"><Tag className="h-3.5 w-3.5" />{t("registry.membersWhole")}</h3><div className="mt-2 grid max-h-64 gap-2 overflow-y-auto md:grid-cols-2">{snapshot.skills.map((item) => { const checked = suiteDraft.members.some((member) => member.skill_id === item.skill.id); return <label key={item.skill.id} className={cn("flex items-center gap-2 rounded-lg border px-3 py-2 text-[12px]", checked ? "border-accent/40 bg-accent/10 text-secondary" : "border-border-subtle text-muted")}><input type="checkbox" checked={checked} onChange={() => toggleSuiteMember(item.skill.id)} />{item.skill.name}</label>; })}</div><h3 className="mt-5 flex items-center gap-2 text-[12.5px] font-semibold text-secondary"><PackagePlus className="h-3.5 w-3.5" />{t("registry.suiteAssets")}</h3><p className="mt-1 text-[11.5px] text-faint">{t("registry.suiteAssetsHint")}</p>{(suiteDraft.assets ?? []).map((asset, index) => (<div key={index} className="mt-2 grid gap-2 md:grid-cols-[110px_1fr_1fr_auto]"><select value={asset.asset_type} onChange={(event) => setSuiteDraft({ ...suiteDraft, assets: suiteDraft.assets.map((item, i) => i === index ? { ...item, asset_type: event.target.value } : item) })} className={inputClass}>{SUITE_ASSET_TYPES.map((type) => <option key={type} value={type}>{t(`registry.assetType.${type}`)}</option>)}</select><input value={asset.name} onChange={(event) => setSuiteDraft({ ...suiteDraft, assets: suiteDraft.assets.map((item, i) => i === index ? { ...item, name: event.target.value } : item) })} placeholder={t("registry.assetName")} className={inputClass} /><input value={asset.path} onChange={(event) => setSuiteDraft({ ...suiteDraft, assets: suiteDraft.assets.map((item, i) => i === index ? { ...item, path: event.target.value } : item) })} placeholder={t("registry.assetPath")} className={inputClass} /><button onClick={() => setSuiteDraft({ ...suiteDraft, assets: suiteDraft.assets.filter((_, i) => i !== index) })} className="rounded-lg border border-border-subtle px-2 text-muted hover:text-red-500"><X className="h-3.5 w-3.5" /></button></div>))}<button onClick={() => setSuiteDraft({ ...suiteDraft, assets: [...(suiteDraft.assets ?? []), { asset_type: "command", name: "", path: "", tool: null, notes: null, sort_order: suiteDraft.assets?.length ?? 0 }] })} className="mt-2 inline-flex items-center gap-1 rounded-lg border border-border-subtle px-2.5 py-1.5 text-[12px] text-secondary hover:bg-surface-hover"><Plus className="h-3.5 w-3.5" />{t("registry.addAsset")}</button><div className="mt-5 flex justify-end"><button disabled={saving} onClick={() => void persistSuite()} className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-[12px] text-white"><Save className="h-3.5 w-3.5" />{t("common.save")}</button></div></div></div>}
    </div>
  );
}
