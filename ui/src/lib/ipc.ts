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
}

export type VaultStage = "prospective" | "active" | "renewal" | "alumni";

export const STAGES: { id: VaultStage; label: string }[] = [
  { id: "prospective", label: "Prospective" },
  { id: "active", label: "Active" },
  { id: "renewal", label: "Renewal" },
  { id: "alumni", label: "Alumni" },
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
  createVault: (name: string, stage: VaultStage) =>
    invoke<VaultCard>("create_vault", { name, stage }),
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
  },
  {
    id: "v-okafor",
    name: "Dr. Okafor — STEM Cohort",
    stage: "active",
    students: 9,
    chambers: 9,
    hourBalance: 27,
    lastActivity: "yesterday",
  },
  {
    id: "v-lindqvist",
    name: "Lindqvist Advising",
    stage: "prospective",
    students: 3,
    chambers: 3,
    hourBalance: 6,
    lastActivity: "3d ago",
  },
  {
    id: "v-summit",
    name: "Summit Legacy Families",
    stage: "renewal",
    students: 21,
    chambers: 21,
    hourBalance: 88,
    lastActivity: "1w ago",
  },
];

const mockAi: Record<string, boolean> = {};
const mockEssays: Record<string, EssayVersion[]> = {};
const mockMilestones: Record<string, Milestone[]> = {};

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
      return [...mockVaults] as T;
    case "create_vault": {
      const card: VaultCard = {
        id: `v-${Math.random().toString(36).slice(2, 8)}`,
        name: String(args?.name ?? "New Vault"),
        stage: (args?.stage as VaultStage) ?? "prospective",
        students: 0,
        chambers: 0,
        hourBalance: 0,
        lastActivity: "just now",
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
