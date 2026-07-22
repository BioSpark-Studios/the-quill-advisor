<p align="center">
  <img src="develop/src-tauri/icons/icon.png" alt="The Quill Advisor — BioSpark Studios" width="400">
</p>

<h1 align="center">The Quill Advisor</h1>


**AI-powered, privacy-centric college-advising studio for independent advisors and small practices.**

A self-hosted desktop application built in Rust (Tauri v2) with a React frontend.
Student data lives in physically isolated, per-vault SQLite files; a multi-provider
AI router ("Omni-Route") powers the Quantum Quill drafting assistant while a
counselor stays in control of when and where AI runs. Almost every feature inside
a vault is a **plugin** — installed, enabled, configured, and arranged per vault
through the BioSpark Forge.

By **BioSpark Studios** · a **Phantori** production.

## Architecture

```
the-quill-advisor/
├─ crates/
│  ├─ quill-core/     # Pure domain: vault hierarchy, gate security, compliance, AI authorization
│  ├─ quill-plugin/   # BioSpark Forge: plugin manifests, capabilities, UI schema, ed25519 signing, vault composition
│  ├─ quill-storage/  # SQLite (WAL): global core_db + isolated per-vault vault_db, .qavault export
│  └─ quill-ai/       # Omni-Route multi-provider router + the Quantum Quill agent
├─ src-tauri/         # Tauri v2 desktop shell: IPC commands, state wiring  (builds where webkit2gtk is present)
├─ ui/                # React + Vite + Tailwind frontend (splash, Master Vault board, plugins, Orb + chat)
└─ docs/architecture/ # The original scaffolding/design documents
```

### The core concepts

- **SSoT vault hierarchy** — Level 0 Master → Level 1 Counselor → Level 2 Classroom → Level 3 Chamber.
- **Gate security** — vertical boundary enforcement with explicit I/O sharing contracts; siblings are isolated.
- **Physical data isolation** — one SQLite file per Counselor vault; a second vault is unreachable through the first.
- **Omni-Route** — a provider-trait router over OpenAI, Anthropic, Gemini, and Ollama, with fallback.
- **Counselor-led AI** — the Quantum Quill agent may only act inside a chamber the counselor has explicitly enabled.

### BioSpark Forge — plugins & vault composition

Vaults are composed, not hard-coded. A plugin has two halves:

- a **manifest** (the *definition*) — id, capabilities, pricing, and either a
  native component or a declarative UI schema; store plugins are **ed25519-signed**
  and badged Verified / Unsigned / Untrusted against a trust store;
- a **vault composition** (the *instance*) — which plugins a vault has enabled,
  each one's per-vault settings and Bento layout, plus the vault's own crest,
  accent, and theme.

Declarative plugins render generically from their UI schema and persist to a
**capability-gated generic record store** — no plugin-specific backend code.
Native tools (Essay Version Control, Application Timeline Weaver, Pathway
Blueprint, College Landscape Atlas, Aetherial Narrative Loom) ride the same
rail. Each vault carries its own theme, accent, and crest, independent of the
global app theme.

## Developing

The logic crates and the UI build anywhere. The Tauri desktop shell needs
`webkit2gtk-4.1` (Linux) or the platform WebView, so full app runs happen on a
developer machine.

```bash
# Rust logic crates — build & test (no system WebView needed)
cargo test

# Lint / format
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Frontend
cd ui && npm install && npm run dev

# Full desktop app (requires webkit2gtk / platform WebView + the Tauri CLI)
cargo tauri dev
```

Copy `.env.example` to `.env` and add provider API keys for cloud LLMs. Ollama
needs no key and is the default local/offline provider.

## Contributing

Sprint work lives on `develop`; open PRs from `feature/*` branches. See `CONTRIBUTING.md`.

## License

See `LICENSE` (Quill Advisor License).
