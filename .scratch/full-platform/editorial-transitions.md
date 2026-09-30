Status: implemented; exact-source CI acceptance pending
Requirements: ADMIN-02, QB-01, TRUST-04

## Problem Statement

Submit/approve/reject previously read status, changed content and wrote review/audit
records outside one transaction. A failed audit can leave a changed item, and
competing reviewers can transition a status they read before another change.

## Solution

Lock the version and validate the transition within one transaction. Commit
review provenance, status and its audit together, preserving the same public
workflow, permissions and UI. Publishing additionally requires a recorded
independent reviewer, alongside the existing author and rights checks.

## User Stories

1. As an author, I want a failed submission to leave my draft unchanged.
2. As a reviewer, I want a competing decision to be rejected after a status change.
3. As an operator, I want every committed transition to have its audit record.
4. As a learner, I want published content to retain an independent review identity.

## Testing Decisions

Use authenticated HTTP workflow and question-search interfaces. A scoped audit
constraint in the disposable CI schema simulates an unavailable audit write;
remove it before assertions. Test stored status through the public search API.
Cover submission and both review decisions, missing/self-review provenance and
competing decisions held behind a database row lock. Run 36786239795 confirmed
the original submission defect: an audit failure left the version in_review.
Retain the existing rights, role and author-separation regressions. No real
clinical approval is inferred from synthetic reviewer fixtures.
