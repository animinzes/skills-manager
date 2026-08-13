import { useEffect, useMemo, useState, type ReactNode } from "react";
import { Loader2, Pencil, Plus, Save, ShieldCheck, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { getErrorMessage } from "../lib/error";
import {
  getSkillGovernance,
  saveSkillGovernance,
  type AccountAccess,
  type AgentSupportLevel,
  type BinaryAccess,
  type FilesystemAccess,
  type SkillDependency,
  type SkillDependencyType,
  type SkillGovernanceInput,
  type SkillGovernanceProfile,
  type SkillKind,
  type SkillLifecycle,
  type SkillProvenance,
  type SkillRiskLevel,
  type ToolInfo,
} from "../lib/tauri";
import { AgentIcon } from "./AgentIcon";

interface Props {
  skillId: string;
  tools: ToolInfo[];
}

const KINDS: SkillKind[] = ["capability", "tool_guide", "integration", "workflow", "governance"];
const PROVENANCE: SkillProvenance[] = ["unknown", "official", "suite", "third_party", "local"];
const LIFECYCLES: SkillLifecycle[] = ["experimental", "active", "deprecated", "archived"];
const RISKS: SkillRiskLevel[] = ["unreviewed", "low", "medium", "high"];
const SUPPORT_LEVELS: AgentSupportLevel[] = ["unknown", "supported", "partial", "unsupported"];
const DEPENDENCY_TYPES: SkillDependencyType[] = ["cli", "mcp", "runtime", "service", "skill", "config"];
const FILE_ACCESS: FilesystemAccess[] = ["unknown", "none", "read", "write"];
const BINARY_ACCESS: BinaryAccess[] = ["unknown", "none", "required"];
const ACCOUNT_ACCESS: AccountAccess[] = ["unknown", "none", "read", "write"];

const fieldClass =
  "w-full rounded-lg border border-border-subtle bg-background px-2.5 py-2 text-[12.5px] text-secondary outline-none transition-colors focus:border-border";

function toInput(profile: SkillGovernanceProfile): SkillGovernanceInput {
  return structuredClone({
    skill_id: profile.skill_id,
    kind: profile.kind,
    provenance: profile.provenance,
    owner: profile.owner,
    maintainer: profile.maintainer,
    lifecycle: profile.lifecycle,
    risk_level: profile.risk_level,
    supported_agents: profile.supported_agents,
    dependencies: profile.dependencies,
    permissions: profile.permissions,
    notes: profile.notes,
  });
}

function emptyDependency(): SkillDependency {
  return {
    dependency_type: "cli",
    name: "",
    version_requirement: null,
    required: true,
    check_command: null,
  };
}

export function SkillGovernanceSection({ skillId, tools }: Props) {
  const { t } = useTranslation();
  const [profile, setProfile] = useState<SkillGovernanceProfile | null>(null);
  const [draft, setDraft] = useState<SkillGovernanceInput | null>(null);
  const [loading, setLoading] = useState(true);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    setLoading(true);
    getSkillGovernance(skillId)
      .then((nextProfile) => {
        if (!active) return;
        setProfile(nextProfile);
        setDraft(toInput(nextProfile));
      })
      .catch((error) => {
        if (active) toast.error(getErrorMessage(error, t("common.error")));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [skillId, t]);

  const supportByAgent = useMemo(
    () => new Map((draft?.supported_agents ?? []).map((item) => [item.agent_key, item.support_level])),
    [draft?.supported_agents]
  );

  const startEditing = () => {
    if (!profile) return;
    setDraft(toInput(profile));
    setEditing(true);
  };

  const cancelEditing = () => {
    if (profile) setDraft(toInput(profile));
    setEditing(false);
  };

  const save = async () => {
    if (!draft || draft.dependencies.some((dependency) => !dependency.name.trim())) {
      toast.error(t("governance.dependencyNameRequired"));
      return;
    }
    setSaving(true);
    try {
      const saved = await saveSkillGovernance(draft);
      setProfile(saved);
      setDraft(toInput(saved));
      setEditing(false);
      toast.success(t("governance.saved"));
    } catch (error) {
      toast.error(getErrorMessage(error, t("common.error")));
    } finally {
      setSaving(false);
    }
  };

  const setSupport = (agentKey: string, supportLevel: AgentSupportLevel) => {
    if (!draft) return;
    const others = draft.supported_agents.filter((item) => item.agent_key !== agentKey);
    setDraft({
      ...draft,
      supported_agents:
        supportLevel === "unknown"
          ? others
          : [...others, { agent_key: agentKey, support_level: supportLevel, notes: null }],
    });
  };

  const updateDependency = (index: number, patch: Partial<SkillDependency>) => {
    if (!draft) return;
    setDraft({
      ...draft,
      dependencies: draft.dependencies.map((dependency, currentIndex) =>
        currentIndex === index ? { ...dependency, ...patch } : dependency
      ),
    });
  };

  if (loading) {
    return (
      <div className="mb-4 flex items-center gap-2 rounded-xl border border-border-subtle px-4 py-3 text-[12.5px] text-muted">
        <Loader2 className="h-3.5 w-3.5 animate-spin" />
        {t("governance.loading")}
      </div>
    );
  }

  if (!profile || !draft) return null;

  return (
    <section className="mb-4 rounded-xl border border-border-subtle bg-surface/50">
      <header className="flex items-center justify-between gap-3 border-b border-border-subtle px-4 py-3">
        <div className="flex min-w-0 items-center gap-2">
          <ShieldCheck className="h-4 w-4 shrink-0 text-accent" />
          <div>
            <h3 className="text-[13px] font-semibold text-secondary">{t("governance.title")}</h3>
            <p className="text-[11.5px] text-muted">{t("governance.subtitle")}</p>
          </div>
        </div>
        {editing ? (
          <div className="flex items-center gap-1.5">
            <button type="button" onClick={cancelEditing} className="rounded-lg p-2 text-muted hover:bg-surface-hover hover:text-secondary" title={t("common.cancel")}>
              <X className="h-3.5 w-3.5" />
            </button>
            <button type="button" onClick={save} disabled={saving} className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-1.5 text-[12px] font-medium text-white disabled:opacity-50">
              {saving ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Save className="h-3.5 w-3.5" />}
              {t("common.save")}
            </button>
          </div>
        ) : (
          <button type="button" onClick={startEditing} className="inline-flex items-center gap-1.5 rounded-lg border border-border-subtle px-2.5 py-1.5 text-[12px] text-muted hover:bg-surface-hover hover:text-secondary">
            <Pencil className="h-3.5 w-3.5" />
            {t("governance.edit")}
          </button>
        )}
      </header>

      {!editing ? (
        <div className="space-y-3 px-4 py-3">
          <div className="flex flex-wrap gap-1.5">
            {[profile.kind, profile.provenance, profile.lifecycle, profile.risk_level].map((value, index) => {
              const groups = ["kind", "provenance", "lifecycle", "risk"];
              return <span key={`${groups[index]}-${value}`} className="rounded-full border border-border-subtle bg-background px-2 py-0.5 text-[11.5px] text-secondary">{t(`governance.options.${groups[index]}.${value}`)}</span>;
            })}
          </div>
          {(profile.owner || profile.maintainer) && (
            <div className="grid gap-2 text-[12px] md:grid-cols-2">
              {profile.owner && <SummaryItem label={t("governance.owner")} value={profile.owner} />}
              {profile.maintainer && <SummaryItem label={t("governance.maintainer")} value={profile.maintainer} />}
            </div>
          )}
          <p className="text-[12px] text-muted">
            {t("governance.summary", {
              agents: profile.supported_agents.length,
              dependencies: profile.dependencies.length,
            })}
          </p>
          {profile.notes && <p className="whitespace-pre-wrap text-[12px] text-secondary">{profile.notes}</p>}
        </div>
      ) : (
        <div className="space-y-5 px-4 py-4">
          <div className="grid gap-3 md:grid-cols-2">
            <SelectField label={t("governance.kind")} value={draft.kind} options={KINDS} optionGroup="kind" onChange={(value) => setDraft({ ...draft, kind: value as SkillKind })} />
            <SelectField label={t("governance.provenance")} value={draft.provenance} options={PROVENANCE} optionGroup="provenance" onChange={(value) => setDraft({ ...draft, provenance: value as SkillProvenance })} />
            <SelectField label={t("governance.lifecycle")} value={draft.lifecycle} options={LIFECYCLES} optionGroup="lifecycle" onChange={(value) => setDraft({ ...draft, lifecycle: value as SkillLifecycle })} />
            <SelectField label={t("governance.risk")} value={draft.risk_level} options={RISKS} optionGroup="risk" onChange={(value) => setDraft({ ...draft, risk_level: value as SkillRiskLevel })} />
            <TextField label={t("governance.owner")} value={draft.owner ?? ""} onChange={(value) => setDraft({ ...draft, owner: value || null })} />
            <TextField label={t("governance.maintainer")} value={draft.maintainer ?? ""} onChange={(value) => setDraft({ ...draft, maintainer: value || null })} />
          </div>

          <EditorGroup title={t("governance.permissions")}>
            <div className="grid gap-3 md:grid-cols-2">
              <SelectField label={t("governance.filesystemAccess")} value={draft.permissions.filesystem_access} options={FILE_ACCESS} optionGroup="access" onChange={(value) => setDraft({ ...draft, permissions: { ...draft.permissions, filesystem_access: value as FilesystemAccess } })} />
              <SelectField label={t("governance.networkAccess")} value={draft.permissions.network_access} options={BINARY_ACCESS} optionGroup="access" onChange={(value) => setDraft({ ...draft, permissions: { ...draft.permissions, network_access: value as BinaryAccess } })} />
              <SelectField label={t("governance.commandExecution")} value={draft.permissions.command_execution} options={BINARY_ACCESS} optionGroup="access" onChange={(value) => setDraft({ ...draft, permissions: { ...draft.permissions, command_execution: value as BinaryAccess } })} />
              <SelectField label={t("governance.accountAccess")} value={draft.permissions.account_access} options={ACCOUNT_ACCESS} optionGroup="access" onChange={(value) => setDraft({ ...draft, permissions: { ...draft.permissions, account_access: value as AccountAccess } })} />
              <SelectField label={t("governance.secretsAccess")} value={draft.permissions.secrets_access} options={BINARY_ACCESS} optionGroup="access" onChange={(value) => setDraft({ ...draft, permissions: { ...draft.permissions, secrets_access: value as BinaryAccess } })} />
            </div>
          </EditorGroup>

          <EditorGroup title={t("governance.agentSupport")}>
            <div className="grid gap-2 md:grid-cols-2">
              {tools.map((tool) => (
                <div key={tool.key} className="flex items-center gap-2 rounded-lg border border-border-subtle bg-background px-2.5 py-2">
                  <AgentIcon agentKey={tool.key} displayName={tool.display_name} className="h-5 w-5 rounded" />
                  <span className="min-w-0 flex-1 truncate text-[12px] text-secondary">{tool.display_name}</span>
                  <select className="rounded-md border border-border-subtle bg-surface px-2 py-1 text-[11.5px] text-secondary" value={supportByAgent.get(tool.key) ?? "unknown"} onChange={(event) => setSupport(tool.key, event.target.value as AgentSupportLevel)}>
                    {SUPPORT_LEVELS.map((level) => <option key={level} value={level}>{t(`governance.options.support.${level}`)}</option>)}
                  </select>
                </div>
              ))}
            </div>
          </EditorGroup>

          <EditorGroup title={t("governance.dependencies")} action={(
            <button type="button" onClick={() => setDraft({ ...draft, dependencies: [...draft.dependencies, emptyDependency()] })} className="inline-flex items-center gap-1 text-[11.5px] text-accent hover:underline">
              <Plus className="h-3 w-3" />{t("governance.addDependency")}
            </button>
          )}>
            {draft.dependencies.length === 0 ? (
              <p className="text-[12px] text-muted">{t("governance.noDependencies")}</p>
            ) : (
              <div className="space-y-2">
                {draft.dependencies.map((dependency, index) => (
                  <div key={`${index}-${dependency.dependency_type}`} className="grid gap-2 rounded-lg border border-border-subtle bg-background p-2.5 md:grid-cols-[110px_1fr_120px_auto]">
                    <select className={fieldClass} value={dependency.dependency_type} onChange={(event) => updateDependency(index, { dependency_type: event.target.value as SkillDependencyType })}>
                      {DEPENDENCY_TYPES.map((type) => <option key={type} value={type}>{t(`governance.options.dependency.${type}`)}</option>)}
                    </select>
                    <input className={fieldClass} value={dependency.name} placeholder={t("governance.dependencyName")} onChange={(event) => updateDependency(index, { name: event.target.value })} />
                    <input className={fieldClass} value={dependency.version_requirement ?? ""} placeholder={t("governance.versionRequirement")} onChange={(event) => updateDependency(index, { version_requirement: event.target.value || null })} />
                    <button type="button" onClick={() => setDraft({ ...draft, dependencies: draft.dependencies.filter((_, currentIndex) => currentIndex !== index) })} className="rounded-lg p-2 text-muted hover:bg-red-500/10 hover:text-red-500" title={t("common.delete")}><Trash2 className="h-3.5 w-3.5" /></button>
                    <input className={`${fieldClass} md:col-span-3`} value={dependency.check_command ?? ""} placeholder={t("governance.checkCommand")} onChange={(event) => updateDependency(index, { check_command: event.target.value || null })} />
                    <label className="flex items-center gap-1.5 text-[11.5px] text-muted"><input type="checkbox" checked={dependency.required} onChange={(event) => updateDependency(index, { required: event.target.checked })} />{t("governance.required")}</label>
                  </div>
                ))}
              </div>
            )}
          </EditorGroup>

          <label className="block">
            <span className="mb-1 block text-[12px] font-medium text-tertiary">{t("governance.notes")}</span>
            <textarea className={`${fieldClass} min-h-20 resize-y`} value={draft.notes ?? ""} onChange={(event) => setDraft({ ...draft, notes: event.target.value || null })} />
          </label>
        </div>
      )}
    </section>
  );
}

function SummaryItem({ label, value }: { label: string; value: string }) {
  return <div><span className="text-faint">{label}: </span><span className="text-secondary">{value}</span></div>;
}

function EditorGroup({ title, action, children }: { title: string; action?: ReactNode; children: ReactNode }) {
  return <div><div className="mb-2 flex items-center justify-between"><h4 className="text-[12px] font-semibold text-secondary">{title}</h4>{action}</div>{children}</div>;
}

function TextField({ label, value, onChange }: { label: string; value: string; onChange: (value: string) => void }) {
  return <label><span className="mb-1 block text-[12px] font-medium text-tertiary">{label}</span><input className={fieldClass} value={value} onChange={(event) => onChange(event.target.value)} /></label>;
}

function SelectField({ label, value, options, optionGroup, onChange }: { label: string; value: string; options: readonly string[]; optionGroup: string; onChange: (value: string) => void }) {
  const { t } = useTranslation();
  return <label><span className="mb-1 block text-[12px] font-medium text-tertiary">{label}</span><select className={fieldClass} value={value} onChange={(event) => onChange(event.target.value)}>{options.map((option) => <option key={option} value={option}>{t(`governance.options.${optionGroup}.${option}`)}</option>)}</select></label>;
}
