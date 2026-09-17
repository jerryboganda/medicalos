# 10 — Runtime approval (owner decision — OPS-02)

Status: ready-for-human
Requirement IDs: OPS-02; §27 'Unresolved runtime boundary'

## Question for owner

Approve the serving/inference runtime for production (managed application + inference infrastructure separate from GitHub Actions). GitHub-hosted runners have execution limits and cannot permanently serve the app (§27, S37).

Note: this BLOCKS FIRST REAL DEPLOYMENT ONLY. All Phase 0–1 development runs in GitHub Actions CI against ephemeral services; no silent exception is created. Hosting region / data residency (§32) should be decided together with this.
