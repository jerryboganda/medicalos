# 01: Personal account security foundation

Status: in-progress (foundation tested; external identity/email adapters pending)

Requirement IDs: CORE-07 (partial until Google/Apple provider adapters are real), prepares TRUST-02 and PROT-02.

Implement the vertical slice in `../spec.md` through the authenticated HTTP seam.

- [x] Email/password registration requires verified email before login.
- [x] Verification and password-reset challenges are expiring, hashed, and single-use.
- [x] Password reset revokes existing sessions.
- [x] Access tokens are short-lived and refresh tokens rotate on use.
- [x] Durable sessions carry stable device identity and a maximum of two active devices is enforced.
- [x] Learner can list active device sessions, sign out current, and sign out other devices.
- [x] Account deletion can be initiated in-app/API, revokes sessions, and blocks login while pending.
- [x] Only one open study session may exist; conflict identifies it and explicit takeover abandons it.
- [x] Client registration/login/session storage and sign-out follow the new server contract.
- [x] A compact Account page exposes session management and deletion initiation using the existing Medical OS visual system.
- [x] Production never exposes raw verification/reset secrets; test-only echo stays hard-disabled in the production binary.
- [x] Google/Apple buttons are not faked; provider adapters remain a visible boundary until real configuration exists.
- [x] Migration remains reversible under the repository up/down/up gate.
- [x] GitHub Actions proves the slice; no heavy local/VPS compute and no deployment.
- [ ] Real production email delivery adapter + credentials are configured and verified.
- [ ] Real Google and Apple sign-in adapters + credentials are implemented and verified.

## Comments

- Implementation started from accepted CORE-04 base `63d5174` on branch `codex/core07-accounts`.
- GitHub Actions run `35428827015` is green across site, client build, Rust fmt, schema apply, Clippy, dependency advisories/licenses, Rust tests, SQLx offline query cache, wasm32 shared-core build, and browser E2E.
- Production registration and password recovery fail closed with `503 auth_email_delivery_unavailable` before account/challenge mutation when no real delivery path exists.
- CORE-07 remains in progress until real email delivery plus Google and Apple provider adapters are supplied and verified.
