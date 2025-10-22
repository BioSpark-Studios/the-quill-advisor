# the-quill-advisor

The Quill Advisor — AI-powered, modular college advising studio for independent advisors and small practices.

Quick start
1. Clone: `git clone git@github.com:<your-org>/the-quill-advisor.git`
2. Create .env from .env.example and add secrets
3. Install: `npm ci`
4. Start dev: `npm run dev` (see package.json scripts per app)
5. Run tests: `npm run test`

Repository layout
- /apps/web — frontend (React/Next)
- /apps/api — backend services
- /libs/ui — shared UI components and manifests
- /libs/agents — agent contracts, prompts, test adapters
- /ops/infra — Terraform, Helm charts, deploy scripts
- /scripts — helper scripts (db seeds, local start helpers)

Contributing
- See CONTRIBUTING.md for branch and PR rules.
- Sprint work lives on `develop`. Use `feature/*` branches for work and open PRs targeting `develop`.

Contacts
- Project owner: <your-name>
- Slack: #the-quill-advisor
