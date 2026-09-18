# 01 — QB-08 report API + quarantine exclusion (backend)

Status: ready-for-agent
Requirement IDs: QB-08 (partial — reporting + pool exclusion; console/SLA in Phase 2).

Implement per `.scratch/phase-1-qb08-reports/spec.md` Implementation Decisions.
New migration `0005_item_reports` (up + down, idempotent IF NOT EXISTS /
IF EXISTS guards like 0001–0004): `question_reports` table with
UNIQUE(question_version_id, reporter_id), status open|quarantined|
resolved_fixed|resolved_rejected, category enum, note <= 2000, resolution
note, resolved_at, created_at.

New routes module `reports`: POST /v1/questions/versions/{id}/reports,
GET /v1/questions/versions/{id}/reports, POST /v1/reports/{id}/resolve
(authenticated; resolve returns 501 editor_console_pending until CORE-04).
Wire into `router()` in lib.rs. Quarantine predicate in the pool queries in
routes/practice.rs (tutor/timed + revision); `report_status` on get_session
items. Same `ApiError` shape ({error:{code,message}}).

Gate evidence: seam tests in apps/api/tests/integration.rs through the HTTP
API only (no internals), green in CI against ephemeral PostgreSQL.
Regenerate `.sqlx/` offline cache in CI before merge.
