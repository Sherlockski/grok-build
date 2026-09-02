---
name: ci-auto-remediate
description: >-
  Diagnose and fix a failed grok-build CI workflow (sync-upstream or a future
  grok-build deploy) using the canonical local repository mirror on the
  agent-harness and, when the failure involves the self-hosted runner, live-debug
  the runner guest over the Vault-signed SSH operator channel. Invoked
  automatically by .github/workflows/auto-remediate.yml after a workflow_run
  failure; produces a local commit that the orchestrator pushes and re-runs.
argument-hint: '[workflow name] [run id] [local path] [error summary]'
user-invocable: false
---

# CI Auto-Remediate (grok-build)

You are running on the **agent-harness** (`agent-harness.lan`), invoked by the
`auto-remediate` GitHub Actions workflow after a grok-build CI job failed. Your
job is to find the root cause, fix it in the local repository mirror, and leave a
single local commit. **Do not push and do not re-run the workflow** — the
orchestrator (`auto-remediate.yml`) does that with a scoped token.

## Canonical paths

- Local mirror (always work here): `/srv/agents/workspaces/grok-build`
  (branch `main`, `origin` = `Sherlockski/grok-build`, `upstream` = `xai-org/grok-build`).
- Infra IaC (debug tooling + this harness's access model):
  `/srv/agents/workspaces/infrastructure`
- Operator SSH into any guest:
  `/srv/agents/workspaces/infrastructure/scripts/ssh-as-operator.sh <tenant> <host>`
- Read `infrastructure/AGENTS.md` for the full debugging model before touching
  guests.

## Workflows covered

- `sync-upstream` — hourly fork-rebase of xai-org/grok-build onto
  `Sherlockski/grok-build`. Runs on **GitHub-hosted** `ubuntu-latest`, so there is
  **no self-hosted runner guest to debug** for this one — failures are local
  merge/rebase or `cargo build`/`cargo test` errors in the mirror.
- A future grok-build **deploy** workflow would run on the self-hosted runner
  `personal-github-runner-prod` (labels include `grok-build`, unit
  `github-actions-runner-grok-build`, dir `/opt/actions-runner-grok-build`).

## Procedure

1. **Orient.** `cd /srv/agents/workspaces/grok-build && git status && git log --oneline -3`.
   Stay on `main`. Do not create feature branches — the orchestrator pushes `HEAD`.

2. **Reproduce locally** (the cheapest signal first):
   - For `sync-upstream`:
     ```bash
     git fetch upstream main && git fetch origin main
     # mirror the workflow's rebase: fork commits are replayed, so "theirs" = fork
     git rebase -X theirs upstream/main || true
     cargo build -p xai-grok-pager-bin --locked
     cargo test --workspace --locked --quiet || true
     ```
   - Capture the *actual* error; the orchestrator's base64 log is a hint, not gospel.

3. **Diagnose.**
   - **Rebase conflict:** identify conflicting files. The fork intentionally wins
     feature conflicts (`git rebase -X theirs`), so resolve any *remaining*
     (non-auto-resolvable) conflicts by favoring the fork's intent while keeping
     upstream changes that don't conflict. Never hand-edit upstream-only logic.
   - **Build/test failure:** fix the source in the mirror. Prefer minimal,
     correct changes over broad refactors.

4. **Live-debug the self-hosted runner guest ONLY if the failure is on the
   self-hosted runner** (i.e. a grok-build deploy job, not `sync-upstream`):
   ```bash
   bash /srv/agents/workspaces/infrastructure/scripts/ssh-as-operator.sh \
     personal personal-github-runner-prod
   ```
   Then inspect as operator `datscreamer`:
   - `systemctl status github-actions-runner-grok-build --no-pager`
   - `journalctl -u github-actions-runner-grok-build -n 200 --no-pager`
   - `/opt/actions-runner-grok-build` layout, disk (`df -h`), memory, egress.
   **Never** use raw `ssh`/static keys or `ping` — go through `ssh-as-operator.sh`
   (Vault-signed cert). ICMP is dropped by nftables anyway.

5. **Apply the fix** in the mirror and verify by re-running the failing command
   locally until it passes.

6. **Commit locally only.** Stage the minimal change and commit on `main`:
   ```bash
   git add -A
   git commit -m "fix(ci): <root cause> — auto-remediated by opencode/muse-spark-1.2-free"
   ```
   The orchestrator pushes `HEAD:main` and re-runs the failed workflow.

7. **Report concisely:** root cause, files changed, local verification result, and
   whether guest live-debug was required. If you cannot safely fix it, say so
   explicitly rather than pushing a guess.
