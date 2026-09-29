"""Register LTI routes in both lib.rs trees (CRLF-safe)."""
import io

path = "apps/api/src/lib.rs"
with io.open(path, encoding="utf-8", newline="") as f:
    s = f.read()
nl = "\r\n" if "\r\n" in s else "\n"

PAIRS = [
    ("/v1", "/api/v1"),
]

v1_old = (
    '        .route(' + nl
    + '            "/v1/institutions/{institution_id}/interop",' + nl
    + '            post(routes::program::record_interop),' + nl
    + '        )' + nl
)
v1_new = (
    '        .route(' + nl
    + '            "/v1/institutions/{institution_id}/interop",' + nl
    + '            post(routes::program::record_interop),' + nl
    + '        )' + nl
    + '        .route(' + nl
    + '            "/v1/institutions/{institution_id}/interop/lti-platforms",' + nl
    + '            post(routes::lti::register_platform),' + nl
    + '        )' + nl
    + '        .route("/v1/lti/login", get(routes::lti::login_get).post(routes::lti::login_post))' + nl
    + '        .route("/v1/lti/launch", post(routes::lti::launch))' + nl
    + '        .route("/v1/lti/jwks.json", get(routes::lti::jwks))' + nl
    + '        .route("/v1/lti/deep-links", post(routes::lti::deep_links))' + nl
)
api_old = v1_old.replace('"/v1/', '"/api/v1/')
api_new = v1_new.replace('"/v1/', '"/api/v1/')

assert s.count(v1_old) == 1, f"v1 anchor x{s.count(v1_old)}"
assert s.count(api_old) == 1, f"api anchor x{s.count(api_old)}"
s = s.replace(v1_old, v1_new).replace(api_old, api_new)

with io.open(path, "w", encoding="utf-8", newline="") as f:
    f.write(s)
print("routes registered")
