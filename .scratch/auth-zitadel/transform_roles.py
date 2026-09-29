"""Role-aware admin gate transformation (auth-zitadel phase 3).

Rewrites require_admin call sites to require_permission with the route's
§18.1 permission, per the table in .scratch/auth-zitadel/spec.md.
Paren-matched insertion; run once, then `cargo fmt -p api`.
"""
import io
import re

TABLE = {
    "admin.rs": {
        "create_node": "ContentAuthor", "update_node": "ContentAuthor",
        "create_question": "ContentAuthor", "search_questions": "ContentAuthor",
        "import": "ContentAuthor", "import_file": "ContentAuthor",
        "rollback_import": "ContentAuthor", "assessment_workflow": "ClinicalApprove",
        "create_content_rights": "ContentAuthor", "list_content_rights": "ContentAuthor",
        "revoke_content_rights": "ContentAuthor", "list_extraction_reports": "ContentAuthor",
        "get_extraction_report": "ContentAuthor", "create_extraction_report": "ContentAuthor",
        "review_extraction_report": "ClinicalApprove", "create_variant": "ContentAuthor",
        "create_incident": "ReportTriage", "list_incidents": "ReportTriage",
        "update_incident": "ReportTriage",
    },
    "concepts.rs": {p: "ContentAuthor" for p in ["list", "create", "create_version", "node_mappings", "set_node_mappings"]},
    "exams.rs": {"create_exam_spec": "ExamConfigure", "create_assessment_form": "ExamConfigure", "qti_export": "ExamConfigure"},
    "library.rs": {
        "admin_list_articles": "ContentAuthor", "create_article": "ContentAuthor",
        "create_article_version": "ContentAuthor", "get_admin_article_version": "ContentAuthor",
        "attach_media": "ContentAuthor", "create_image_case": "ContentAuthor",
        "admin_image_case_concepts": "ContentAuthor", "set_admin_image_case_concepts": "ContentAuthor",
        "create_image_annotation": "ContentAuthor", "list_image_annotations": "ClinicalApprove",
        "review_image_annotation": "ClinicalApprove",
    },
    "mock.rs": {"create_mock": "ExamConfigure"},
    "program.rs": {"generate_pregen": "ContentAuthor", "create_scenario": "ContentAuthor", "create_scenario_version": "ContentAuthor"},
    "reports.rs": {"review_queue": "ReportTriage", "resolve": "ReportTriage"},
    "sim.rs": {p: "ExamAssess" for p in ["pending_assessments", "admin_assessment", "record_assessment",
               "list_scenario_assessment_appeals", "get_scenario_assessment_appeal", "review_scenario_assessment_appeal"]},
    "source_changes.rs": {p: "ContentAuthor" for p in ["register_passage", "link_dependency", "record_change", "get_change", "resolve_task"]},
    "community.rs": {"prize_review": "ReportTriage"},
}

# wrapper fn name -> (hardcoded domain | None for caller-passed param)
WRAPPERS = {
    "concepts.rs": ("require_admin", "ContentAuthor"),
    "exams.rs": ("admin", "ExamConfigure"),
    "library.rs": ("require_article_admin", None),
    "program.rs": ("admin", None),
    "sim.rs": ("require_admin", "ExamAssess"),
    "source_changes.rs": ("require_admin", "ContentAuthor"),
}


def match_paren(s, open_idx):
    depth = 0
    for i in range(open_idx, len(s)):
        if s[i] == "(":
            depth += 1
        elif s[i] == ")":
            depth -= 1
            if depth == 0:
                return i
    raise ValueError("unbalanced parens")


def fn_spans(s):
    spans = []
    starts = [m for m in re.finditer(r"(?:pub )?async fn (\w+)\(", s)]
    for i, m in enumerate(starts):
        end = starts[i + 1].start() if i + 1 < len(starts) else len(s)
        spans.append((m.start(), end, m.group(1)))
    return spans


def enclosing(spans, idx):
    for start, end, name in spans:
        if start <= idx < end:
            return name
    return None


for fname, table in TABLE.items():
    path = f"apps/api/src/routes/{fname}"
    with io.open(path, encoding="utf-8", newline="") as f:
        s = f.read()

    wrapper, wdomain = WRAPPERS.get(fname, (None, None))
    spans = fn_spans(s)

    # every require_admin / admin( / require_article_admin( call site
    sites = [m for m in re.finditer(r"(?:require_admin|require_article_admin|(?<![\w_])admin)\(", s)
             if "fn " not in s[max(0, m.start() - 30):m.start()]]

    # decide per site
    for m in reversed(sites):
        idx = m.start()
        fn = enclosing(spans, idx)
        is_wrapper_call = wrapper is not None and s[idx:].startswith(wrapper + "(")
        if fn in table:
            perm = table[fn]
        elif fn is None and is_wrapper_call:
            # the wrapper's own body: forward its param or hardcode the domain
            perm = wdomain if wdomain else "permission"
        else:
            continue  # PlatformOps keeper
        open_idx = s.find("(", idx)
        close = match_paren(s, open_idx)
        rename_only = is_wrapper_call and wdomain is not None
        if not rename_only:
            s = s[:close] + ", Permission::" + perm + s[close:]
        s = s[:idx] + "require_permission" + s[idx + len(m.group(0)) - 1:]

    # wrapper def: rename + (mixed-domain only) accept the caller's permission
    if wrapper:
        s = s.replace(f"fn {wrapper}(", "fn require_permission(", 1)

    if "Permission::" in s and "use crate::authz::Permission;" not in s:
        for anchor in ("use crate::state::AppState;", "use crate::auth::AuthUser;"):
            if anchor in s:
                s = s.replace(anchor, anchor + "\nuse crate::authz::Permission;", 1)
                break

    with io.open(path, "w", encoding="utf-8", newline="") as f:
        f.write(s)
    n_perm = s.count("Permission::")
    n_admin = len(re.findall(r"require_admin\(", s))
    print(f"{fname}: perm-refs={n_perm} require_admin-left={n_admin}")
