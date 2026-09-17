# 18 — CI-built artifacts on developer devices (owner ruling)

Status: ready-for-human
Requirement IDs: §32 'Running CI-built artifacts on developer-owned devices'; §27.1 mobile feedback loop

## Question for owner

Under the GitHub Actions-only compute rule: is installing/running a CI-built build on a developer-owned simulator or physical phone permitted for debugging and the Phase 0 spike? The plan leaves this explicitly open (§27.1). Recommendation: YES for running artifacts (device testing requires it; building stays in CI), with the signed-artifact provenance requirement intact.
