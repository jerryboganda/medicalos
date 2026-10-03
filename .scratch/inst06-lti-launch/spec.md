# INST-06 — LTI 1.3 launch and browser handoff

Status: implementation complete; CI accepted in run 37038719199; real-campus launch and certification remain external
Requirement IDs: INST-06, INST-01 (institution membership), §18.1.

## Problem statement

Before this slice, Medical OS verified an LTI 1.3 launch and issued its normal application session, but the browser received JSON. LMS users lacked a browser continuation, and instructors lacked a content picker for returning Medical OS content to the LMS. The implementation below closes those browser gaps; real campus acceptance remains external.

## Solution

Complete the two owner-authorized surfaces using the existing design system: a browser handoff that transfers the verified session and opens the launched article, and a deep-link picker that searches published library articles and submits selected LTI resource links to the platform.

## User stories

1. As an LMS learner, I want a valid resource-link launch to establish my Medical OS session and open the linked article.
2. As an LMS learner, I want an understandable message when a launch cannot restore browser storage or lacks a usable resource.
3. As an instructor, I want to search published Medical OS library articles and select only content available to learners.
4. As an instructor, I want the picker to honor the platform's accepted content type and single/multiple selection settings.
5. As an instructor, I want a clear return action that posts the signed deep-link response to the return URL saved from the verified launch.
6. As a user, I want launch data and tokens kept out of URLs, logs, and persistent browser history.

## Implementation decisions

- Keep the existing LTI API route and add HTML content negotiation for top-level browser form posts; API clients continue receiving the current JSON response.
- Transfer the issued token into the same-origin application storage in a nonce-protected, non-cacheable handoff document. Keep only non-secret launch context in session storage.
- Route deep-link requests to the picker and resource-link launches to the handoff page. Open the existing library article reader when the signed resource-link custom data identifies a valid article slug; otherwise provide a safe study-plan destination.
- Search only the existing published editorial-article results. Never offer private imports or learner-specific documents in the LMS picker.
- Return the existing signed `LtiResourceLink` response, with each item's URL pointing to the registered LTI launch endpoint and custom data identifying the article slug and optional jurisdiction.
- Keep the handoff and picker usable in secure LMS frames. Use the existing design-system tokens and components; do not introduce a new visual theme.
- Preserve app-wide `X-Frame-Options: DENY`. Add a route-local HTTPS frame policy only to the LTI launch HTML and `/lti/*` pages so LMS launches can render; all other paths keep the existing deny policy.
- Keep real LMS registration, account linking, key provisioning, and 1EdTech certification as external acceptance gates.

## Testing decisions

- Test the public browser handoff contract: HTML is returned only to HTML navigation, API callers keep JSON, tokens never appear in the redirect URL, and untrusted text cannot break the bootstrap script.
- Test the user-visible browser flow with Playwright: resource launch continuation, published article search, selection constraints, signed-response request, and form return to the platform.
- Test the HTTP header policy at the deployment boundary: LTI routes can render in an HTTPS parent frame while ordinary app routes retain `DENY`.
- Use existing API and Playwright behavior tests as prior art; do not assert private component implementation details.

## Out of scope

- Names and provisioning, assignment grades, launch-role authorization changes, or migration from localStorage to httpOnly cookies.
- Real-campus LMS certification and operator provisioning of LTI signing keys.
- Non-article content types, private imports, and new article-reader capabilities.

## Further notes

The current tool-side flow remains the source of truth for identity linking, nonce/state validation, session issuance, and the single-use deep-link return settings. The browser work must not weaken those checks.

Acceptance evidence: [GitHub Actions run 37038719199](https://github.com/jerryboganda/medicalos/actions/runs/37038719199), source commit `8f586d1`: all five jobs passed, including 156 Rust workspace tests, the 364-test type-export run, 77 Playwright tests, and containerized LTI frame-policy checks. Browser LTI fixtures and the mocked-campus API test do not replace real-campus certification.
