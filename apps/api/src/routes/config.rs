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
