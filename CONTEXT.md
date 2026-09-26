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

**QOTD exam**: The exam a learner selects for their question of the day. The choice is scoped to QOTD and is independent of plan and practice-session exam contexts.

**Shared daily question**: One eligible published question version for an exam and the database calendar day, shown to every learner who selects that exam.

**Daily QOTD answer lock**: After a learner answers that day's question, the selected QOTD exam stays fixed until the next database calendar day, preserving one QOTD answer per learner per day.

## Content provenance

**Source passage**: A stable reference to a precise part of an external source, identified by its citation and locator. The reference does not imply that the source text is stored or licensed for redistribution.

**Source dependency**: A recorded link from a source passage to an exact version of a learning resource that relies on it.

**Source-change case**: An editorial record of an observed source change, its affected resource versions, review tasks, and any required safety actions.

**Content-rights record**: An operator-maintained statement of a content source's permitted uses, territory, validity window, and revocation state. It records supplied licensing terms; it does not prove that a contract or ownership claim is authentic.

**Private import**: A document owned by one learner account and readable only while its linked content-rights record permits private import and display. It is not a shared library resource and may not contain patient-identifiable information.

## Clinical simulation

**Rubric criterion**: A version-scoped learning objective with a label and maximum score.

**Criterion assessment**: A human examiner's result for one rubric criterion. An assessed result cites transcript events and a score; a not-assessed result has no score and explains why.

## Document extraction quality

**Extraction report**: An immutable, checksum- and parser-version-scoped manifest of expected, extracted, uncertain, and critical document regions. The API derives missing regions and report status; the record does not contain the source file or extracted prose.

**Extraction review**: A distinct operator's append-only decision over the report. Approval requires an active extraction grant, a clean scan state, complete region coverage, and explicit verification of every critical or uncertain region.
