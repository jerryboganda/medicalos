# Medical Learning OS

The product connects navigational curricula, learning content, and learner evidence while keeping their distinct identities clear.

## Curriculum and concepts

**Curriculum node**: An exam-scoped place in the browsing and reporting hierarchy, such as a subject, system, chapter, topic, or subtopic.
_Avoid_: Concept, when referring to a browsing category.

**Concept identity**: A stable, reusable identity for a unit of knowledge that may be linked from multiple curriculum nodes and learning modalities.
_Avoid_: Tag, when referring to a canonical knowledge identity.

**Concept version**: An immutable, numbered description of a concept identity. The stable identity survives wording changes to its description.

**Curriculum mapping**: A curated link between a curriculum node and a concept identity. A node may link to several concepts, and a concept may appear under several nodes.

**Note tag**: A learner-entered label used to organize private notes. It is not a canonical concept identity unless explicitly linked to one.

## Daily engagement

**Daily available minutes**: The learner's persisted declaration of how many minutes they can study on a day. Today uses it as the planning budget, and minutes-mode engagement uses the same value as its goal target.

**Daily goal mode**: Whether daily engagement progress is measured by completed answered questions or by recorded elapsed minutes on completed answers. Recorded minutes are a learner-facing progress aid, not proctored time evidence.

**QOTD exam**: The exam a learner selects for their question of the day. The choice is scoped to QOTD and is independent of plan and practice-session exam contexts.

**Shared daily question**: One eligible published question version for an exam and the database calendar day, shown to every learner who selects that exam.

**Daily QOTD answer lock**: After a learner answers that day's question, the selected QOTD exam stays fixed until the next database calendar day, preserving one QOTD answer per learner per day.

## Content provenance

**Source passage**: A stable reference to a precise part of an external source, identified by its citation and locator. The reference does not imply that the source text is stored or licensed for redistribution.

**Source dependency**: A recorded link from a source passage to an exact version of a learning resource that relies on it.

**Source-change case**: An editorial record of an observed source change, its affected resource versions, review tasks, and any required safety actions.

**Content-rights record**: An operator-maintained statement of a content source's permitted uses, territory, validity window, and revocation state. It records supplied licensing terms; it does not prove that a contract or ownership claim is authentic.

**Current display eligibility**: Whether a question version may be shown to a learner at the time of a request, based on its current display permission, audience, validity, seat allocation, and complete source/media scope. Publication approval does not establish lasting eligibility.

**Question-linked learner record**: A learner-owned note, mark, or answer associated with an exact question version. Its stored state and ownership are distinct from whether the linked question content is currently eligible for display.

**Private import**: A document owned by one learner account and readable only while its linked content-rights record permits private import and display. It is not a shared library resource and may not contain patient-identifiable information.

## Clinical simulation

**Rubric criterion**: A version-scoped learning objective with a label and maximum score.

**Criterion assessment**: A human examiner's result for one rubric criterion. An assessed result cites transcript events and a score; a not-assessed result has no score and explains why.

## Document extraction quality

**Extraction report**: An immutable, checksum- and parser-version-scoped manifest of expected, extracted, uncertain, and critical document regions. The API derives missing regions and report status; the record does not contain the source file or extracted prose.

**Extraction review**: A distinct operator's append-only decision over the report. Approval requires an active extraction grant, a clean scan state, complete region coverage, and explicit verification of every critical or uncertain region.
