# 01 — QB-08 report API + quarantine exclusion (backend)

Status: implemented; current working-tree API verification pending
Requirement IDs: QB-08 (report intake, quarantine, grouped resolution, private
reporter feedback, public correction history, and SLA fields; see the QB-16
addendum for full acceptance boundaries).

Implement per `.scratch/phase-1-qb08-reports/spec.md` Implementation Decisions.
New migration `0005_item_reports` (up + down, idempotent IF NOT EXISTS /
IF EXISTS guards like 0001–0004): `question_reports` table with
UNIQUE(question_version_id, reporter_id), status open|quarantined|
resolved_fixed|resolved_rejected, category enum, note <= 2000, resolution
note, resolved_at, created_at.

New routes module `reports`: POST /v1/questions/versions/{id}/reports,
GET /v1/questions/versions/{id}/reports, GET /v1/admin/reports, and
POST /v1/reports/{id}/resolve (authenticated and admin-token gated; see the
resolution and SLA addendum in `.scratch/remaining-phases/issues/02-qb16-report-sla.md`).
Wire into `router()` in lib.rs. Quarantine predicate in the pool queries in
routes/practice.rs (tutor/timed + revision); `report_status` on get_session
items. Same `ApiError` shape ({error:{code,message}}).

Earlier API CI covered only the report intake/quarantine baseline. It does not
validate the current working tree. The resolution, correction-history, and SLA
behavior need the deferred final Actions and browser run.
