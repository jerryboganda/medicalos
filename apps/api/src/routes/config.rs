//! OPS-06: staged-rollout config resolution. Each flag with
//! rollout_percent < 100 resolves per-user via a stable hash so the same
//! learner always lands in the same bucket (deterministic staged rollout).

use axum::extract::State;
use axum::Json;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::error::ApiResult;
use crate::state::AppState;

fn bucket(user_id: Uuid, key: &str) -> u32 {
    // Stable, dependency-free hash: FNV-1a over "user:key".
    let mut h: u32 = 0x811c_9dc5;
    let mut bytes = user_id.as_bytes().to_vec();
    bytes.extend_from_slice(key.as_bytes());
    for b in bytes {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h % 100
}

pub async fn config(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query!("SELECT key, value, rollout_percent FROM feature_flags ORDER BY key")
        .fetch_all(&state.pool)
        .await?;
    let flags: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let enabled = bucket(user.user_id, &r.key) < r.rollout_percent as u32;
            json!({ "key": r.key, "enabled": enabled })
        })
        .collect();
    Ok(Json(json!({ "flags": flags })))
}

// ---- OPS-06: forced / soft update + version compatibility metadata ----------

/// The client asks before it logs in. Backed by the `client_update` feature
/// flag; absent flag means "no update required" (honest default). Two-version
/// compatibility: `min_supported_client` names the oldest API contract this
/// deployment still answers.
pub async fn client_update(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query!("SELECT value FROM feature_flags WHERE key = 'client_update'")
        .fetch_optional(&state.pool)
        .await?;
    let version = env!("CARGO_PKG_VERSION");
    let mut update = json!({
        "mode": "none",
        "api_version": version,
        "min_supported_client": version,
        "recommended_version": version,
    });
    if let Some(r) = row {
        let flag = &r.value;
        let mode = flag.get("mode").and_then(|m| m.as_str()).unwrap_or("none");
        if !matches!(mode, "none" | "soft" | "forced") {
            // A misconfigured flag is an operator error, not a client one.
            return Err(crate::error::ApiError::internal());
        }
        update["mode"] = json!(mode);
        if let Some(min) = flag.get("min_supported_client") {
            update["min_supported_client"] = min.clone();
        }
        if let Some(rec) = flag.get("recommended_version") {
            update["recommended_version"] = rec.clone();
        }
    }
    Ok(Json(update))
}
