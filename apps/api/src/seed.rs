//! Synthetic fixture content for tests and local exploration. Deliberately
//! fictional ("gloopoid gland") so nothing here can be mistaken for medical
//! material (§27: synthetic fixtures only).

use domain_contracts::option_count;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

#[derive(serde::Serialize, serde::Deserialize)]
#[cfg_attr(
    feature = "type-export",
    derive(ts_rs::TS),
    ts(
        export,
        export_to = "packs/QuestionOption.ts",
        rename = "QuestionOption"
    )
)]
pub struct QuestionOption {
    pub text: String,
    pub rationale: String,
}

pub struct SeedIds {
    pub exam_id: Uuid,
    pub chapter1: Uuid,
    pub chapter2: Uuid,
    pub chapter3: Uuid,
    pub question_versions: [Uuid; 5],
}

struct NewQuestion {
    chapter: Uuid,
    difficulty: &'static str,
    vignette: String,
    lead_in: &'static str,
    options: Vec<QuestionOption>,
    correct: usize,
    key_point: &'static str,
    exam_tip: Option<&'static str>,
    high_yield: bool,
}

async fn insert_question(pool: &PgPool, q: &NewQuestion) -> Result<Uuid, ApiError> {
    // QB-11: the 2..=10 option rule lives in the shared crate, so the fixture
    // goes through the same validation real ingestion will use.
    let _count = option_count(q.options.len()).map_err(|_| {
        ApiError::unprocessable(
            "invalid_option_count",
            "question options must number between 2 and 10",
        )
    })?;
    let qid = Uuid::new_v4();
    let vid = Uuid::new_v4();
    let options = serde_json::to_value(&q.options).map_err(|_| ApiError::internal())?;
    sqlx::query!(
        "INSERT INTO questions (id, family_id) VALUES ($1, $2)",
        qid,
        qid
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        r#"INSERT INTO question_versions
           (id, question_id, version, status, chapter_id, difficulty, vignette,
            lead_in, options, correct_index, key_learning_point, exam_tip,
            high_yield, source_ref)
           VALUES ($1, $2, 1, 'published', $3, $4, $5, $6, $7, $8, $9, $10,
                   $11, $12)"#,
        vid,
        qid,
        q.chapter,
        q.difficulty,
        q.vignette,
        q.lead_in,
        options,
        q.correct as i16,
        q.key_point,
        q.exam_tip,
        q.high_yield,
        "Synthetic CI fixture - fictional content, not medical material",
    )
    .execute(pool)
    .await?;
    Ok(vid)
}

async fn ensure_synthetic_question_rights(pool: &PgPool) -> ApiResult<()> {
    const SOURCE_REF: &str = "Synthetic CI fixture - fictional content, not medical material";
    const RIGHTS_REF: &str = "MEDICALOS-SYNTHETIC-SEED";

    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM question_versions WHERE source_ref = $1)")
            .bind(SOURCE_REF)
            .fetch_one(pool)
            .await?;
    if !exists {
        return Ok(());
    }

    let asset_refs: serde_json::Value = sqlx::query_scalar(
        "SELECT COALESCE(
             jsonb_agg(DISTINCT to_jsonb(BTRIM(asset_ref.value))),
             '[]'::jsonb
         )
         FROM question_versions qv
         CROSS JOIN LATERAL unnest(
             ARRAY[qv.source_ref] || qv.source_refs || qv.media_refs
         ) AS asset_ref(value)
         WHERE qv.source_ref = $1 AND BTRIM(asset_ref.value) <> ''",
    )
    .bind(SOURCE_REF)
    .fetch_one(pool)
    .await?;
    sqlx::query(
        "INSERT INTO content_rights
             (id, ref_code, licensor, territory, permitted_uses, valid_from,
              notes, asset_refs, audiences)
         VALUES ($1, $2, $3, 'worldwide', $4, DATE '2020-01-01', $5, $6, $7)
         ON CONFLICT (ref_code) DO UPDATE
         SET permitted_uses = EXCLUDED.permitted_uses
         WHERE content_rights.notes = EXCLUDED.notes",
    )
    .bind(Uuid::new_v4())
    .bind(RIGHTS_REF)
    .bind("Medical OS fictional seed content")
    .bind(serde_json::json!([
        "display",
        "derivatives",
        "offline",
        "distribution"
    ]))
    .bind("Synthetic local/test fixtures only; not a third-party license grant.")
    .bind(asset_refs)
    .bind(serde_json::json!(["learners"]))
    .execute(pool)
    .await?;
    sqlx::query(
        "UPDATE question_versions SET rights_ref = $1
         WHERE source_ref = $2 AND rights_ref IS NULL",
    )
    .bind(RIGHTS_REF)
    .bind(SOURCE_REF)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn seed(pool: &PgPool) -> ApiResult<SeedIds> {
    // Idempotency: `api --seed` may be re-run against an already-seeded
    // database (local exploration), and re-inserting would violate
    // exams_code_key. Return the existing fixture instead. Integration
    // tests truncate in setup(), so they always take the full-seed path.
    // Runtime sqlx (not query!) so no offline-cache entry is needed.
    let exam_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM exams WHERE code = 'PILT' ORDER BY id LIMIT 1")
            .fetch_optional(pool)
            .await?;
    if let Some(exam_id) = exam_id {
        ensure_synthetic_question_rights(pool).await?;
        let nil = Uuid::nil();
        let chapters: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM curriculum_nodes WHERE exam_id = $1 AND kind = 'chapter'
             ORDER BY display_order, id LIMIT 3",
        )
        .bind(exam_id)
        .fetch_all(pool)
        .await?;
        let versions: Vec<Uuid> = sqlx::query_scalar(
            "SELECT qv.id FROM question_versions qv
             JOIN curriculum_nodes c ON c.id = qv.chapter_id
             WHERE c.exam_id = $1 ORDER BY qv.id LIMIT 5",
        )
        .bind(exam_id)
        .fetch_all(pool)
        .await?;
        let pick = |v: &[Uuid], i: usize| v.get(i).copied().unwrap_or(nil);
        return Ok(SeedIds {
            exam_id,
            chapter1: pick(&chapters, 0),
            chapter2: pick(&chapters, 1),
            chapter3: pick(&chapters, 2),
            question_versions: [
                pick(&versions, 0),
                pick(&versions, 1),
                pick(&versions, 2),
                pick(&versions, 3),
                pick(&versions, 4),
            ],
        });
    }

    let exam_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO exams (id, code, name, official_source_url, aliases)
         VALUES ($1, $2, $3, $4, $5)",
        exam_id,
        "PILT",
        "Pilot Exam (synthetic fixture)",
        "https://fixtures.example.test/pilot-blueprint",
        serde_json::json!(["PILT-DEMO", "Fixture Licensing Exam"]),
    )
    .execute(pool)
    .await?;

    async fn insert_node(
        pool: &PgPool,
        exam_id: Uuid,
        kind: &'static str,
        name: &str,
        parent: Option<Uuid>,
        order: i32,
    ) -> ApiResult<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO curriculum_nodes (id, exam_id, kind, name, parent_id, display_order)
             VALUES ($1, $2, $3, $4, $5, $6)",
            id,
            exam_id,
            kind,
            name,
            parent,
            order
        )
        .execute(pool)
        .await?;
        Ok(id)
    }

    let subject = insert_node(pool, exam_id, "subject", "Fictional Systems", None, 0).await?;
    let system = insert_node(pool, exam_id, "system", "Gloopoid Axis", Some(subject), 0).await?;
    let chapter1 = insert_node(
        pool,
        exam_id,
        "chapter",
        "Gloopoid Physiology",
        Some(system),
        0,
    )
    .await?;
    let chapter2 = insert_node(
        pool,
        exam_id,
        "chapter",
        "Glorbin Measurement",
        Some(system),
        1,
    )
    .await?;

    let mk = |chapter: Uuid,
              difficulty: &'static str,
              vignette: String,
              lead_in: &'static str,
              options: Vec<(&'static str, &'static str)>,
              correct: usize,
              key_point: &'static str,
              exam_tip: Option<&'static str>,
              high_yield: bool| NewQuestion {
        chapter,
        difficulty,
        vignette,
        lead_in,
        options: options
            .into_iter()
            .map(|(t, r)| QuestionOption {
                text: t.to_string(),
                rationale: r.to_string(),
            })
            .collect(),
        correct,
        key_point,
        exam_tip,
        high_yield,
    };

    let q1 = mk(
        chapter1,
        "easy",
        "In the fictional endocrine model, the gloopoid gland secretes glorbin \
         when stimulated by hormone Z. Rising glorbin levels are known to act \
         back on hormone Z release."
            .to_string(),
        "What happens to hormone Z secretion as glorbin rises?",
        vec![
            (
                "It decreases",
                "Correct: the fixture models classic negative feedback.",
            ),
            (
                "It increases",
                "That would be positive feedback, not this model.",
            ),
            (
                "It stops permanently",
                "Nothing in the fixture implies permanence.",
            ),
            (
                "It is unaffected",
                "The fixture states glorbin acts back on Z.",
            ),
        ],
        0,
        "In classic negative-feedback loops, rising product suppresses the upstream signal.",
        Some("Feedback direction questions: identify the loop before the option."),
        true,
    );
    let q2 = mk(
        chapter1,
        "medium",
        "A fictional biopsy of the gloopoid gland shows normal glorbin synthesis \
         but absent storage granules, and blood glorbin is low."
            .to_string(),
        "Which step of the fictional pathway is defective?",
        vec![
            ("Synthesis", "Synthesis is described as normal."),
            (
                "Storage and packaging",
                "Correct: made but not stored, so release is low.",
            ),
            (
                "Receptor binding",
                "Receptors act after secretion; synthesis reached blood.",
            ),
            (
                "Degradation",
                "Low - not high - blood levels argue against excess breakdown.",
            ),
        ],
        1,
        "Normal synthesis plus absent granules points to the storage step, not production.",
        None,
        false,
    );
    let q3 = mk(
        chapter2,
        "easy",
        "A fictional lab reports glorbin 40 mu/mL. The fictional reference range \
         is 10-50 mu/mL."
            .to_string(),
        "Which interpretation is correct?",
        vec![
            (
                "Within the fictional reference range",
                "Correct: 40 lies between 10 and 50.",
            ),
            (
                "Above the fictional reference range",
                "40 is below the upper limit of 50.",
            ),
            (
                "Below the fictional reference range",
                "40 is above the lower limit of 10.",
            ),
            (
                "Uninterpretable without repeats",
                "The fixture defines a single interpretable value.",
            ),
        ],
        0,
        "Compare the value against both bounds of the reference range before interpreting.",
        None,
        false,
    );
    let q4 = mk(
        chapter2,
        "medium",
        "In the fictional model, drug G-blocker competitively inhibits glorbin \
         receptors. A patient has normal blood glorbin and takes G-blocker."
            .to_string(),
        "What happens to glorbin's effect?",
        vec![
            (
                "It increases",
                "Inhibition reduces, not amplifies, the effect.",
            ),
            (
                "It decreases despite normal blood levels",
                "Correct: the receptor step is blocked.",
            ),
            (
                "It is unchanged because levels are normal",
                "Levels alone do not determine effect.",
            ),
            (
                "It converts to an agonist",
                "Nothing in the fixture implies conversion.",
            ),
        ],
        1,
        "Effect depends on both level and receptor action; blocking receptors lowers effect.",
        Some("Drug-mechanism questions: name the step being blocked first."),
        false,
    );

    let v1 = insert_question(pool, &q1).await?;
    let v2 = insert_question(pool, &q2).await?;
    let v3 = insert_question(pool, &q3).await?;
    let v4 = insert_question(pool, &q4).await?;

    // Single-question chapter: deterministic community-stats and mock flows
    // (every session here serves exactly this question).
    let chapter3 = insert_node(
        pool,
        exam_id,
        "chapter",
        "Glorbin Pharmacology",
        Some(system),
        2,
    )
    .await?;
    let q5 = mk(
        chapter3,
        "easy",
        "In the fictional model, drug G-boost increases receptor sensitivity          without changing blood glorbin."
            .to_string(),
        "What happens to glorbin's effect?",
        vec![
            ("It increases", "Correct: the receptor step is amplified."),
            ("It decreases", "Sensitivity raises, not lowers, the effect."),
            ("It is unchanged because levels are normal", "Levels alone do not determine effect."),
            ("It converts to an agonist", "Nothing in the fixture implies conversion."),
        ],
        0,
        "Effect can change through receptor sensitivity even when the level is stable.",
        None,
        false,
    );
    let v5 = insert_question(pool, &q5).await?;
    ensure_synthetic_question_rights(pool).await?;

    // EX-07 fixture: deterministic frozen form (both chapter-1 questions;
    // answering A on both yields exactly 1 correct = 50% = pass at mark 50).
    let blueprint = serde_json::json!([{ "chapter_id": chapter1, "count": 2 }]);
    let mock_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO mocks
           (id, title, exam_id, blueprint, time_limit_seconds, pass_mark_percent, attempts_allowed)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        mock_id,
        "Pilot Fixture Mock (full)",
        exam_id,
        blueprint,
        600,
        50,
        2
    )
    .execute(pool)
    .await?;

    Ok(SeedIds {
        exam_id,
        chapter1,
        chapter2,
        chapter3,
        question_versions: [v1, v2, v3, v4, v5],
    })
}
