# GROW-01 tails — share cards (score, consistency, league)

Requirements: GROW-01 (share cards), TRUST-01 (no fabricated numbers),
QB-15/ENG-01/COMP-01 (data sources)
Source: `MEDICAL_LEARNING_OS_MASTER_PLAN_v2.md` §26.1 ("share cards for
score, consistency and league results that never contain question content");
ledger row GROW-01.

## Scope

`GET /v1/me/share-cards` returns the learner's honestly available share
cards plus the ones that are unavailable and why:

- **Score** — last-30-day accuracy from real attempts; available only when
  the answered sample meets the community minimum (QB-15 gate). No speed
  component (§17 XP rule: nothing for speed).
- **Consistency** — the current daily-goal streak and how many of the last
  7 days met the goal (ENG-01 records only).
- **League** — the learner's most recent competition entry: rank, score,
  and competition title from the real leaderboard ordering; requires the
  opt-in handle the entry already carries.

Every card carries `share_text` for clipboard sharing. No card contains
question content, vignettes, or option text (GROW-01 rule). Unavailable
cards state the honest reason instead of fabricating numbers (TRUST-01).

The community page renders the cards with the shared design tokens, a
copy-to-clipboard action for `share_text`, and an SVG card download built
from the live token values.

Ambassador codes stay out of scope: issuing codes is an owner program
decision. Deep-link plumbing stays out of scope (Tauri-gated).

## Acceptance

- Fresh user: all three cards unavailable with honest reasons; `cards: []`.
- After enough answered attempts: score card with the exact real percent;
  streak card matches the engagement record; competition entry produces a
  rank card with the real title.
- No payload field ever contains question content.
- Integration tests cover available + unavailable paths for all three.
