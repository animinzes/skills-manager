import { useCallback, useEffect, useMemo, useState } from "react";
import { Boxes, History, Loader2, Package, Pencil, Plus, RefreshCw, Save, Search, Tag, Trash2, Wrench, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useApp } from "../context/AppContext";
import { getErrorMessage } from "../lib/error";
import {
  getRegistry,
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
} from "../lib/tauri";
import { cn } from "../utils";

type Tab = "skills" | "suites" | "resources" | "history";

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
      </div>

      {editingSkill && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"><div className="w-full max-w-lg rounded-2xl border border-border bg-surface p-5 shadow-2xl"><div className="flex items-center justify-between"><h2 className="text-[15px] font-semibold text-primary">{editingSkill.skill.name}</h2><button onClick={() => setEditingSkill(null)}><X className="h-4 w-4 text-muted" /></button></div><div className="mt-4 grid gap-3"><label className="text-[12px] text-muted">{t("registry.category")}<select value={skillKind} onChange={(event) => setSkillKind(event.target.value as SkillKind)} className={`${inputClass} mt-1 w-full`}>{KINDS.map((value) => <option key={value} value={value}>{t(`governance.options.kind.${value}`)}</option>)}</select></label><label className="text-[12px] text-muted">{t("registry.tagsComma")}<input value={skillTags} onChange={(event) => setSkillTagsDraft(event.target.value)} className={`${inputClass} mt-1 w-full`} /></label></div><div className="mt-5 flex justify-end"><button disabled={saving} onClick={() => void saveSkillClassification()} className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-[12px] text-white"><Save className="h-3.5 w-3.5" />{t("common.save")}</button></div></div></div>}

      {suiteDraft && snapshot && <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4"><div className="max-h-[88vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-border bg-surface p-5 shadow-2xl"><div className="flex items-center justify-between"><h2 className="text-[15px] font-semibold text-primary">{t(suiteDraft.id ? "registry.editSuite" : "registry.newSuite")}</h2><button onClick={() => setSuiteDraft(null)}><X className="h-4 w-4 text-muted" /></button></div><div className="mt-4 grid gap-3 md:grid-cols-2"><input value={suiteDraft.name} onChange={(event) => setSuiteDraft({ ...suiteDraft, name: event.target.value })} placeholder={t("registry.suiteName")} className={inputClass} /><input value={suiteDraft.version ?? ""} onChange={(event) => setSuiteDraft({ ...suiteDraft, version: event.target.value || null })} placeholder={t("registry.version")} className={inputClass} /><input value={suiteDraft.category} onChange={(event) => setSuiteDraft({ ...suiteDraft, category: event.target.value })} placeholder={t("registry.category")} className={inputClass} /><input value={suiteDraft.tags.join(", ")} onChange={(event) => setSuiteDraft({ ...suiteDraft, tags: event.target.value.split(",").map((value) => value.trim()).filter(Boolean) })} placeholder={t("registry.tagsComma")} className={inputClass} /><textarea value={suiteDraft.description ?? ""} onChange={(event) => setSuiteDraft({ ...suiteDraft, description: event.target.value || null })} placeholder={t("registry.description")} className={`${inputClass} min-h-20 md:col-span-2`} /></div><h3 className="mt-5 flex items-center gap-2 text-[12.5px] font-semibold text-secondary"><Tag className="h-3.5 w-3.5" />{t("registry.membersWhole")}</h3><div className="mt-2 grid max-h-64 gap-2 overflow-y-auto md:grid-cols-2">{snapshot.skills.map((item) => { const checked = suiteDraft.members.some((member) => member.skill_id === item.skill.id); return <label key={item.skill.id} className={cn("flex items-center gap-2 rounded-lg border px-3 py-2 text-[12px]", checked ? "border-accent/40 bg-accent/10 text-secondary" : "border-border-subtle text-muted")}><input type="checkbox" checked={checked} onChange={() => toggleSuiteMember(item.skill.id)} />{item.skill.name}</label>; })}</div><div className="mt-5 flex justify-end"><button disabled={saving} onClick={() => void persistSuite()} className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-2 text-[12px] text-white"><Save className="h-3.5 w-3.5" />{t("common.save")}</button></div></div></div>}
    </div>
  );
}
