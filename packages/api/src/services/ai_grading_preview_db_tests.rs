//! Database-backed allowed/denied matrix for teacher AI preview authorization.
//! Fixtures deliberately live outside server_functions: production server
//! functions must never contain or accept an unscoped PgPool.

use super::{find_authorized_grading_source, AuthorizedGradingSource};
use sqlx::Row;
use uuid::Uuid;

async fn source_as(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    school_id: Uuid,
    submission_id: Uuid,
) -> Option<AuthorizedGradingSource> {
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    let actor =
        AuthorizedActor::new(user_id, "Teacher", Some(school_id)).expect("valid test actor");
    let tx = AuthorizedTx::begin(pool, actor)
        .await
        .expect("begin authorized grading test transaction");
    let authorized_pool = tx.authorized_pool();
    tx.scope(
        find_authorized_grading_source(&authorized_pool, submission_id, user_id),
        |result| result.is_ok(),
    )
    .await
    .expect("complete grading authorization transaction")
    .expect("lookup grading context")
}

/// The PDF metadata source is independent of the existing text-only preview.
/// Both are read inside the same real PostgreSQL Teacher RLS transaction.
async fn pdf_source_as(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    role: &str,
    school_id: Uuid,
    submission_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
) -> Option<crate::services::ai_grading_pdf_source::AuthorizedPdfMetadata> {
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    let actor = AuthorizedActor::new(user_id, role, Some(school_id))
        .expect("valid RLS native PDF grading test actor");
    let tx = AuthorizedTx::begin(pool, actor)
        .await
        .expect("begin authorized native PDF test transaction");
    let scoped = tx.authorized_pool();
    tx.scope(
        crate::services::ai_grading_pdf_source::find_authorized_pdf_metadata(
            &scoped,
            submission_id,
            user_id,
            revision,
            approval_id,
        ),
        Result::is_ok,
    )
    .await
    .expect("complete native PDF authorization transaction")
    .expect("native PDF metadata lookup")
}

/// The PDF egress gate is checked with the same request-scoped Teacher RLS
/// transaction that authorizes a grading suggestion.
async fn pdf_egress_enabled_as(raw: &sqlx::PgPool, teacher_user_id: Uuid, school_id: Uuid) -> bool {
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    let actor = AuthorizedActor::new(teacher_user_id, "Teacher", Some(school_id))
        .expect("valid PDF school gate actor");
    let tx = AuthorizedTx::begin(raw, actor)
        .await
        .expect("begin PDF gate RLS transaction");
    let scoped = tx.authorized_pool();
    tx.scope(
        crate::services::ai_grading_pdf_source::school_pdf_grading_enabled(&scoped, school_id),
        Result::is_ok,
    )
    .await
    .expect("finish PDF gate RLS transaction")
    .expect("read school PDF opt-in")
}

/// Exercise the SECURITY DEFINER budget gate with explicit request actor.
/// Fails closed without configured limits or current Teacher/rubric/PDF rights.
async fn pdf_budget_reserve_as(
    raw: &sqlx::PgPool,
    user_id: Uuid,
    role: &str,
    school_id: Uuid,
    submission_id: Uuid,
    revision: Uuid,
    approval_id: Uuid,
    bytes: i64,
) -> bool {
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    let actor = AuthorizedActor::new(user_id, role, Some(school_id))
        .expect("valid budget actor");
    let tx = AuthorizedTx::begin(raw, actor)
        .await
        .expect("budget authorized transaction");
    let scoped = tx.authorized_pool();
    tx.scope(
        sqlx::query_scalar::<_, bool>(
            "SELECT public.reserve_ai_grading_pdf_budget($1,$2,$3,$4,$5)",
        )
        .bind(school_id)
        .bind(submission_id)
        .bind(revision)
        .bind(approval_id)
        .bind(bytes)
        .fetch_one(&scoped),
        Result::is_ok,
    )
    .await
    .expect("budget transaction completion")
    .expect("budget reservation query")
}

#[tokio::test]
async fn school_and_teacher_authorization_matrix() {
    use sqlx::PgPool;

    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL required for grading authorization database proof");
    let pool = PgPool::connect(&url)
        .await
        .expect("connect test PostgreSQL");
    let suffix = Uuid::new_v4().simple().to_string();
    let school_a = Uuid::new_v4();
    let school_b = Uuid::new_v4();
    let subject_id = Uuid::new_v4();
    let class_a = Uuid::new_v4();
    let class_b = Uuid::new_v4();
    let teacher_a_user = Uuid::new_v4();
    let teacher_peer_user = Uuid::new_v4();
    let teacher_b_user = Uuid::new_v4();
    let student_a_user = Uuid::new_v4();
    let student_b_user = Uuid::new_v4();
    let teacher_a = Uuid::new_v4();
    let teacher_peer = Uuid::new_v4();
    let teacher_b = Uuid::new_v4();
    let student_a = Uuid::new_v4();
    let student_b = Uuid::new_v4();

    let teacher_role: Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE name::text = 'Teacher' LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("teacher role");
    let student_role: Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE name::text = 'Student' LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("student role");
    for (school, name) in [(school_a, "A"), (school_b, "B")] {
        sqlx::query("INSERT INTO schools (id, name) VALUES ($1, $2)")
            .bind(school)
            .bind(format!("Grading preview {name} {suffix}"))
            .execute(&pool)
            .await
            .expect("school fixture");
    }

    for (user_id, name, role, school) in [
        (teacher_a_user, "Teacher A", teacher_role, school_a),
        (
            teacher_peer_user,
            "Same-school peer",
            teacher_role,
            school_a,
        ),
        (teacher_b_user, "Teacher B", teacher_role, school_b),
        (student_a_user, "Student A", student_role, school_a),
        (student_b_user, "Student B", student_role, school_b),
    ] {
        sqlx::query(
            "INSERT INTO users (id, name, email, role_id, school_id, is_active, metadata) \
             VALUES ($1, $2, $3, $4, $5, TRUE, '{}'::jsonb)",
        )
        .bind(user_id)
        .bind(name)
        .bind(format!("{user_id}@example.test"))
        .bind(role)
        .bind(school)
        .execute(&pool)
        .await
        .expect("user fixture");
    }

    for (id, user, school) in [
        (teacher_a, teacher_a_user, school_a),
        (teacher_peer, teacher_peer_user, school_a),
        (teacher_b, teacher_b_user, school_b),
    ] {
        sqlx::query(
            "INSERT INTO teachers (id, user_id, school_id, subject, created_at) \
             VALUES ($1, $2, $3, NULL, NOW())",
        )
        .bind(id)
        .bind(user)
        .bind(school)
        .execute(&pool)
        .await
        .expect("teacher fixture");
    }

    for (id, user, school) in [
        (student_a, student_a_user, school_a),
        (student_b, student_b_user, school_b),
    ] {
        sqlx::query(
            "INSERT INTO students \
             (id, user_id, school_id, parent_id, talent_profile_ref, created_at) \
             VALUES ($1, $2, $3, NULL, NULL, NOW())",
        )
        .bind(id)
        .bind(user)
        .bind(school)
        .execute(&pool)
        .await
        .expect("student fixture");
    }

    sqlx::query("INSERT INTO subjects (id, code, name) VALUES ($1, $2, $3)")
        .bind(subject_id)
        .bind(format!("GRADE-{suffix}"))
        .bind("Grading authorization subject")
        .execute(&pool)
        .await
        .expect("subject fixture");

    for (class, school, name) in [(class_a, school_a, "A"), (class_b, school_b, "B")] {
        sqlx::query(
            "INSERT INTO class_sections \
             (id, school_id, subject_id, name, term) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(class)
        .bind(school)
        .bind(subject_id)
        .bind(format!("Grading class {name} {suffix}"))
        .bind("2026")
        .execute(&pool)
        .await
        .expect("class fixture");
    }

    for (class, teacher, student) in [
        (class_a, teacher_a, student_a),
        (class_b, teacher_b, student_b),
    ] {
        sqlx::query(
            "INSERT INTO teaching_assignments \
             (id, class_section_id, teacher_id) VALUES ($1, $2, $3)",
        )
        .bind(Uuid::new_v4())
        .bind(class)
        .bind(teacher)
        .execute(&pool)
        .await
        .expect("teaching assignment fixture");

        sqlx::query(
            "INSERT INTO enrollments \
             (id, class_section_id, student_id, enrolled_at) \
             VALUES ($1, $2, $3, NOW())",
        )
        .bind(Uuid::new_v4())
        .bind(class)
        .bind(student)
        .execute(&pool)
        .await
        .expect("enrollment fixture");
    }

    let mut submissions = Vec::new();
    for (teacher, class, student) in [
        (teacher_a, class_a, student_a),
        (teacher_b, class_b, student_b),
    ] {
        let assignment = Uuid::new_v4();
        let custom = Uuid::new_v4();
        let submission = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO assignments \
             (id, teacher_id, class_section_id, subject_id, title, body, \
              due_at, status, material_ids) \
             VALUES ($1, $2, $3, $4, $5, $6, NOW() + INTERVAL '7 days', \
              'Published'::assignment_status, '{}'::uuid[])",
        )
        .bind(assignment)
        .bind(teacher)
        .bind(class)
        .bind(subject_id)
        .bind("Teacher-approved assessment")
        .bind("Show your reasoning.")
        .execute(&pool)
        .await
        .expect("assignment fixture");

        sqlx::query(
            "INSERT INTO custom_assignments \
             (id, assignment_id, student_id, due_at, status, submitted_at) \
             VALUES ($1, $2, $3, NOW() + INTERVAL '7 days', \
              'Submitted'::custom_status, NOW())",
        )
        .bind(custom)
        .bind(assignment)
        .bind(student)
        .execute(&pool)
        .await
        .expect("custom assignment fixture");

        sqlx::query(
            "INSERT INTO submissions \
             (id, custom_assignment_id, student_id, content, submitted_at, grade_scale) \
             VALUES ($1, $2, $3, $4, NOW(), 20)",
        )
        .bind(submission)
        .bind(custom)
        .bind(student)
        .bind(serde_json::json!({"text": "The answer is 42."}))
        .execute(&pool)
        .await
        .expect("submission fixture");
        submissions.push(submission);
    }

    let own = source_as(&pool, teacher_a_user, school_a, submissions[0])
        .await
        .expect("assigned teacher may preview submitted text");
    assert_eq!(own.school_id, school_a);
    assert_eq!(own.grade_scale, 20);
    assert_eq!(own.submission_text, "The answer is 42.");
    assert_eq!(own.assignment_instructions, "Show your reasoning.");

    assert!(
        source_as(&pool, teacher_peer_user, school_a, submissions[0])
            .await
            .is_none(),
        "a same-school teacher without the assignment must be denied"
    );
    assert!(
        source_as(&pool, teacher_b_user, school_b, submissions[0])
            .await
            .is_none(),
        "cross-school reading must be denied"
    );
    assert!(
        source_as(&pool, teacher_a_user, school_a, submissions[1])
            .await
            .is_none(),
        "cross-school submission enumeration must be denied"
    );
    assert!(
        source_as(&pool, student_a_user, school_a, submissions[0])
            .await
            .is_none(),
        "students may not masquerade as teacher preview callers"
    );

    sqlx::query("UPDATE submissions SET revision = gen_random_uuid() WHERE id = $1")
        .bind(submissions[0])
        .execute(&pool)
        .await
        .expect("update submission revision");
    let changed = source_as(&pool, teacher_a_user, school_a, submissions[0])
        .await
        .expect("teacher can load revised submission");
    assert_ne!(own.submission_revision, changed.submission_revision);

    // Explicit teacher approval must be independently protected by RLS and
    // database triggers; the generated personalization rubric is not authority.
    let custom_id: Uuid =
        sqlx::query_scalar("SELECT custom_assignment_id FROM submissions WHERE id = $1")
            .bind(submissions[0])
            .fetch_one(&pool)
            .await
            .expect("custom assignment fixture");

    let criteria = serde_json::json!([
        {"criterion_id": "استدلال", "possible_points": 12.0},
        {"criterion_id": "accuracy", "possible_points": 8.0}
    ]);

    async fn attempt_approval(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        role: &str,
        school_id: Uuid,
        custom_id: Uuid,
        teacher_id: Uuid,
        criteria: serde_json::Value,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor =
            AuthorizedActor::new(user_id, role, Some(school_id)).expect("valid rubric test actor");
        let tx = AuthorizedTx::begin(pool, actor)
            .await
            .expect("begin rubric test transaction");
        let scoped = tx.authorized_pool();
        tx.scope(
            sqlx::query_scalar::<_, Uuid>(
                r#"
                INSERT INTO public.ai_grading_rubric_approvals
                    (custom_assignment_id, school_id, teacher_id, grade_scale,
                     criteria, rubric_fingerprint)
                VALUES ($1, $2, $3, 20, $4, repeat('0', 64))
                ON CONFLICT DO NOTHING RETURNING id
                "#,
            )
            .bind(custom_id)
            .bind(school_id)
            .bind(teacher_id)
            .bind(criteria)
            .fetch_optional(&scoped),
            |result| result.is_ok(),
        )
        .await
        .expect("finish rubric test transaction")
    }

    assert!(
        attempt_approval(
            &pool,
            teacher_peer_user,
            "Teacher",
            school_a,
            custom_id,
            teacher_a,
            criteria.clone()
        )
        .await
        .is_err(),
        "same-school unassigned teacher may not approve another teacher's rubric"
    );
    assert!(
        attempt_approval(
            &pool,
            teacher_b_user,
            "Teacher",
            school_b,
            custom_id,
            teacher_a,
            criteria.clone()
        )
        .await
        .is_err(),
        "cross-school teacher may not approve a rubric"
    );
    assert!(
        attempt_approval(
            &pool,
            student_a_user,
            "Student",
            school_a,
            custom_id,
            teacher_a,
            criteria.clone()
        )
        .await
        .is_err(),
        "students may not authorize grading criteria"
    );

    let rubric_id = attempt_approval(
        &pool,
        teacher_a_user,
        "Teacher",
        school_a,
        custom_id,
        teacher_a,
        criteria.clone(),
    )
    .await
    .expect("own teacher approval succeeds")
    .expect("new approval row");
    assert!(
        attempt_approval(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            custom_id,
            teacher_a,
            criteria.clone()
        )
        .await
        .expect("idempotent duplicate uses ON CONFLICT DO NOTHING")
        .is_none(),
        "one active approval per personalized assignment"
    );

    let stored_fingerprint: String = sqlx::query_scalar(
        "SELECT rubric_fingerprint FROM ai_grading_rubric_approvals WHERE id=$1",
    )
    .bind(rubric_id)
    .fetch_one(&pool)
    .await
    .expect("fingerprint exists");
    assert_eq!(stored_fingerprint.len(), 64);
    assert_ne!(stored_fingerprint, "0".repeat(64));

    // A teacher approving the rubric after a student already finalized text
    // must create exactly one durable, private queue job; duplicate requests
    // cannot fan out duplicate inference.
    let queued_at_approval: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2 \
           AND rubric_approval_id=$3 AND status='queued'",
    )
    .bind(submissions[0])
    .bind(changed.submission_revision)
    .bind(rubric_id)
    .fetch_one(&pool)
    .await
    .expect("queue created on late teacher approval");
    assert_eq!(queued_at_approval, 1);

    // A queued job is claimable only by the bounded system-job actor.
    // The separate teacher/Student actor cannot call queue functions simply
    // by knowing a job UUID.
    let worker_id = Uuid::new_v4();
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(student_a_user, "Student", Some(school_a))
            .expect("student queue negative actor");
        let tx = AuthorizedTx::begin(&pool, actor)
            .await
            .expect("student claim negative transaction");
        let scoped = tx.authorized_pool();
        let result = tx
            .scope(
                sqlx::query("SELECT * FROM public.claim_next_ai_grading_job($1)")
                    .bind(worker_id)
                    .fetch_optional(&scoped),
                |_| false,
            )
            .await
            .expect("student claim rollback");
        assert!(result.is_err(), "student must not claim private AI work");
    }

    let claimed_job: Uuid;
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(worker_id))
            .await
            .expect("bounded queue claim transaction");
        let scoped = tx.authorized_pool();
        let row = tx
            .scope(
                sqlx::query("SELECT * FROM public.claim_next_ai_grading_job($1)")
                    .bind(worker_id)
                    .fetch_optional(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("finish claim transaction")
            .expect("claim query")
            .expect("eligible text-only work is claimable");
        claimed_job = row.get("job_id");
        assert_eq!(row.get::<Uuid, _>("submission_id"), submissions[0]);
        assert_eq!(row.get::<Uuid, _>("rubric_approval_id"), rubric_id);
        assert_eq!(
            row.get::<Uuid, _>("submission_revision"),
            changed.submission_revision
        );
        assert_eq!(row.get::<i32, _>("attempt_count"), 1);
    }
    // The scheduler may learn only the assigned active teacher identity.
    // It cannot declare a job successful without matching persisted evidence.
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(worker_id))
            .await
            .expect("bounded teacher resolution transaction");
        let scoped = tx.authorized_pool();
        let (teacher, incomplete): (Option<Uuid>, bool) = tx
            .scope(
                async {
                    let teacher =
                        sqlx::query_scalar("SELECT public.resolve_ai_grading_job_teacher($1,$2)")
                            .bind(worker_id)
                            .bind(claimed_job)
                            .fetch_one(&scoped)
                            .await?;
                    let incomplete =
                        sqlx::query_scalar("SELECT public.complete_ai_grading_job($1,$2,$3)")
                            .bind(worker_id)
                            .bind(claimed_job)
                            .bind(Uuid::new_v4())
                            .fetch_one(&scoped)
                            .await?;
                    Ok::<_, sqlx::Error>((teacher, incomplete))
                },
                |result| result.is_ok(),
            )
            .await
            .expect("finish bounded teacher discovery")
            .expect("worker metadata lookups");
        assert_eq!(teacher, Some(teacher_a_user));
        assert!(!incomplete, "completion without evidence must be denied");
    }
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(worker_id))
            .await
            .expect("bounded heartbeat transaction");
        let scoped = tx.authorized_pool();
        let heartbeat: bool = tx
            .scope(
                sqlx::query_scalar("SELECT public.heartbeat_ai_grading_job($1,$2)")
                    .bind(worker_id)
                    .bind(claimed_job)
                    .fetch_one(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("finish heartbeat transaction")
            .expect("active lease heartbeat");
        assert!(heartbeat, "active lease can be renewed");
    }
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(worker_id))
            .await
            .expect("bounded failure transaction");
        let scoped = tx.authorized_pool();
        let state: Option<String> = tx
            .scope(
                sqlx::query_scalar("SELECT public.record_ai_grading_job_failure($1,$2,$3,$4,$5)")
                    .bind(worker_id)
                    .bind(claimed_job)
                    .bind("rate_limited")
                    .bind(true)
                    .bind(5_i32)
                    .fetch_one(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("finish bounded failure")
            .expect("failure query");
        assert_eq!(state.as_deref(), Some("queued"));
    }
    // Synthesize a worker crash and expire its lease, without sleeping;
    // recovery must release ownership and preserve an auditable attempt.
    sqlx::query(
        "UPDATE public.ai_grading_jobs \
         SET status='running',attempt_count=2,lease_owner=$2, \
             lease_until=NOW()-INTERVAL '1 second' WHERE id=$1",
    )
    .bind(claimed_job)
    .bind(worker_id)
    .execute(&pool)
    .await
    .expect("prepare expired lease");
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(Uuid::new_v4()))
            .await
            .expect("independent recovery worker");
        let recovery_worker = tx.actor().user_id;
        let scoped = tx.authorized_pool();
        let recovered: i64 = tx
            .scope(
                sqlx::query_scalar("SELECT public.recover_expired_ai_grading_jobs($1)")
                    .bind(recovery_worker)
                    .fetch_one(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("finish recovery transaction")
            .expect("recover expired lease");
        assert_eq!(recovered, 1);
    }
    let (status, current_lease): (String, Option<Uuid>) = {
        let row = sqlx::query("SELECT status,lease_owner FROM public.ai_grading_jobs WHERE id=$1")
            .bind(claimed_job)
            .fetch_one(&pool)
            .await
            .expect("recovered job metadata");
        (row.get("status"), row.get("lease_owner"))
    };
    assert_eq!(status, "queued");
    assert!(
        current_lease.is_none(),
        "expired lease owner must be cleared"
    );

    // An independent worker can reclaim a requeued job, but only the worker
    // holding the current valid lease may cancel it for a verified reason.
    let next_worker = Uuid::new_v4();
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(next_worker))
            .await
            .expect("reclaim recovered work");
        let scoped = tx.authorized_pool();
        let next_id: Uuid = tx
            .scope(
                async {
                    let row = sqlx::query("SELECT * FROM public.claim_next_ai_grading_job($1)")
                        .bind(next_worker)
                        .fetch_one(&scoped)
                        .await?;
                    Ok::<Uuid, sqlx::Error>(row.get("job_id"))
                },
                |result| result.is_ok(),
            )
            .await
            .expect("complete reclaim transaction")
            .expect("reclaim recovered job");
        assert_eq!(next_id, claimed_job);
    }
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let wrong_worker = Uuid::new_v4();
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(wrong_worker))
            .await
            .expect("wrong lease owner transaction");
        let scoped = tx.authorized_pool();
        let cancelled: bool = tx
            .scope(
                sqlx::query_scalar("SELECT public.cancel_ai_grading_job($1,$2,$3)")
                    .bind(wrong_worker)
                    .bind(claimed_job)
                    .bind("authorization_revoked")
                    .fetch_one(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("finish wrong lease transaction")
            .expect("wrong owner cancellation query");
        assert!(!cancelled, "another worker may not cancel an active lease");
    }
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(next_worker))
            .await
            .expect("owner cancellation transaction");
        let scoped = tx.authorized_pool();
        let cancelled: bool = tx
            .scope(
                sqlx::query_scalar("SELECT public.cancel_ai_grading_job($1,$2,$3)")
                    .bind(next_worker)
                    .bind(claimed_job)
                    .bind("authorization_revoked")
                    .fetch_one(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("finish owner cancellation transaction")
            .expect("owner cancellation query");
        assert!(cancelled);
    }

    // The test fixture uses a privileged PostgreSQL owner connection (which
    // bypasses RLS even when the transaction context says "Student"). Test
    // structure here and test the real NOBYPASSRLS runtime role in the CI
    // transaction-scoped RLS verification, not via a false actor simulation.
    let row = sqlx::query(
        "SELECT relrowsecurity, relforcerowsecurity FROM pg_class \
         WHERE oid = 'public.ai_grading_jobs'::regclass",
    )
    .fetch_one(&pool)
    .await
    .expect("grading queue RLS metadata");
    assert!(row.get::<bool, _>("relrowsecurity"));
    assert!(row.get::<bool, _>("relforcerowsecurity"));
    let policy_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_policies \
         WHERE schemaname = 'public' AND tablename = 'ai_grading_jobs'",
    )
    .fetch_one(&pool)
    .await
    .expect("grading queue policy inventory");
    assert_eq!(
        policy_count, 3,
        "only owner-scoped policies may see or mutate queue rows"
    );

    // Persisted model suggestions must be denied without the exact currently
    // approved rubric, submission revision and active assigned teacher.
    async fn try_store_suggestion(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        role: &str,
        school_id: Uuid,
        submission_id: Uuid,
        revision: Uuid,
        approval_id: Uuid,
        fingerprint: &str,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(user_id, role, Some(school_id))
            .expect("valid AI suggestion test actor");
        let tx = AuthorizedTx::begin(pool, actor)
            .await
            .expect("begin AI suggestion test transaction");
        let scoped = tx.authorized_pool();
        let criteria = serde_json::json!([
            {"criterion_id": "استدلال", "possible_points": 12.0,
             "awarded_points": 10.0, "evidence": "Reasoning is mostly complete"},
            {"criterion_id": "accuracy", "possible_points": 8.0,
             "awarded_points": 8.0, "evidence": "Correct calculation"}
        ]);
        tx.scope(
            async {
                sqlx::query_scalar::<_, Uuid>(
                    r#"
                    INSERT INTO public.ai_grading_suggestions (
                        submission_id, submission_revision, rubric_approval_id,
                        school_id, rubric_fingerprint, contract_version,
                        grade_scale, suggested_grade, confidence, feedback, criteria
                    ) VALUES ($1,$2,$3,$4,$5,1,20,18,0.85,'Detailed teacher review pending',$6)
                    ON CONFLICT (submission_id, submission_revision, rubric_approval_id) DO NOTHING
                    RETURNING id
                    "#,
                )
                .bind(submission_id)
                .bind(revision)
                .bind(approval_id)
                .bind(school_id)
                .bind(fingerprint)
                .bind(criteria)
                .fetch_optional(&scoped)
                .await
            },
            |result| result.is_ok(),
        )
        .await
        .expect("finish AI suggestion test transaction")
    }

    assert!(
        try_store_suggestion(
            &pool,
            teacher_peer_user,
            "Teacher",
            school_a,
            submissions[0],
            changed.submission_revision,
            rubric_id,
            &stored_fingerprint
        )
        .await
        .is_err(),
        "same-school unassigned teacher cannot persist an AI suggestion"
    );
    assert!(
        try_store_suggestion(
            &pool,
            teacher_b_user,
            "Teacher",
            school_b,
            submissions[0],
            changed.submission_revision,
            rubric_id,
            &stored_fingerprint
        )
        .await
        .is_err(),
        "cross-school AI suggestion persistence must fail"
    );
    assert!(
        try_store_suggestion(
            &pool,
            student_a_user,
            "Student",
            school_a,
            submissions[0],
            changed.submission_revision,
            rubric_id,
            &stored_fingerprint
        )
        .await
        .is_err(),
        "student cannot insert an unreviewed AI suggestion"
    );
    assert!(
        try_store_suggestion(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            Uuid::new_v4(),
            rubric_id,
            &stored_fingerprint
        )
        .await
        .is_err(),
        "stale submission revisions must be rejected at the database boundary"
    );
    assert!(
        try_store_suggestion(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            changed.submission_revision,
            rubric_id,
            &"f".repeat(64)
        )
        .await
        .is_err(),
        "teacher cannot substitute an unapproved rubric fingerprint"
    );
    assert!(
        try_store_suggestion(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            changed.submission_revision,
            Uuid::new_v4(),
            &stored_fingerprint
        )
        .await
        .is_err(),
        "teacher cannot forge an unknown rubric approval"
    );
    let stored_id = try_store_suggestion(
        &pool,
        teacher_a_user,
        "Teacher",
        school_a,
        submissions[0],
        changed.submission_revision,
        rubric_id,
        &stored_fingerprint,
    )
    .await
    .expect("authorized teacher suggestion may be persisted")
    .expect("inserted pending suggestion");

    // Accept and reject each run inside a deliberately rolled-back teacher
    // transaction, so the existing rubric-revocation matrix remains isolated.
    for action in ["accept", "reject"] {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(teacher_a_user, "Teacher", Some(school_a))
            .expect("decision teacher actor");
        let tx = AuthorizedTx::begin(&pool, actor)
            .await
            .expect("begin authorized decision");
        let scoped = tx.authorized_pool();
        let (accepted, official_grade, decision_count): (bool, Option<f64>, i64) = tx
            .scope(
                async {
                    let accepted: bool =
                        sqlx::query_scalar("SELECT public.decide_ai_grading_suggestion($1,$2)")
                            .bind(stored_id)
                            .bind(action)
                            .fetch_one(&scoped)
                            .await?;
                    let grade = sqlx::query_scalar::<_, Option<f64>>(
                        "SELECT grade::float8 FROM public.submissions WHERE id=$1",
                    )
                    .bind(submissions[0])
                    .fetch_one(&scoped)
                    .await?;
                    let count = sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM public.ai_grading_decisions WHERE suggestion_id=$1",
                    )
                    .bind(stored_id)
                    .fetch_one(&scoped)
                    .await?;
                    Ok::<_, sqlx::Error>((accepted, grade, count))
                },
                |_| false,
            )
            .await
            .expect("rollback isolated teacher decision")
            .expect("decision and grade state");
        assert!(accepted);
        assert_eq!(decision_count, 1);
        assert_eq!(
            official_grade,
            if action == "accept" { Some(18.0) } else { None }
        );
    }
    // An unrelated teacher must never be able to decide an artifact.
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(teacher_peer_user, "Teacher", Some(school_a))
            .expect("unassigned reviewer");
        let tx = AuthorizedTx::begin(&pool, actor)
            .await
            .expect("negative review tx");
        let scoped = tx.authorized_pool();
        let unauthorized = tx
            .scope(
                sqlx::query_scalar::<_, bool>(
                    "SELECT public.decide_ai_grading_suggestion($1,'accept')",
                )
                .bind(stored_id)
                .fetch_one(&scoped),
                |_| false,
            )
            .await
            .expect("rollback unauthorized decision");
        assert!(!matches!(unauthorized, Ok(true)));
    }

    // A valid lease can be completed only with the exact revision/rubric
    // evidence; success never publishes or mutates the official grade.
    let completion_worker = Uuid::new_v4();
    sqlx::query(
        "UPDATE public.ai_grading_jobs SET status='running', \
         lease_owner=$2, lease_until=NOW()+INTERVAL '180 seconds' WHERE id=$1",
    )
    .bind(claimed_job)
    .bind(completion_worker)
    .execute(&pool)
    .await
    .expect("reopen fixture lease for completion");
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(&pool, AuthorizedActor::system_queue(completion_worker))
            .await
            .expect("completion proof authorization");
        let scoped = tx.authorized_pool();
        let (wrong_evidence, correct_evidence): (bool, bool) = tx
            .scope(
                async {
                    let wrong: bool =
                        sqlx::query_scalar("SELECT public.complete_ai_grading_job($1,$2,$3)")
                            .bind(completion_worker)
                            .bind(claimed_job)
                            .bind(Uuid::new_v4())
                            .fetch_one(&scoped)
                            .await?;
                    let right: bool =
                        sqlx::query_scalar("SELECT public.complete_ai_grading_job($1,$2,$3)")
                            .bind(completion_worker)
                            .bind(claimed_job)
                            .bind(stored_id)
                            .fetch_one(&scoped)
                            .await?;
                    Ok::<_, sqlx::Error>((wrong, right))
                },
                |result| result.is_ok(),
            )
            .await
            .expect("commit evidence completion")
            .expect("bounded evidence outcomes");
        assert!(!wrong_evidence);
        assert!(correct_evidence);
    }
    let finished: (String, Option<f64>) = {
        let row = sqlx::query(
            "SELECT j.status, s.grade FROM public.ai_grading_jobs j \
             JOIN public.submissions s ON s.id=j.submission_id WHERE j.id=$1",
        )
        .bind(claimed_job)
        .fetch_one(&pool)
        .await
        .expect("completion metadata");
        (row.get("status"), row.get("grade"))
    };
    assert_eq!(finished.0, "succeeded");
    assert_eq!(finished.1, None, "AI suggestions never publish grades");

    // This test also proves that a queued job is cancelled upon a later
    // rubric revocation. Restore only the synthetic fixture's queue state:
    // operationally, a succeeded job is never requeued by the service.
    sqlx::query(
        "UPDATE public.ai_grading_jobs SET status='queued', completed_at=NULL, \
         lease_owner=NULL, lease_until=NULL WHERE id=$1",
    )
    .bind(claimed_job)
    .execute(&pool)
    .await
    .expect("restore independent revocation fixture");

    assert!(
        try_store_suggestion(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            changed.submission_revision,
            rubric_id,
            &stored_fingerprint
        )
        .await
        .expect("idempotent repeat")
        .is_none(),
        "concurrent/repeated generation must not overwrite evidence"
    );
    let stored_status: String =
        sqlx::query_scalar("SELECT status FROM ai_grading_suggestions WHERE id=$1")
            .bind(stored_id)
            .fetch_one(&pool)
            .await
            .expect("persisted suggestion status");
    assert_eq!(stored_status, "pending_review");

    // No review/update/delete permissions can be smuggled through the
    // teacher's bounded request context before an audited review API exists.
    use crate::rls_context::{AuthorizedActor, AuthorizedTx};
    let actor = AuthorizedActor::new(teacher_a_user, "Teacher", Some(school_a))
        .expect("teacher review negative actor");
    let tx = AuthorizedTx::begin(&pool, actor)
        .await
        .expect("begin denied direct review");
    let scoped = tx.authorized_pool();
    let changed_rows = tx
        .scope(
            async {
                sqlx::query(
                    "UPDATE ai_grading_suggestions SET status='rejected', \
                 reviewed_by=$2, reviewed_at=NOW() WHERE id=$1",
                )
                .bind(stored_id)
                .bind(teacher_a)
                .execute(&scoped)
                .await
            },
            |_| false,
        )
        .await
        .expect("finish denied direct review");
    assert!(
        changed_rows.is_err() || changed_rows.expect("update count").rows_affected() == 0,
        "teacher must not bypass controlled review by raw UPDATE"
    );

    assert!(
        attempt_approval(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            custom_id,
            teacher_a,
            serde_json::json!([
                {"criterion_id": "alternative", "possible_points": 20.0}
            ])
        )
        .await
        .expect("active approval blocks any replacement")
        .is_none(),
        "an active approval cannot silently be replaced"
    );

    // A direct mutation of approved evidence is forbidden even for an owner.
    async fn update_approval_as(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        school_id: Uuid,
        approval_id: Uuid,
        change_evidence: bool,
    ) -> Result<u64, sqlx::Error> {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(user_id, "Teacher", Some(school_id))
            .expect("valid approval actor");
        let tx = AuthorizedTx::begin(pool, actor)
            .await
            .expect("begin approval update");
        let scoped = tx.authorized_pool();
        let sql = if change_evidence {
            "UPDATE public.ai_grading_rubric_approvals \
             SET grade_scale = 100, revoked_at = NOW() WHERE id = $1"
        } else {
            "UPDATE public.ai_grading_rubric_approvals \
             SET revoked_at = NOW() WHERE id = $1 AND revoked_at IS NULL"
        };
        tx.scope(
            async {
                sqlx::query(sql)
                    .bind(approval_id)
                    .execute(&scoped)
                    .await
                    .map(|changed| changed.rows_affected())
            },
            |result| result.is_ok(),
        )
        .await
        .expect("finish rubric update transaction")
    }
    assert!(
        update_approval_as(&pool, teacher_a_user, school_a, rubric_id, true)
            .await
            .is_err(),
        "approved rubric evidence cannot be altered during revocation"
    );
    assert_eq!(
        update_approval_as(&pool, teacher_a_user, school_a, rubric_id, false)
            .await
            .expect("teacher may explicitly revoke"),
        1
    );
    let cancelled_after_revoke: String = sqlx::query_scalar(
        "SELECT status FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2 AND rubric_approval_id=$3",
    )
    .bind(submissions[0])
    .bind(changed.submission_revision)
    .bind(rubric_id)
    .fetch_one(&pool)
    .await
    .expect("revoked rubric job still auditable");
    assert_eq!(cancelled_after_revoke, "cancelled");
    assert_eq!(
        update_approval_as(&pool, teacher_a_user, school_a, rubric_id, false)
            .await
            .expect("revocation is terminal"),
        0
    );
    let replacement_approval = attempt_approval(
        &pool,
        teacher_a_user,
        "Teacher",
        school_a,
        custom_id,
        teacher_a,
        criteria,
    )
    .await
    .expect("new approval after revocation")
    .expect("revoked authority requires an explicit replacement approval");
    let replacement_job_status: String = sqlx::query_scalar(
        "SELECT status FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2 AND rubric_approval_id=$3",
    )
    .bind(submissions[0])
    .bind(changed.submission_revision)
    .bind(replacement_approval)
    .fetch_one(&pool)
    .await
    .expect("new approved rubric automatically enqueues new work");
    assert_eq!(replacement_job_status, "queued");
    let replacement_fingerprint: String = sqlx::query_scalar(
        "SELECT rubric_fingerprint FROM ai_grading_rubric_approvals WHERE id=$1",
    )
    .bind(replacement_approval)
    .fetch_one(&pool)
    .await
    .expect("replacement fingerprint");
    let replacement_suggestion = try_store_suggestion(
        &pool,
        teacher_a_user,
        "Teacher",
        school_a,
        submissions[0],
        changed.submission_revision,
        replacement_approval,
        &replacement_fingerprint,
    )
    .await
    .expect("replacement approval can issue its own suggestion")
    .expect("new suggestion exists for unchanged submission revision");
    assert_ne!(
        replacement_suggestion, stored_id,
        "revoked rubric evidence must remain immutable beside replacement evidence"
    );

    // Two independent connections: finalization holds the approved rubric
    // SHARE lock while enqueuing. A simultaneous teacher revocation cannot
    // overtake the uncommitted queue row and leave stale work runnable.
    let mut finalization_tx = pool.begin().await.expect("begin concurrent finalization");
    let concurrent_revision: Uuid = sqlx::query_scalar(
        "UPDATE public.submissions SET revision=gen_random_uuid() \
         WHERE id=$1 RETURNING revision",
    )
    .bind(submissions[0])
    .fetch_one(&mut *finalization_tx)
    .await
    .expect("enqueue in open finalization transaction");
    let new_queued: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2 \
           AND rubric_approval_id=$3 AND status='queued'",
    )
    .bind(submissions[0])
    .bind(concurrent_revision)
    .bind(replacement_approval)
    .fetch_one(&mut *finalization_tx)
    .await
    .expect("new job visible to finalizing transaction");
    assert_eq!(new_queued, 1);

    let other_connection = pool.clone();
    let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel();
    let mut concurrent_revoke = tokio::spawn(async move {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(teacher_a_user, "Teacher", Some(school_a))
            .expect("concurrent teacher actor");
        let tx = AuthorizedTx::begin(&other_connection, actor)
            .await
            .expect("independent revocation transaction");
        let scoped = tx.authorized_pool();
        ready_sender.send(()).expect("signal ready to revoke");
        let result = tx
            .scope(
                sqlx::query(
                    "UPDATE public.ai_grading_rubric_approvals \
                     SET revoked_at=NOW() WHERE id=$1 AND revoked_at IS NULL",
                )
                .bind(replacement_approval)
                .execute(&scoped),
                |result| result.is_ok(),
            )
            .await
            .expect("complete concurrent revocation");
        result
            .expect("authorized teacher may revoke")
            .rows_affected()
    });
    ready_receiver
        .await
        .expect("revocation transaction started");
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(250),
            &mut concurrent_revoke
        )
        .await
        .is_err(),
        "revocation must wait for the uncommitted grading enqueue"
    );
    finalization_tx
        .commit()
        .await
        .expect("commit submitted revision");
    assert_eq!(
        concurrent_revoke
            .await
            .expect("join independent revocation"),
        1,
        "teacher revocation must eventually succeed"
    );
    let cancelled: String = sqlx::query_scalar(
        "SELECT status FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2 AND rubric_approval_id=$3",
    )
    .bind(submissions[0])
    .bind(concurrent_revision)
    .bind(replacement_approval)
    .fetch_one(&pool)
    .await
    .expect("concurrent job remains auditable");
    assert_eq!(
        cancelled, "cancelled",
        "revoked rubric must leave no queued work after both commits"
    );

    // The initial schema requires submissions.submitted_at NOT NULL;
    // custom_assignments.submitted_at is nullable and authoritative here.
    sqlx::query(
        "UPDATE custom_assignments SET submitted_at = NULL \
         WHERE id = (SELECT custom_assignment_id FROM submissions WHERE id = $1)",
    )
    .bind(submissions[0])
    .execute(&pool)
    .await
    .expect("withdraw finalization for negative test");
    assert!(
        source_as(&pool, teacher_a_user, school_a, submissions[0])
            .await
            .is_none(),
        "unfinalized work must never be sent to the grading gateway"
    );

    // Restore finalization and attach a verified submitted PDF alongside
    // nonempty text. Existing text-only inference remains blocked, while
    // the separate native-PDF source may authorize this exact revision.
    sqlx::query(
        "UPDATE custom_assignments SET submitted_at = NOW() \
         WHERE id = (SELECT custom_assignment_id FROM submissions WHERE id = $1)",
    )
    .bind(submissions[0])
    .execute(&pool)
    .await
    .expect("restore submitted assignment");
    assert!(
        source_as(&pool, teacher_a_user, school_a, submissions[0])
            .await
            .is_some(),
        "text-only submission remains eligible before adding an attachment"
    );
    sqlx::query(
        r#"
        INSERT INTO public.submission_attachments (
            id, custom_assignment_id, student_id, school_id, submission_id,
            request_id, filename, media_type, byte_size, sha256,
            status, verified_at, finalized_at
        )
        SELECT $1, s.custom_assignment_id, s.student_id, $2, s.id,
               $3, 'solution.pdf', 'application/pdf', 12, repeat('a', 64),
               'submitted', NOW(), NOW()
        FROM public.submissions s
        WHERE s.id = $4
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(school_a)
    .bind(Uuid::new_v4())
    .bind(submissions[0])
    .execute(&pool)
    .await
    .expect("insert verified finalized attachment fixture");

    assert!(
        source_as(&pool, teacher_a_user, school_a, submissions[0])
            .await
            .is_none(),
        "mixed text and submitted attachment must not be preview-graded"
    );
    assert!(
        source_as(&pool, teacher_b_user, school_b, submissions[1])
            .await
            .is_some(),
        "another school's text-only submission remains eligible"
    );

    let pdf_revision: Uuid =
        sqlx::query_scalar("SELECT revision FROM public.submissions WHERE id=$1")
            .bind(submissions[0])
            .fetch_one(&pool)
            .await
            .expect("submitted PDF revision");
    // The prior approval was explicitly revoked in the concurrency proof.
    // The new source must reject it even when its attachment still exists.
    assert!(
        pdf_source_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            pdf_revision,
            replacement_approval,
        )
        .await
        .is_none(),
        "revoked rubric may never authorize native PDF retrieval"
    );
    let pdf_approval = attempt_approval(
        &pool,
        teacher_a_user,
        "Teacher",
        school_a,
        custom_id,
        teacher_a,
        serde_json::json!([
            {"criterion_id": "استدلال", "possible_points": 12.0},
            {"criterion_id": "accuracy", "possible_points": 8.0}
        ]),
    )
    .await
    .expect("approve current PDF assessment")
    .expect("assigned teacher can explicitly approve a PDF rubric");
    let own_pdf = pdf_source_as(
        &pool,
        teacher_a_user,
        "Teacher",
        school_a,
        submissions[0],
        pdf_revision,
        pdf_approval,
    )
    .await
    .expect("assigned Teacher can access exactly one finalized PDF metadata record");
    assert_eq!(own_pdf.school_id, school_a);
    assert_eq!(own_pdf.submission_revision, pdf_revision);
    assert_eq!(own_pdf.rubric_approval_id, pdf_approval);
    assert_eq!(own_pdf.byte_size, 12);
    assert_eq!(own_pdf.sha256, "a".repeat(64));
    assert_eq!(own_pdf.submission_text, "The answer is 42.");

    // A separate unfinished upload (submission_id NULL) is part of the
    // same student's answer. The native PDF source must never ignore it.
    let pending_sibling = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO public.submission_attachments (
            id, custom_assignment_id, student_id, school_id,
            request_id, filename, media_type, byte_size, sha256, status
        )
        SELECT $1, s.custom_assignment_id, s.student_id, $2, $3,
               'unfinished.png', 'image/png', 15, repeat('b',64), 'pending'
        FROM public.submissions s WHERE s.id=$4
        "#,
    )
    .bind(pending_sibling)
    .bind(school_a)
    .bind(Uuid::new_v4())
    .bind(submissions[0])
    .execute(&pool)
    .await
    .expect("create unfinished sibling upload");
    assert!(
        pdf_source_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            pdf_revision,
            pdf_approval,
        )
        .await
        .is_none(),
        "pending sibling with NULL submission ID must deny native PDF grading"
    );
    sqlx::query("UPDATE public.submission_attachments SET status='removed' WHERE id=$1")
        .bind(pending_sibling)
        .execute(&pool)
        .await
        .expect("remove unfinished sibling");
    assert!(
        pdf_source_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            pdf_revision,
            pdf_approval,
        )
        .await
        .is_some(),
        "removed unfinished upload must not block current eligible PDF"
    );
    for (user, role, school) in [
        (teacher_peer_user, "Teacher", school_a),
        (teacher_b_user, "Teacher", school_b),
        (student_a_user, "Student", school_a),
    ] {
        assert!(
            pdf_source_as(
                &pool,
                user,
                role,
                school,
                submissions[0],
                pdf_revision,
                pdf_approval,
            )
            .await
            .is_none(),
            "wrong actor cannot retrieve native PDF metadata"
        );
    }
    assert!(
        pdf_source_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            Uuid::new_v4(),
            pdf_approval,
        )
        .await
        .is_none(),
        "stale submission revision must be denied"
    );
    assert!(
        pdf_source_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            pdf_revision,
            Uuid::new_v4(),
        )
        .await
        .is_none(),
        "unapproved rubric must be denied"
    );

    // A new finalized revision with a submitted PDF must cancel old queued
    // work without creating a new text-only grading job.
    let before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.ai_grading_jobs WHERE submission_id=$1")
            .bind(submissions[0])
            .fetch_one(&pool)
            .await
            .expect("queue count before resubmission");
    sqlx::query("UPDATE public.submissions SET revision=gen_random_uuid() WHERE id=$1")
        .bind(submissions[0])
        .execute(&pool)
        .await
        .expect("resubmit attached work");
    let after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.ai_grading_jobs WHERE submission_id=$1")
            .bind(submissions[0])
            .fetch_one(&pool)
            .await
            .expect("queue count after attached resubmission");
    assert_eq!(
        before, after,
        "attachments must never queue incomplete AI grading"
    );
    assert!(
        pdf_source_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            pdf_revision,
            pdf_approval,
        )
        .await
        .is_none(),
        "PDF grading source must reject the stale revision after resubmission"
    );
    let status: String = sqlx::query_scalar(
        "SELECT status FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND rubric_approval_id=$2",
    )
    .bind(submissions[0])
    .bind(replacement_approval)
    .fetch_one(&pool)
    .await
    .expect("prior revision cancelled");
    assert_eq!(status, "cancelled");

    // The migration starts with zero PDF-enabled schools. A Teacher-scoped
    // transaction must not opt in a school, even when the database account
    // is used to execute FORCE-RLS migrations.
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(teacher_a_user, "Teacher", Some(school_a))
            .expect("valid teacher opt-in denial actor");
        let tx = AuthorizedTx::begin(&pool, actor)
            .await
            .expect("start denied PDF opt-in transaction");
        let scoped = tx.authorized_pool();
        let attempt = tx
            .scope(
                sqlx::query(
                    "INSERT INTO public.ai_grading_pdf_school_optins (school_id,enabled) \
                     VALUES ($1,TRUE)",
                )
                .bind(school_a)
                .execute(&scoped),
                |_| false,
            )
            .await
            .expect("finish denied opt-in");
        assert!(attempt.is_err(), "Teacher must not opt in their own school");
    }
    let enabled_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ai_grading_pdf_school_optins \
         WHERE school_id=$1 AND enabled",
    )
    .bind(school_a)
    .fetch_one(&pool)
    .await
    .expect("read initially disabled pilot");
    assert_eq!(enabled_count, 0, "migration must not enable PDF inference");
    assert!(
        !pdf_egress_enabled_as(&pool, teacher_a_user, school_a).await,
        "direct teacher PDF inference must be disabled before operator opt-in"
    );

    // Simulate a separately approved migration-owner operator action. Only
    // a subsequent finalized revision may enqueue, never retroactive work.
    {
        // Privileged schema migration transaction: no runtime write RLS
        // policy exists, including for a database-owner-backed app session.
        let mut operator = pool.begin().await.expect("begin operator migration");
        sqlx::query("ALTER TABLE public.ai_grading_pdf_school_optins NO FORCE ROW LEVEL SECURITY")
            .execute(&mut *operator)
            .await
            .expect("operator temporarily removes FORCE inside a transaction");
        sqlx::query(
            "INSERT INTO public.ai_grading_pdf_school_optins (school_id,enabled) \
             VALUES ($1,TRUE)",
        )
        .bind(school_a)
        .execute(&mut *operator)
        .await
        .expect("owner explicitly opts one school in");
        sqlx::query("ALTER TABLE public.ai_grading_pdf_school_optins FORCE ROW LEVEL SECURITY")
            .execute(&mut *operator)
            .await
            .expect("operator restores FORCE before commit");
        operator
            .commit()
            .await
            .expect("commit scoped opt-in migration");
    }
    assert!(
        pdf_egress_enabled_as(&pool, teacher_a_user, school_a).await,
        "explicit operator opt-in must enable teacher-authorized native PDF egress"
    );
    assert!(
        !pdf_egress_enabled_as(&pool, teacher_b_user, school_b).await,
        "school B must remain disabled when school A is enabled"
    );
    // Even when a privileged operator has enabled a school, the runtime
    // Teacher must not change or delete its opt-in via an owner DB connection.
    for statement in [
        "UPDATE public.ai_grading_pdf_school_optins SET enabled=FALSE WHERE school_id=$1",
        "DELETE FROM public.ai_grading_pdf_school_optins WHERE school_id=$1",
    ] {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(teacher_a_user, "Teacher", Some(school_a))
            .expect("Teacher opt-in mutation denied actor");
        let tx = AuthorizedTx::begin(&pool, actor)
            .await
            .expect("begin denied PDF opt-in update");
        let scoped = tx.authorized_pool();
        let denied = tx
            .scope(
                sqlx::query(statement).bind(school_a).execute(&scoped),
                |_| false,
            )
            .await
            .expect("complete denied opt-in update");
        assert!(denied.is_err(), "runtime Teacher bypassed PDF opt-in guard");
    }
    // A pending sibling has NULL submission_id; the queue must count it
    // against the same custom assignment before deciding a PDF is complete.
    let queue_pending_sibling = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO public.submission_attachments (
            id, custom_assignment_id, student_id, school_id,
            request_id, filename, media_type, byte_size, sha256, status
        )
        SELECT $1, s.custom_assignment_id, s.student_id, $2, $3,
               'not-finished.png', 'image/png', 20, repeat('c',64), 'pending'
        FROM public.submissions s WHERE s.id=$4
        "#,
    )
    .bind(queue_pending_sibling)
    .bind(school_a)
    .bind(Uuid::new_v4())
    .bind(submissions[0])
    .execute(&pool)
    .await
    .expect("create pending sibling before queue");
    let incomplete_revision: Uuid = sqlx::query_scalar(
        "UPDATE public.submissions SET revision=gen_random_uuid() \
         WHERE id=$1 RETURNING revision",
    )
    .bind(submissions[0])
    .fetch_one(&pool)
    .await
    .expect("finalize incomplete PDF revision");
    let incomplete_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2",
    )
    .bind(submissions[0])
    .bind(incomplete_revision)
    .fetch_one(&pool)
    .await
    .expect("check incomplete PDF queue denial");
    assert_eq!(
        incomplete_count, 0,
        "opted-in school must never enqueue a PDF while a sibling upload is pending"
    );
    sqlx::query("UPDATE public.submission_attachments SET status='removed' WHERE id=$1")
        .bind(queue_pending_sibling)
        .execute(&pool)
        .await
        .expect("remove unfinished sibling before fresh finalization");

    let queued_revision: Uuid = sqlx::query_scalar(
        "UPDATE public.submissions SET revision=gen_random_uuid() \
         WHERE id=$1 RETURNING revision",
    )
    .bind(submissions[0])
    .fetch_one(&pool)
    .await
    .expect("finalize opt-in PDF revision");
    let queued_status: String = sqlx::query_scalar(
        "SELECT status FROM public.ai_grading_jobs \
         WHERE submission_id=$1 AND submission_revision=$2 \
           AND rubric_approval_id=$3",
    )
    .bind(submissions[0])
    .bind(queued_revision)
    .bind(pdf_approval)
    .fetch_one(&pool)
    .await
    .expect("approved single PDF enqueued at exact revision");
    assert_eq!(queued_status, "queued");

    // A school opt-in by itself does NOT grant permission to incur charges:
    // the durable, operator-configured budget starts empty and denies all.
    assert!(
        !pdf_budget_reserve_as(
            &pool, teacher_a_user, "Teacher", school_a,
            submissions[0], queued_revision, pdf_approval, 12,
        )
        .await,
        "unconfigured PDF budget must block model egress"
    );
    // Budget policies can be configured only by an unscoped schema operator.
    sqlx::query(
        "INSERT INTO edutalent_internal.ai_grading_pdf_budget_policy
           (school_id,max_requests_per_utc_day,max_input_bytes_per_utc_day,
            reserved_microusd_per_request,max_reserved_microusd_per_utc_day)
         VALUES ($1,1,120,250000,250000)",
    )
    .bind(school_a)
    .execute(&pool)
    .await
    .expect("operator sets a one-attempt PDF pilot budget");
    {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let tx = AuthorizedTx::begin(
            &pool,
            AuthorizedActor::new(teacher_a_user, "Teacher", Some(school_a))
                .expect("Teacher budget policy guard actor"),
        )
        .await
        .expect("begin denied Teacher budget policy mutation");
        let scoped = tx.authorized_pool();
        let denied = tx
            .scope(
                sqlx::query(
                    "UPDATE edutalent_internal.ai_grading_pdf_budget_policy
                     SET max_requests_per_utc_day=1000 WHERE school_id=$1",
                )
                .bind(school_a)
                .execute(&scoped),
                |_| false,
            )
            .await
            .expect("finish denied runtime budget policy write");
        assert!(denied.is_err(), "Teacher must not raise an operator budget");
    }
    for (actor, role, school, revision, rubric) in [
        (teacher_peer_user, "Teacher", school_a, queued_revision, pdf_approval),
        (teacher_b_user, "Teacher", school_b, queued_revision, pdf_approval),
        (student_a_user, "Student", school_a, queued_revision, pdf_approval),
        (teacher_a_user, "Teacher", school_a, Uuid::new_v4(), pdf_approval),
        (teacher_a_user, "Teacher", school_a, queued_revision, Uuid::new_v4()),
    ] {
        assert!(
            !pdf_budget_reserve_as(
                &pool, actor, role, school,
                submissions[0], revision, rubric, 12,
            )
            .await,
            "unauthorized or stale budget reservation must be denied"
        );
    }

    // The production Rust helper commits an independent Teacher-RLS budget
    // reservation before outbound model I/O. It must remain charged even when
    // the caller's longer model/suggestion transaction later fails.
    crate::services::ai_grading_pdf_budget::reserve_pdf_budget_before_egress(
        &pool, teacher_a_user, school_a,
        submissions[0], queued_revision, pdf_approval, 12,
    )
    .await
    .expect("first authorized school PDF budget reservation");
    assert_eq!(
        crate::services::ai_grading_pdf_budget::reserve_pdf_budget_before_egress(
            &pool, teacher_a_user, school_a,
            submissions[0], queued_revision, pdf_approval, 12,
        )
        .await,
        Err(crate::services::ai_grading_pdf_budget::PdfBudgetError::Denied),
        "second inference attempt must be denied by hard daily cap"
    );
    let usage: (i32, i64, i64) = sqlx::query_as(
        "SELECT request_count, input_bytes, reserved_microusd
         FROM edutalent_internal.ai_grading_pdf_budget_usage
         WHERE school_id=$1 AND utc_day=(NOW() AT TIME ZONE 'UTC')::date",
    )
    .bind(school_a)
    .fetch_one(&pool)
    .await
    .expect("durably committed school PDF budget");
    assert_eq!(usage, (1, 12, 250000));
    let school_b_budget_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM edutalent_internal.ai_grading_pdf_budget_usage
         WHERE school_id=$1",
    )
    .bind(school_b)
    .fetch_one(&pool)
    .await
    .expect("other school cannot spend A budget");
    assert_eq!(school_b_budget_count, 0);

    let school_b_optin: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.ai_grading_pdf_school_optins \
         WHERE school_id=$1",
    )
    .bind(school_b)
    .fetch_one(&pool)
    .await
    .expect("other school remains off");
    assert_eq!(school_b_optin, 0);

    // Return the fixture to a disabled and non-running state so subsequent
    // manual-grade acceptance assertions remain isolated.
    {
        let mut operator = pool.begin().await.expect("begin operator rollback");
        sqlx::query("ALTER TABLE public.ai_grading_pdf_school_optins NO FORCE ROW LEVEL SECURITY")
            .execute(&mut *operator)
            .await
            .expect("operator temporary migration-only opt-in change");
        sqlx::query(
            "UPDATE public.ai_grading_pdf_school_optins SET enabled=FALSE,updated_at=NOW() \
             WHERE school_id=$1",
        )
        .bind(school_a)
        .execute(&mut *operator)
        .await
        .expect("operator disables pilot");
        sqlx::query("ALTER TABLE public.ai_grading_pdf_school_optins FORCE ROW LEVEL SECURITY")
            .execute(&mut *operator)
            .await
            .expect("operator restores FORCE on rollback");
        operator
            .commit()
            .await
            .expect("commit scoped opt-in rollback");
    }
    assert!(
        !pdf_egress_enabled_as(&pool, teacher_a_user, school_a).await,
        "operator revocation must disable fresh PDF egress"
    );
    sqlx::query(
        "UPDATE public.ai_grading_jobs SET status='cancelled',completed_at=NOW(), \
         last_error_code='authorization_revoked' \
         WHERE submission_id=$1 AND submission_revision=$2 AND status='queued'",
    )
    .bind(submissions[0])
    .bind(queued_revision)
    .execute(&pool)
    .await
    .expect("retire pilot job");

    // Exact-object teacher authorization and two-row manual-grade lifecycle
    // proof against real PostgreSQL/RLS, after AI-preview assertions are done.
    async fn manual_grade_as(
        raw: &sqlx::PgPool,
        actor_id: Uuid,
        actor_role: &str,
        actor_school: Uuid,
        submission_id: Uuid,
        points: f64,
    ) -> Result<bool, dioxus::prelude::ServerFnError> {
        use crate::rls_context::{AuthorizedActor, AuthorizedTx};
        let actor = AuthorizedActor::new(actor_id, actor_role, Some(actor_school))
            .expect("valid teacher-grade test actor");
        let tx = AuthorizedTx::begin(raw, actor)
            .await
            .expect("begin manual-grade authorization test");
        let scoped = tx.authorized_pool();
        tx.scope(
            crate::server_functions::dashboard_functions::save_authorized_manual_grade(
                &scoped,
                actor_id,
                submission_id,
                points,
                Some("Teacher-reviewed".to_string()),
            ),
            Result::is_ok,
        )
        .await
        .expect("complete grading test transaction")
    }

    for (actor, role, school, target) in [
        (teacher_peer_user, "Teacher", school_a, submissions[0]),
        (teacher_b_user, "Teacher", school_b, submissions[0]),
        (teacher_a_user, "Teacher", school_a, submissions[1]),
        (student_a_user, "Student", school_a, submissions[0]),
    ] {
        assert!(
            manual_grade_as(&pool, actor, role, school, target, 90.0)
                .await
                .is_err(),
            "unassigned, cross-school, or student grading must fail closed"
        );
    }
    let before_manual: (Option<f64>, Option<Uuid>) = sqlx::query_as(
        "SELECT grade::double precision, graded_by FROM public.submissions WHERE id=$1",
    )
    .bind(submissions[0])
    .fetch_one(&pool)
    .await
    .expect("read unchanged grade after denied cases");
    assert_eq!(before_manual, (None, None));

    assert!(
        manual_grade_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            90.0
        )
        .await
        .expect("assigned teacher may grade"),
        "assigned teacher must commit a valid grade"
    );
    let after_manual: (Option<f64>, i16, Option<Uuid>, String) = sqlx::query_as(
        "SELECT s.grade::double precision, s.grade_scale, s.graded_by, ca.status::text \
         FROM public.submissions s JOIN public.custom_assignments ca \
           ON ca.id=s.custom_assignment_id WHERE s.id=$1",
    )
    .bind(submissions[0])
    .fetch_one(&pool)
    .await
    .expect("read committed manual grade and lifecycle");
    assert_eq!(
        after_manual,
        (Some(90.0), 100, Some(teacher_a), "Graded".to_string())
    );
    assert!(
        manual_grade_as(
            &pool,
            teacher_a_user,
            "Teacher",
            school_a,
            submissions[0],
            f64::NAN
        )
        .await
        .is_err(),
        "nonfinite scores must fail closed without mutation"
    );
    let unchanged_grade: Option<f64> =
        sqlx::query_scalar("SELECT grade::double precision FROM public.submissions WHERE id=$1")
            .bind(submissions[0])
            .fetch_one(&pool)
            .await
            .expect("unchanged official grade");
    assert_eq!(unchanged_grade, Some(90.0));
}
