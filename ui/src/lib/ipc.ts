/**
 * Typed wrappers over the Tauri IPC commands.
 *
 * When the app runs inside Tauri, calls go to the Rust backend via `invoke`.
 * When it runs in a plain browser (e.g. `npm run dev` for UI work, or CI), it
 * falls back to an in-memory mock so the whole flow is exercisable without the
 * desktop shell. The command names and shapes mirror `src-tauri/src/commands`.
 */

export interface VaultCard {
  id: string;
  name: string;
  stage: VaultStage;
  students: number;
  chambers: number;
  hourBalance: number;
  lastActivity: string;
  /** Emoji/crest shown on the card and vault header (from customization). */
  icon?: string | null;
  /** Accent as a CSS RGB triple, e.g. "217 119 6" (from customization). */
  accent?: string | null;
}

export type VaultStage = "prospective" | "active" | "renewal" | "alumni";

export const STAGES: { id: VaultStage; label: string }[] = [
  { id: "prospective", label: "Prospective" },
  { id: "active", label: "Active" },
  { id: "renewal", label: "Renewal" },
  { id: "alumni", label: "Alumni" },
];

/** Preset accent swatches offered in the vault dialog (CSS RGB triples). */
export const ACCENT_SWATCHES: { id: string; label: string; rgb: string }[] = [
  { id: "forest", label: "Forest", rgb: "70 128 103" },
  { id: "amber", label: "Amber", rgb: "217 119 6" },
  { id: "emerald", label: "Emerald", rgb: "52 211 153" },
  { id: "cyan", label: "Cyan", rgb: "6 182 212" },
  { id: "violet", label: "Violet", rgb: "139 92 246" },
  { id: "rose", label: "Rose", rgb: "244 63 94" },
  { id: "gold", label: "Gold", rgb: "202 138 4" },
  { id: "sky", label: "Sky", rgb: "56 189 248" },
];

/** Emoji crests offered as quick-pick vault icons. */
export const VAULT_ICONS = ["🎓", "✒️", "🏛️", "🔬", "🌱", "🚀", "⭐", "📚", "🧭", "🗝️"];

/**
 * Starter templates: a named preset of plugins to enable when a vault is
 * created. "plugins" are manifest ids that also live in the Forge store; the
 * backend enables them into the new vault's composition.
 */
export interface VaultTemplate {
  id: string;
  label: string;
  description: string;
  plugins: string[];
}

export const VAULT_TEMPLATES: VaultTemplate[] = [
  { id: "blank", label: "Blank", description: "Empty vault — add plugins yourself.", plugins: [] },
  {
    id: "advising",
    label: "College Advising",
    description: "Essay Version Control + Application Timeline Weaver.",
    plugins: ["biospark.essay-version-control", "biospark.timeline-weaver"],
  },
  {
    id: "practice",
    label: "Full Practice",
    description: "Advising tools plus Recommendation Manager and Session Notes.",
    plugins: [
      "biospark.essay-version-control",
      "biospark.timeline-weaver",
      "biospark.recommendation-manager",
      "biospark.session-notes",
    ],
  },
];

export interface ChatReply {
  text: string;
  provider: string;
  model: string;
}

export interface EssayVersion {
  id: string;
  essayId: string;
  seq: number;
  message: string;
  body: string;
}

export interface Milestone {
  id: string;
  title: string;
  dueAt: string | null;
  done: boolean;
}

export interface BillingEntry {
  id: string;
  studentId: string | null;
  minutes: number;
  description: string;
  billedAt: string;
}

export interface BillingLedger {
  retainerMinutes: number;
  usedMinutes: number;
  balanceMinutes: number;
  entries: BillingEntry[];
}

// --- Plugin / Forge types (mirror the Rust manifest JSON) -------------------

export type FieldKind =
  | { type: "text" }
  | { type: "long_text" }
  | { type: "date" }
  | { type: "bool" }
  | { type: "select"; options: string[] };

export interface FieldSpec {
  key: string;
  label: string;
  kind: FieldKind;
  required: boolean;
}

export type PanelKind =
  | {
      type: "collection";
      collection: string;
      add_label: string;
      title_field: string;
      subtitle_field?: string | null;
      fields: FieldSpec[];
    }
  | { type: "note"; content: string };

export interface UiSchema {
  panels: { title: string; kind: PanelKind }[];
}

export type PluginKind =
  | { type: "native"; component: string }
  | { type: "declarative"; ui: UiSchema };

export type Pricing = { type: "free" } | { type: "paid"; tier: string; price_cents: number };

export interface PluginManifest {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;
  icon: string;
  category: string;
  scope: "vault" | "chamber";
  capabilities: string[];
  kind: PluginKind;
  default_layout: { w: number; h: number };
  config_schema: FieldSpec[];
  pricing: Pricing;
}

export type TrustLevel = "verified" | "unsigned" | "untrusted";

export interface AvailablePlugin {
  manifest: PluginManifest;
  trust: TrustLevel;
  source: "builtin" | "forge";
  installed: boolean;
}

export interface EnabledPlugin {
  plugin_id: string;
  settings: unknown;
  layout: { w: number; h: number };
  order: number;
}

export interface VaultCustomization {
  theme?: string | null;
  accent?: string | null;
  icon?: string | null;
}

export interface VaultComposition {
  customization: VaultCustomization;
  plugins: EnabledPlugin[];
}

export interface PluginRecord {
  id: string;
  data: Record<string, unknown>;
}

// --- Tauri detection --------------------------------------------------------

interface TauriGlobal {
  core?: { invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> };
}
function tauri(): TauriGlobal | undefined {
  return (globalThis as { __TAURI__?: TauriGlobal }).__TAURI__;
}
export function isTauri(): boolean {
  return tauri()?.core?.invoke !== undefined;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const t = tauri();
  if (t?.core?.invoke) return t.core.invoke<T>(cmd, args);
  return mock<T>(cmd, args);
}

// --- Public API -------------------------------------------------------------

export const api = {
  healthCheck: () => invoke<boolean>("health_check"),
  listVaults: () => invoke<VaultCard[]>("get_vault_hierarchy"),
  createVault: (
    name: string,
    stage: VaultStage,
    customization?: VaultCustomization,
    template?: string[],
  ) => invoke<VaultCard>("create_vault", { name, stage, customization, template }),
  moveVault: (id: string, stage: VaultStage) =>
    invoke<void>("move_vault", { id, stage }),
  chamberAiEnabled: (chamberId: string) =>
    invoke<boolean>("chamber_ai_enabled", { chamberId }),
  setChamberAi: (chamberId: string, enabled: boolean) =>
    invoke<void>("set_chamber_ai", { chamberId, enabled }),
  askQuill: (chamberId: string, prompt: string) =>
    invoke<ChatReply>("invoke_quantum_quill", { chamberId, prompt }),
  commitEssay: (vaultId: string, chamberId: string, essayId: string, message: string, body: string) =>
    invoke<EssayVersion>("commit_essay", { vaultId, chamberId, essayId, message, body }),
  essayHistory: (vaultId: string, essayId: string) =>
    invoke<EssayVersion[]>("essay_history", { vaultId, essayId }),
  addMilestone: (vaultId: string, chamberId: string, title: string, dueAt: string | null) =>
    invoke<Milestone>("add_milestone", { vaultId, chamberId, title, dueAt }),
  listMilestones: (vaultId: string, chamberId: string) =>
    invoke<Milestone[]>("list_milestones", { vaultId, chamberId }),
  setMilestoneDone: (vaultId: string, id: string, done: boolean) =>
    invoke<void>("set_milestone_done", { vaultId, id, done }),
  getBillingLedger: (vaultId: string) =>
    invoke<BillingLedger>("get_billing_ledger", { vaultId }),
  setRetainerMinutes: (vaultId: string, minutes: number) =>
    invoke<void>("set_retainer_minutes", { vaultId, minutes }),
  logBillingEntry: (
    vaultId: string,
    studentId: string | null,
    minutes: number,
    description: string,
    billedAt: string,
  ) => invoke<BillingEntry>("log_billing_entry", { vaultId, studentId, minutes, description, billedAt }),
  deleteBillingEntry: (vaultId: string, id: string) =>
    invoke<void>("delete_billing_entry", { vaultId, id }),

  // Forge / plugin system
  listAvailablePlugins: () => invoke<AvailablePlugin[]>("list_available_plugins"),
  installPlugin: (id: string) => invoke<void>("install_plugin", { id }),
  uninstallPlugin: (id: string) => invoke<void>("uninstall_plugin", { id }),
  getVaultComposition: (vaultId: string) =>
    invoke<VaultComposition>("get_vault_composition", { vaultId }),
  setVaultComposition: (vaultId: string, composition: VaultComposition) =>
    invoke<void>("set_vault_composition", { vaultId, composition }),
  pluginRecordAdd: (
    vaultId: string,
    chamberId: string | null,
    pluginId: string,
    collection: string,
    data: Record<string, unknown>,
  ) => invoke<PluginRecord>("plugin_record_add", { vaultId, chamberId, pluginId, collection, data }),
  pluginRecordList: (
    vaultId: string,
    chamberId: string | null,
    pluginId: string,
    collection: string,
  ) => invoke<PluginRecord[]>("plugin_record_list", { vaultId, chamberId, pluginId, collection }),
  pluginRecordDelete: (vaultId: string, id: string) =>
    invoke<void>("plugin_record_delete", { vaultId, id }),
};

// --- In-browser mock backend ------------------------------------------------

const mockVaults: VaultCard[] = [
  {
    id: "v-rivera",
    name: "Ms. Rivera — Class of 2027",
    stage: "active",
    students: 14,
    chambers: 14,
    hourBalance: 42,
    lastActivity: "2h ago",
    icon: "🎓",
    accent: "70 128 103",
  },
  {
    id: "v-okafor",
    name: "Dr. Okafor — STEM Cohort",
    stage: "active",
    students: 9,
    chambers: 9,
    hourBalance: 27,
    lastActivity: "yesterday",
    icon: "🔬",
    accent: "6 182 212",
  },
  {
    id: "v-lindqvist",
    name: "Lindqvist Advising",
    stage: "prospective",
    students: 3,
    chambers: 3,
    hourBalance: 6,
    lastActivity: "3d ago",
    icon: "🌱",
    accent: "52 211 153",
  },
  {
    id: "v-summit",
    name: "Summit Legacy Families",
    stage: "renewal",
    students: 21,
    chambers: 21,
    hourBalance: 88,
    lastActivity: "1w ago",
    icon: "🏛️",
    accent: "202 138 4",
  },
];

const mockAi: Record<string, boolean> = {};
const mockEssays: Record<string, EssayVersion[]> = {};
const mockMilestones: Record<string, Milestone[]> = {};
const mockRetainer: Record<string, number> = { "v-rivera": 2520, "v-okafor": 1620 };
const mockBillingEntries: Record<string, BillingEntry[]> = {
  "v-rivera": [
    { id: "be-1", studentId: null, minutes: 60, description: "Kickoff session", billedAt: "2026-07-01" },
    { id: "be-2", studentId: null, minutes: 45, description: "Essay review call", billedAt: "2026-07-10" },
  ],
};
const mockComposition: Record<string, VaultComposition> = {
  "v-rivera": {
    customization: { icon: "🎓", accent: "70 128 103", theme: null },
    plugins: [
      { plugin_id: "biospark.essay-version-control", settings: null, layout: { w: 2, h: 1 }, order: 0 },
      { plugin_id: "biospark.timeline-weaver", settings: null, layout: { w: 1, h: 1 }, order: 1 },
    ],
  },
  "v-okafor": {
    // A per-vault theme override: this vault renders in Cyan Holographic
    // regardless of the global app theme.
    customization: { icon: "🔬", accent: "6 182 212", theme: "cyan" },
    plugins: [
      { plugin_id: "biospark.timeline-weaver", settings: null, layout: { w: 2, h: 2 }, order: 0 },
      { plugin_id: "biospark.essay-version-control", settings: null, layout: { w: 1, h: 1 }, order: 1 },
    ],
  },
};
const mockRecords: Record<string, PluginRecord[]> = {};
const mockInstalled = new Set<string>();

function collectionManifest(
  id: string,
  name: string,
  description: string,
  icon: string,
  pricing: Pricing,
  panelTitle: string,
  spec: Extract<PanelKind, { type: "collection" }>,
  config_schema: FieldSpec[] = [],
): PluginManifest {
  return {
    id,
    name,
    description,
    version: "1.0.0",
    author: "BioSpark Studios",
    icon,
    category: "advising",
    scope: "chamber",
    capabilities: ["store_plugin_records"],
    kind: { type: "declarative", ui: { panels: [{ title: panelTitle, kind: spec }] } },
    default_layout: { w: 1, h: 1 },
    config_schema,
    pricing,
  };
}

const MOCK_BUILTINS: PluginManifest[] = [
  {
    id: "biospark.essay-version-control",
    name: "Essay Version Control",
    description: "Git-style drafts, revision history, and diffs.",
    version: "1.0.0",
    author: "BioSpark Studios",
    icon: "📝",
    category: "advising",
    scope: "chamber",
    capabilities: ["read_essays", "write_essays"],
    kind: { type: "native", component: "EssayVersionControl" },
    default_layout: { w: 1, h: 1 },
    config_schema: [],
    pricing: { type: "free" },
  },
  {
    id: "biospark.timeline-weaver",
    name: "Application Timeline Weaver",
    description: "Deadlines and milestones on a chronological timeline.",
    version: "1.0.0",
    author: "BioSpark Studios",
    icon: "🗓️",
    category: "advising",
    scope: "chamber",
    capabilities: ["read_milestones", "write_milestones"],
    kind: { type: "native", component: "TimelineWeaver" },
    default_layout: { w: 1, h: 1 },
    config_schema: [],
    pricing: { type: "free" },
  },
  {
    id: "biospark.pathway-blueprint",
    name: "Pathway Blueprint",
    description: "A four-year academic plan — courses, activities, and milestones by grade.",
    version: "1.0.0",
    author: "BioSpark Studios",
    icon: "🧭",
    category: "advising",
    scope: "chamber",
    capabilities: ["store_plugin_records"],
    kind: { type: "native", component: "PathwayBlueprint" },
    default_layout: { w: 2, h: 2 },
    config_schema: [],
    pricing: { type: "free" },
  },
  {
    id: "biospark.college-atlas",
    name: "College Landscape Atlas",
    description: "Browse colleges and build a balanced reach/target/safety shortlist.",
    version: "1.0.0",
    author: "BioSpark Studios",
    icon: "🗺️",
    category: "advising",
    scope: "chamber",
    capabilities: ["store_plugin_records"],
    kind: { type: "native", component: "CollegeAtlas" },
    default_layout: { w: 2, h: 2 },
    config_schema: [],
    pricing: { type: "free" },
  },
  {
    id: "biospark.narrative-loom",
    name: "Aetherial Narrative Loom",
    description: "Capture story threads and weave them into candidate essay angles.",
    version: "1.0.0",
    author: "BioSpark Studios",
    icon: "🧵",
    category: "advising",
    scope: "chamber",
    capabilities: ["store_plugin_records"],
    kind: { type: "native", component: "NarrativeLoom" },
    default_layout: { w: 2, h: 1 },
    config_schema: [],
    pricing: { type: "free" },
  },
  {
    id: "biospark.billing-ledger",
    name: "Billing & Hours",
    description: "Track the retainer, hours worked, and running balance.",
    version: "1.0.0",
    author: "BioSpark Studios",
    icon: "🧾",
    category: "business",
    scope: "vault",
    capabilities: ["read_billing", "write_billing"],
    kind: { type: "native", component: "BillingLedger" },
    default_layout: { w: 1, h: 1 },
    config_schema: [],
    pricing: { type: "free" },
  },
];

const MOCK_STORE: PluginManifest[] = [
  collectionManifest(
    "biospark.recommendation-manager",
    "Recommendation Manager",
    "Track recommendation letters and their status.",
    "✉️",
    { type: "free" },
    "Letters",
    {
      type: "collection",
      collection: "letters",
      add_label: "Add letter",
      title_field: "recommender",
      subtitle_field: "status",
      fields: [
        { key: "recommender", label: "Recommender", kind: { type: "text" }, required: true },
        {
          key: "status",
          label: "Status",
          kind: { type: "select", options: ["Requested", "Received", "Submitted"] },
          required: false,
        },
        { key: "due", label: "Due date", kind: { type: "date" }, required: false },
      ],
    },
    [
      {
        key: "default_status",
        label: "Default status for new letters",
        kind: { type: "select", options: ["Requested", "Received", "Submitted"] },
        required: false,
      },
    ],
  ),
  collectionManifest(
    "biospark.session-notes",
    "Session Notes",
    "Log advising-session notes per student.",
    "🗒️",
    { type: "free" },
    "Notes",
    {
      type: "collection",
      collection: "notes",
      add_label: "Add note",
      title_field: "note",
      subtitle_field: "date",
      fields: [
        { key: "date", label: "Date", kind: { type: "date" }, required: false },
        { key: "note", label: "Note", kind: { type: "long_text" }, required: true },
      ],
    },
  ),
  collectionManifest(
    "biospark.scholarship-tracker",
    "Scholarship Tracker",
    "Track scholarship applications, amounts, and deadlines.",
    "🎓",
    { type: "paid", tier: "pro", price_cents: 500 },
    "Scholarships",
    {
      type: "collection",
      collection: "scholarships",
      add_label: "Add scholarship",
      title_field: "name",
      subtitle_field: "status",
      fields: [
        { key: "name", label: "Name", kind: { type: "text" }, required: true },
        { key: "amount", label: "Amount", kind: { type: "text" }, required: false },
        { key: "deadline", label: "Deadline", kind: { type: "date" }, required: false },
        {
          key: "status",
          label: "Status",
          kind: { type: "select", options: ["Researching", "Applied", "Awarded"] },
          required: false,
        },
      ],
    },
  ),
];

function ledgerBalanceHours(vaultId: string): number {
  const retainer = mockRetainer[vaultId] ?? 0;
  const used = (mockBillingEntries[vaultId] ?? []).reduce((sum, e) => sum + e.minutes, 0);
  return Math.round(((retainer - used) / 60) * 10) / 10;
}

function sortMilestones(list: Milestone[]): Milestone[] {
  return [...list].sort((a, b) => {
    if (a.dueAt === b.dueAt) return 0;
    if (a.dueAt === null) return 1;
    if (b.dueAt === null) return -1;
    return a.dueAt < b.dueAt ? -1 : 1;
  });
}

async function mock<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  await new Promise((r) => setTimeout(r, 180));
  switch (cmd) {
    case "health_check":
      return true as T;
    case "get_vault_hierarchy":
      // Customization is sourced from each vault's composition (the SSoT);
      // the hour balance is computed live from the billing ledger, same as
      // the real backend.
      return mockVaults.map((v) => {
        const c = mockComposition[v.id]?.customization;
        return {
          ...v,
          icon: c?.icon ?? v.icon,
          accent: c?.accent ?? v.accent,
          hourBalance: ledgerBalanceHours(v.id),
        };
      }) as T;
    case "create_vault": {
      const id = `v-${Math.random().toString(36).slice(2, 8)}`;
      const customization = (args?.customization as VaultCustomization | undefined) ?? {};
      const template = (args?.template as string[] | undefined) ?? [];
      const allManifests = [...MOCK_BUILTINS, ...MOCK_STORE];
      const composition: VaultComposition = {
        customization,
        plugins: template.flatMap((pid, i) => {
          const m = allManifests.find((x) => x.id === pid);
          return m ? [{ plugin_id: pid, settings: null, layout: m.default_layout, order: i }] : [];
        }),
      };
      mockComposition[id] = composition;
      // Enabling a store plugin via a template implies it is installed.
      template.forEach((pid) => {
        if (MOCK_STORE.some((m) => m.id === pid)) mockInstalled.add(pid);
      });
      const card: VaultCard = {
        id,
        name: String(args?.name ?? "New Vault"),
        stage: (args?.stage as VaultStage) ?? "prospective",
        students: 0,
        chambers: 0,
        hourBalance: 0,
        lastActivity: "just now",
        icon: customization.icon ?? null,
        accent: customization.accent ?? null,
      };
      mockVaults.push(card);
      return card as T;
    }
    case "move_vault": {
      const v = mockVaults.find((x) => x.id === args?.id);
      if (v) v.stage = args?.stage as VaultStage;
      return undefined as T;
    }
    case "chamber_ai_enabled":
      return (mockAi[String(args?.chamberId)] ?? false) as T;
    case "set_chamber_ai":
      mockAi[String(args?.chamberId)] = Boolean(args?.enabled);
      return undefined as T;
    case "commit_essay": {
      const key = `${args?.vaultId}:${args?.essayId}`;
      const history = mockEssays[key] ?? (mockEssays[key] = []);
      const version: EssayVersion = {
        id: `ev-${Math.random().toString(36).slice(2, 8)}`,
        essayId: String(args?.essayId),
        seq: history.length + 1,
        message: String(args?.message ?? ""),
        body: String(args?.body ?? ""),
      };
      history.push(version);
      return version as T;
    }
    case "essay_history": {
      const key = `${args?.vaultId}:${args?.essayId}`;
      return [...(mockEssays[key] ?? [])] as T;
    }
    case "add_milestone": {
      const key = `${args?.vaultId}:${args?.chamberId}`;
      const list = mockMilestones[key] ?? (mockMilestones[key] = []);
      const m: Milestone = {
        id: `m-${Math.random().toString(36).slice(2, 8)}`,
        title: String(args?.title ?? ""),
        dueAt: (args?.dueAt as string | null) ?? null,
        done: false,
      };
      list.push(m);
      return m as T;
    }
    case "list_milestones": {
      const key = `${args?.vaultId}:${args?.chamberId}`;
      return sortMilestones(mockMilestones[key] ?? []) as T;
    }
    case "set_milestone_done": {
      for (const list of Object.values(mockMilestones)) {
        const m = list.find((x) => x.id === args?.id);
        if (m) m.done = Boolean(args?.done);
      }
      return undefined as T;
    }
    case "get_billing_ledger": {
      const vaultId = String(args?.vaultId);
      const retainerMinutes = mockRetainer[vaultId] ?? 0;
      const entries = [...(mockBillingEntries[vaultId] ?? [])].sort((a, b) =>
        a.billedAt < b.billedAt ? 1 : -1,
      );
      const usedMinutes = entries.reduce((sum, e) => sum + e.minutes, 0);
      const ledger: BillingLedger = {
        retainerMinutes,
        usedMinutes,
        balanceMinutes: retainerMinutes - usedMinutes,
        entries,
      };
      return ledger as T;
    }
    case "set_retainer_minutes": {
      mockRetainer[String(args?.vaultId)] = Number(args?.minutes ?? 0);
      return undefined as T;
    }
    case "log_billing_entry": {
      const vaultId = String(args?.vaultId);
      const list = mockBillingEntries[vaultId] ?? (mockBillingEntries[vaultId] = []);
      const entry: BillingEntry = {
        id: `be-${Math.random().toString(36).slice(2, 8)}`,
        studentId: (args?.studentId as string | null) ?? null,
        minutes: Number(args?.minutes ?? 0),
        description: String(args?.description ?? ""),
        billedAt: String(args?.billedAt ?? ""),
      };
      list.push(entry);
      return entry as T;
    }
    case "delete_billing_entry": {
      for (const list of Object.values(mockBillingEntries)) {
        const i = list.findIndex((e) => e.id === args?.id);
        if (i >= 0) list.splice(i, 1);
      }
      return undefined as T;
    }
    case "list_available_plugins": {
      const builtins: AvailablePlugin[] = MOCK_BUILTINS.map((manifest) => ({
        manifest,
        trust: "verified",
        source: "builtin",
        installed: true,
      }));
      const store: AvailablePlugin[] = MOCK_STORE.map((manifest) => ({
        manifest,
        trust: "verified",
        source: "forge",
        installed: mockInstalled.has(manifest.id),
      }));
      return [...builtins, ...store] as T;
    }
    case "install_plugin":
      mockInstalled.add(String(args?.id));
      return undefined as T;
    case "uninstall_plugin":
      mockInstalled.delete(String(args?.id));
      return undefined as T;
    case "get_vault_composition":
      return (mockComposition[String(args?.vaultId)] ?? {
        customization: {},
        plugins: [],
      }) as T;
    case "set_vault_composition":
      mockComposition[String(args?.vaultId)] = args?.composition as VaultComposition;
      return undefined as T;
    case "plugin_record_add": {
      const key = `${args?.vaultId}:${args?.pluginId}:${args?.collection}:${args?.chamberId ?? ""}`;
      const list = mockRecords[key] ?? (mockRecords[key] = []);
      const rec: PluginRecord = {
        id: `pr-${Math.random().toString(36).slice(2, 8)}`,
        data: (args?.data as Record<string, unknown>) ?? {},
      };
      list.push(rec);
      return rec as T;
    }
    case "plugin_record_list": {
      const key = `${args?.vaultId}:${args?.pluginId}:${args?.collection}:${args?.chamberId ?? ""}`;
      return [...(mockRecords[key] ?? [])] as T;
    }
    case "plugin_record_delete": {
      for (const list of Object.values(mockRecords)) {
        const i = list.findIndex((r) => r.id === args?.id);
        if (i >= 0) list.splice(i, 1);
      }
      return undefined as T;
    }
    case "invoke_quantum_quill": {
      const prompt = String(args?.prompt ?? "");
      const reply: ChatReply = {
        text: `Here's a start on "${prompt.slice(0, 60)}": open with a specific, sensory moment that only you could have written, then connect it to what it revealed about you. (mock reply — connect a live model to see real drafting.)`,
        provider: "mock",
        model: "mock",
      };
      return reply as T;
    }
    default:
      throw new Error(`unknown command: ${cmd}`);
  }
}
