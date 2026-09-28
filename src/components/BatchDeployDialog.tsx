import { useEffect, useState } from "react";
import { X, ArrowDownToLine, ArrowUpFromLine, CheckSquare, Square } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../utils";
import { AgentIcon } from "./AgentIcon";

export interface BatchDeployAgent {
  key: string;
  displayName: string;
}

interface Props {
  open: boolean;
  skillCount: number;
  agents: BatchDeployAgent[];
  onClose: () => void;
  onApply: (toolKeys: string[], mode: "add" | "remove") => Promise<void>;
}

/// Bulk deploy dialog for the MySkills multi-select toolbar: pick target
/// agents, choose deploy or undeploy, confirm. The backend applies every
/// (skill, agent) pair as one planned batch — it refuses the whole batch
/// rather than half-applying when a target holds unmanaged content.
export function BatchDeployDialog({ open, skillCount, agents, onClose, onApply }: Props) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<string[]>([]);
  const [mode, setMode] = useState<"add" | "remove">("add");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (open) {
      setSelected([]);
      setMode("add");
    }
  }, [open]);

  if (!open) return null;

  const toggleAgent = (key: string) => {
    setSelected((prev) =>
      prev.includes(key) ? prev.filter((k) => k !== key) : [...prev, key]
    );
  };

  const handleApply = async () => {
    if (selected.length === 0) {
      onClose();
      return;
    }
    setLoading(true);
    try {
      await onApply(selected, mode);
      onClose();
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-black/70 backdrop-blur-sm" onClick={onClose} />
      <div className="relative bg-surface border border-border rounded-xl w-full max-w-[440px] p-5 shadow-2xl">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-[13px] font-semibold text-primary flex items-center gap-2">
            <ArrowDownToLine className="w-4 h-4 text-accent-light" />
            {t("mySkills.batchDeployDialog.title", { count: skillCount })}
          </h2>
          <button
            onClick={onClose}
            className="text-muted hover:text-secondary p-1 rounded transition-colors outline-none"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="mb-3 flex gap-1 rounded-lg border border-border-subtle p-0.5">
          {(["add", "remove"] as const).map((value) => (
            <button
              key={value}
              onClick={() => setMode(value)}
              className={cn(
                "flex-1 inline-flex items-center justify-center gap-1.5 rounded-md px-2 py-1 text-[12px] font-medium transition-colors",
                mode === value
                  ? "bg-accent-bg text-accent-light"
                  : "text-muted hover:text-secondary"
              )}
            >
              {value === "add" ? (
                <ArrowDownToLine className="h-3.5 w-3.5" />
              ) : (
                <ArrowUpFromLine className="h-3.5 w-3.5" />
              )}
              {t(`mySkills.batchDeployDialog.mode.${value}`)}
            </button>
          ))}
        </div>

        <div className="max-h-[320px] overflow-y-auto rounded-lg border border-border-subtle p-1.5">
          {agents.length === 0 ? (
            <p className="p-3 text-[12px] text-faint">
              {t("mySkills.batchDeployDialog.noAgents")}
            </p>
          ) : (
            agents.map((agent) => {
              const checked = selected.includes(agent.key);
              return (
                <button
                  key={agent.key}
                  onClick={() => toggleAgent(agent.key)}
                  className={cn(
                    "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-[13px] transition-colors",
                    checked ? "bg-accent-bg" : "hover:bg-surface-hover"
                  )}
                >
                  {checked ? (
                    <CheckSquare className="h-4 w-4 shrink-0 text-accent-light" />
                  ) : (
                    <Square className="h-4 w-4 shrink-0 text-faint" />
                  )}
                  <AgentIcon agentKey={agent.key} className="h-4 w-4 shrink-0" />
                  <span className="truncate text-secondary">{agent.displayName}</span>
                </button>
              );
            })
          )}
        </div>

        <p className="mt-2 text-[12px] text-faint">
          {t("mySkills.batchDeployDialog.hint")}
        </p>

        <div className="flex justify-end gap-2 pt-4">
          <button
            onClick={onClose}
            className="px-3 py-1.5 rounded-lg text-[13px] font-medium text-tertiary hover:text-secondary hover:bg-surface-hover transition-colors outline-none"
          >
            {t("common.cancel")}
          </button>
          <button
            onClick={handleApply}
            disabled={loading || selected.length === 0}
            className={cn(
              "px-3 py-1.5 rounded-lg text-white text-[13px] font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed border outline-none",
              mode === "remove"
                ? "bg-red-600/90 hover:bg-red-500 border-red-400/40"
                : "bg-accent-dark hover:bg-accent border-accent-border"
            )}
          >
            {loading
              ? t("common.loading")
              : t(mode === "remove"
                  ? "mySkills.batchDeployDialog.removeConfirm"
                  : "mySkills.batchDeployDialog.addConfirm", { count: skillCount, agents: selected.length })}
          </button>
        </div>
      </div>
    </div>
  );
}
