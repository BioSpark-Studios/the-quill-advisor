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
