//! Synthetic fixture content for tests and local exploration. Deliberately
//! fictional ("gloopoid gland") so nothing here can be mistaken for medical
//! material (§27: synthetic fixtures only).

use domain_contracts::option_count;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain_option;
use crate::error::{ApiError, ApiResult};

#[derive(serde::Serialize)]
pub struct QuestionOption {
    pub text: String,
    pub rationale: String,
}

pub struct SeedIds {
    pub exam_id: Uuid,
    pub chapter1: Uuid,
    pub chapter2: Uuid,
    pub question_versions: [Uuid; 4],
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
    let _count = option_count(q.options.len())?;
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

pub async fn seed(pool: &PgPool) -> ApiResult<SeedIds> {
    let exam_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO exams (id, code, name) VALUES ($1, $2, $3)",
        exam_id,
        "PILT",
        "Pilot Exam (synthetic fixture)"
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

    Ok(SeedIds {
        exam_id,
        chapter1,
        chapter2,
        question_versions: [v1, v2, v3, v4],
    })
}
