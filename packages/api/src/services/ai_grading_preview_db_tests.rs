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
    let mut finalization_tx = pool
        .begin()
        .await
        .expect("begin concurrent finalization");
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
        result.expect("authorized teacher may revoke")
            .rows_affected()
    });
    ready_receiver.await.expect("revocation transaction started");
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(250),
            &mut concurrent_revoke
        )
        .await
        .is_err(),
        "revocation must wait for the uncommitted grading enqueue"
    );
    finalization_tx.commit().await.expect("commit submitted revision");
    assert_eq!(
        concurrent_revoke.await.expect("join independent revocation"),
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

    // Restore finalization and attach a verified submitted PDF *alongside*
    // nonempty text. Only the complete answer may be graded: without OCR,
    // this mixed submission must fail closed before calling the LLM.
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
}
