# The Quill Advisor

**AI-powered, privacy-centric college-advising studio for independent advisors and small practices.**

A self-hosted desktop application built in Rust (Tauri v2) with a React frontend.
Student data lives in physically isolated, per-vault SQLite files; a multi-provider
AI router ("Omni-Route") powers the Quantum Quill drafting assistant while a
counselor stays in control of when and where AI runs.

By **BioSpark Studios** · a **Phantori** production.

## Architecture

```
the-quill-advisor/
├─ crates/
│  ├─ quill-core/     # Pure domain: vault hierarchy, gate security, compliance, AI authorization
│  ├─ quill-storage/  # SQLite (WAL): global core_db + isolated per-vault vault_db, .qavault export
│  └─ quill-ai/       # Omni-Route multi-provider router + the Quantum Quill agent
├─ src-tauri/         # Tauri v2 desktop shell: IPC commands, state wiring  (builds where webkit2gtk is present)
├─ ui/                # React + Vite + Tailwind frontend (splash, Master Vault board, Orb + chat)
└─ docs/architecture/ # The original scaffolding/design documents
```

### The core concepts

- **SSoT vault hierarchy** — Level 0 Master → Level 1 Counselor → Level 2 Classroom → Level 3 Chamber.
- **Gate security** — vertical boundary enforcement with explicit I/O sharing contracts; siblings are isolated.
- **Physical data isolation** — one SQLite file per Counselor vault; a second vault is unreachable through the first.
- **Omni-Route** — a provider-trait router over OpenAI, Anthropic, Gemini, and Ollama, with fallback.
- **Counselor-led AI** — the Quantum Quill agent may only act inside a chamber the counselor has explicitly enabled.

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
