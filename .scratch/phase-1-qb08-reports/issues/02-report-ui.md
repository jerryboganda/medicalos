# 02 — QB-08 report button in session UI

Status: ready-for-agent
Requirement IDs: QB-08 (partial — learner affordance + honest states).

Session page (`apps/client/src/routes/session/[id]/+page.svelte`, in place —
hallmark safety rail, no route changes): a "Report a problem" control on
answered items opening the 7 categories + optional note, POSTing through a
new `Api.reportQuestion` client call, with truthful states
(loading / reported-thanks / already-reported / error+retry per CORE-06).
Items with non-null `report_status` on session detail show an honest label
("Flagged by learners — under review" / "Corrected" after resolve).
No new tokens or styles beyond existing `app.css` primitives
(card/btn/linklike/error-text/chip).

Gate evidence: exercised in the browser E2E suite against the real API;
existing learner-loop E2E stays green.
