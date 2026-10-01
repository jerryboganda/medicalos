use crate::error::{ApiError, ApiResult};
use sqlx::PgPool;
use uuid::Uuid;

pub fn unavailable_error() -> ApiError {
    ApiError::forbidden(
        "rights_unavailable",
        "the question's current display rights are unavailable",
    )
}

/// Refuse question content when its current learner-display grant is missing
/// or no longer active. The SQL predicate is shared with question-pool queries.
pub async fn ensure_question_displayable(pool: &PgPool, version_id: Uuid) -> ApiResult<()> {
    let active = sqlx::query_scalar::<_, bool>(
        "SELECT question_display_rights_active(
             rights_ref, source_ref, source_refs, media_refs
         )
         FROM question_versions WHERE id = $1",
    )
    .bind(version_id)
    .fetch_optional(pool)
    .await?;

    match active {
        Some(true) => Ok(()),
        Some(false) => Err(unavailable_error()),
        None => Err(ApiError::not_found("question_not_found")),
    }
}
