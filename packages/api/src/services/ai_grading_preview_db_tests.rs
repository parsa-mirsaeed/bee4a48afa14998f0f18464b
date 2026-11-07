//! Database-backed allowed/denied matrix for teacher AI preview authorization.
//! Fixtures deliberately live outside server_functions: production server
//! functions must never contain or accept an unscoped PgPool.

use super::{find_authorized_grading_source, AuthorizedGradingSource};
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
}
