// Curated, quality-first discovery sources (REQUIREMENTS v2.1 §5).
// The market (skills.sh) ranks purely by install telemetry with no human
// curation, so the install page defaults to hiding low-signal entries:
// anything from an owner below this bar is shown only after the user
// explicitly expands the hidden set. Owners listed here come from the
// official-team sections of community curated lists (VoltAgent
// awesome-agent-skills et al.).
export const TRUSTED_OWNERS: ReadonlySet<string> = new Set([
  "anthropics",
  "microsoft",
  "openai",
  "google",
  "cloudflare",
  "stripe",
  "trailofbits",
  "vercel-labs",
  "github",
]);

export const MARKET_MIN_INSTALLS = 500;

export interface TrustedSource {
  repo: string;
  label: string;
  note: string;
}

// Quick-install shortcuts rendered on the Git tab. Each entry points at a
// repository the user has reason to trust; clicking fills the install form.
export const TRUSTED_SOURCES: readonly TrustedSource[] = [
  {
    repo: "anthropics/skills",
    label: "Anthropic Official",
    note: "docx/pdf/pptx/xlsx 文档技能与参考实现，量少质精",
  },
  {
    repo: "vercel-labs/skills",
    label: "Vercel Labs",
    note: "skills.sh 官方仓库，find-skills 等工具技能",
  },
  {
    repo: "cloudflare/agents",
    label: "Cloudflare",
    note: "官方 agent 生态与模板",
  },
  {
    repo: "stripe/stripe-agent-toolkit",
    label: "Stripe",
    note: "支付与 API 集成工具",
  },
];

export function isTrustedSource(source: string): boolean {
  const owner = source.split("/")[0];
  return TRUSTED_OWNERS.has(owner);
}

export function passesQualityBar(source: string, installs: number): boolean {
  return isTrustedSource(source) || installs >= MARKET_MIN_INSTALLS;
}
