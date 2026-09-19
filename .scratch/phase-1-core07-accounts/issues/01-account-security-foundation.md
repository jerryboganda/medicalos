# 01: Personal account security foundation

Status: claimed

Requirement IDs: CORE-07 (partial until Google/Apple provider adapters are real), prepares TRUST-02 and PROT-02.

Implement the vertical slice in `../spec.md` through the authenticated HTTP seam.

- [ ] Email/password registration requires verified email before login.
- [ ] Verification and password-reset challenges are expiring, hashed, and single-use.
- [ ] Password reset revokes existing sessions.
- [ ] Access tokens are short-lived and refresh tokens rotate on use.
- [ ] Durable sessions carry stable device identity and a maximum of two active devices is enforced.
- [ ] Learner can list active device sessions, sign out current, and sign out other devices.
- [ ] Account deletion can be initiated in-app/API, revokes sessions, and blocks login while pending.
- [ ] Only one open study session may exist; conflict identifies it and explicit takeover abandons it.
- [ ] Client registration/login/session storage and sign-out follow the new server contract.
- [ ] A compact Account page exposes session management and deletion initiation using the existing Medical OS visual system.
- [ ] Production never exposes raw verification/reset secrets; test-only echo stays hard-disabled in the production binary.
- [ ] Google/Apple buttons are not faked; provider adapters remain a visible boundary until real configuration exists.
- [ ] Migration remains reversible under the repository up/down/up gate.
- [ ] GitHub Actions proves the slice; no heavy local/VPS compute and no deployment.

## Comments

- Implementation started from accepted CORE-04 base `63d5174` on branch `codex/core07-accounts`.
