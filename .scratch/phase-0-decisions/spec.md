# Phase 0 — Decisions and Evidence: Spec

Master plan §28 + §28.1, Phase 0. Gate to advance: **spike report against the §30.2 client budgets on the reference devices, and no unresolved decision that would invalidate the first implementation slice.**

## Deliverables

Per §28: repository/greenfield decision, budget envelope, content rights inventory, pilot exam, official blueprint, runtime approval, architecture records, design tokens, reference-based interaction prototype.
Per §28.1 additions: Tauri mobile go/no-go spike (§20.1), shared Rust core proven natively and as WebAssembly, CI runner matrix (§27.1), product-analytics taxonomy, store accounts + storefront payment-policy check, decision on pre-medical packs.

## Ticket index

Executable (agent-ready):

- issues/01-greenfield-decision.md — audit conclusion + record (OPS row of §32)
- issues/02-adr-decisions-9-13.md — architecture records for approved decisions
- issues/03-monorepo-scaffold.md — §20.4 tree, lazily populated
- issues/04-ci-runner-matrix.md — §27.1 pipeline gates in GitHub Actions (OPS-01, OPS-07)
- issues/05-design-tokens.md — §7.1 initial tokens into packages/design-system
- issues/06-companion-files-update.md — README.md + AGENT_IMPLEMENTATION_HANDOFF.md to the new stack (§32 mandatory)
- issues/07-analytics-taxonomy.md — Appendix B → docs/requirements/analytics-taxonomy.md (OPS-05 prep)
- issues/08-tauri-mobile-spike.md — go/no-go spike plan + report template (§20.1, §30.2 budgets)

Owner decisions (each prepared with a recommendation; owner decides):

- issues/09-pilot-exam-pack.md · issues/10-runtime-approval.md (OPS-02, blocks deployment not development) · issues/11-budget-and-commercial-proposals.md · issues/12-brand-entity.md · issues/13-blueprint-acquisition.md · issues/14-rights-inventory.md · issues/15-store-accounts-payment-policy.md · issues/16-premedical-packs.md · issues/17-desktop-distribution.md · issues/18-ci-artifacts-on-devices.md · issues/19-ui-reference-asset.md

## Constraints honored

- Compute rule (§27, AGENTS.md): all builds/tests in GitHub Actions. Local work is file authoring only.
- No empty packages (§20.4): scaffold contains only crates/apps Phase 0 exercises.
- Runtime approval is NOT required to build Phase 0–1 in CI; it gates first real deployment only.
