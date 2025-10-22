# Contributing to the-quill-advisor

Welcome — thanks for contributing.

Branching
- main — protected (releases only)
- develop — integration branch
- feature/<short-desc> — feature branches
- release/<version> — release preparation
- hotfix/<short-desc> — urgent fixes

Commits
- Use conventional commits (feat:, fix:, chore:, docs:, refactor:).
- Keep messages concise and descriptive.

Pull requests
- Target `develop` for feature work.
- Include a clear summary and link related issues.
- Ensure CI passes and include tests for new logic.
- Request reviews from CODEOWNERS.

Local setup
- See README for install and dev commands.
- Use the provided seed scripts to populate dev DB: `scripts/seed-dev.sh`.

Issue tracking
- Use labels: bug, enhancement, chore, priority/P0, priority/P1.
- Add a short reproduction or spec to issues before starting work.

Security & data
- Never commit secrets. Use GitHub secrets for CI and environment variables.
