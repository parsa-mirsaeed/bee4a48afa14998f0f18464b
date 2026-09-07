//! Execute authorization and immutable-original invariants with a real
//! NOBYPASSRLS database role. All synthetic state, including the role, rolls back.
use sqlx::{postgres::PgPoolOptions, Executor};
use uuid::Uuid;

#[tokio::test]
async fn private_originals_enforce_actor_school_state_and_cleanup_boundaries() {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(
            &std::env::var("DATABASE_URL").expect("attachment security proof requires PostgreSQL"),
        )
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let school = Uuid::new_v4();
    let other_school = Uuid::new_v4();
    let student = Uuid::new_v4();
    let student_user = Uuid::new_v4();
    let teacher = Uuid::new_v4();
    let teacher_user = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let outsider_teacher = Uuid::new_v4();
    let class = Uuid::new_v4();
    let subject = Uuid::new_v4();
    let assignment = Uuid::new_v4();
    let custom = Uuid::new_v4();
    let attachment = Uuid::new_v4();
    let submission = Uuid::new_v4();
    let role = format!("attachment_test_{}", Uuid::new_v4().simple());
    tx.execute(format!(r#"
        CREATE ROLE {role} NOLOGIN NOBYPASSRLS;
        GRANT USAGE ON SCHEMA public,edutalent_internal TO {role};
        GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA public TO {role};
        GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO {role};
        GRANT EXECUTE ON FUNCTION edutalent_internal.lock_student_submission_assignment(uuid) TO {role};
        GRANT EXECUTE ON FUNCTION edutalent_internal.claim_submission_attachment_cleanup() TO {role};
        INSERT INTO schools(id,name) VALUES('{school}','Attachment A'),('{other_school}','Attachment B');
        INSERT INTO users(id,name,email,school_id,role_id,is_active) VALUES
          ('{student_user}','Student','{student_user}@example.test','{school}',(SELECT id FROM roles WHERE name='Student'),true),
          ('{teacher_user}','Teacher','{teacher_user}@example.test','{school}',(SELECT id FROM roles WHERE name='Teacher'),true),
          ('{outsider}','Other teacher','{outsider}@example.test','{school}',(SELECT id FROM roles WHERE name='Teacher'),true);
        INSERT INTO students(id,user_id,school_id) VALUES('{student}','{student_user}','{school}');
        INSERT INTO teachers(id,user_id,school_id) VALUES('{teacher}','{teacher_user}','{school}'),('{outsider_teacher}','{outsider}','{school}');
        INSERT INTO subjects(id,code,name) VALUES('{subject}','{subject}','Attachment subject');
        INSERT INTO class_sections(id,school_id,subject_id,name,term) VALUES('{class}','{school}','{subject}','Attachment class','Test');
        INSERT INTO teaching_assignments(class_section_id,teacher_id) VALUES('{class}','{teacher}');
        INSERT INTO enrollments(class_section_id,student_id) VALUES('{class}','{student}');
        INSERT INTO assignments(id,teacher_id,class_section_id,subject_id,title,body,status,due_at) VALUES('{assignment}','{teacher}','{class}','{subject}','Attachment work','Work','Published',NOW()+INTERVAL '1 day');
        INSERT INTO custom_assignments(id,assignment_id,student_id,status,due_at) VALUES('{custom}','{assignment}','{student}','Assigned',NOW()+INTERVAL '1 day');
        SET LOCAL ROLE {role};
        SET LOCAL app.user_id='{student_user}'; SET LOCAL app.user_role='Student'; SET LOCAL app.school_id='{school}';
        INSERT INTO submission_attachments(id,custom_assignment_id,student_id,school_id,request_id,filename,media_type,byte_size,sha256)
        VALUES('{attachment}','{custom}','{student}','{school}','{attachment}','work.pdf','application/pdf',100,repeat('a',64));
    "#).as_str()).await.unwrap();
    let count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM submission_attachments")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM edutalent_internal.lock_student_submission_assignment($1)",
        )
        .bind(custom)
        .fetch_one(&mut *tx)
        .await
        .unwrap(),
        1,
        "own published assignment must be lockable without widening row mutation rights"
    );
    // The Student UPDATE policy exists only so finalize can lock and mark the
    // exact row. The guard must reject arbitrary assignment-field mutation.
    tx.execute("SAVEPOINT student_transition_guard;")
        .await
        .unwrap();
    assert!(sqlx::query(
        "UPDATE custom_assignments SET due_at=due_at+INTERVAL '1 day' WHERE id=$1"
    )
    .bind(custom)
    .execute(&mut *tx)
    .await
    .is_err());
    tx.execute("ROLLBACK TO student_transition_guard;")
        .await
        .unwrap();

    // Same school unrelated actor, cross-school actor, Parent, manager, admin:
    // none may see a pending original, acquire the Student lock, or smuggle a
    // tenant-reassigned write.
    for (actor, kind, scope) in [
        (outsider, "Student", school),
        (student_user, "Student", other_school),
        (teacher_user, "Teacher", school),
        (student_user, "Parent", school),
        (student_user, "SchoolManager", school),
        (student_user, "PlatformAdmin", school),
    ] {
        tx.execute(format!("SET LOCAL app.user_id='{actor}';SET LOCAL app.user_role='{kind}';SET LOCAL app.school_id='{scope}';").as_str()).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM submission_attachments")
                .fetch_one(&mut *tx)
                .await
                .unwrap(),
            0,
            "{kind}/{scope} must not see pending originals"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM edutalent_internal.lock_student_submission_assignment($1)",
            )
            .bind(custom)
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
            0,
            "{kind}/{scope} must not acquire the Student submission lock"
        );
        assert_eq!(
            sqlx::query("UPDATE submission_attachments SET status='removed'")
                .execute(&mut *tx)
                .await
                .unwrap()
                .rows_affected(),
            0
        );
    }
    tx.execute(format!("SET LOCAL app.user_id='{student_user}';SET LOCAL app.user_role='Student';SET LOCAL app.school_id='{school}';SAVEPOINT rejected;").as_str()).await.unwrap();
    assert!(
        sqlx::query("UPDATE submission_attachments SET school_id=$1")
            .bind(other_school)
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.execute("ROLLBACK TO rejected;").await.unwrap();
    assert!(
        sqlx::query("SELECT * FROM edutalent_internal.claim_submission_attachment_cleanup()")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.execute("ROLLBACK TO rejected;").await.unwrap();
    // Finalization is atomic: lock the same Student-owned row, attach the
    // verified original and submitted row, then prove the constrained status
    // transition succeeds while Teacher read and terminal-original guards hold.
    tx.execute(format!(r#"
        SELECT 1 FROM custom_assignments WHERE id='{custom}' FOR UPDATE;
        INSERT INTO submissions(id,custom_assignment_id,student_id,content,submitted_at) VALUES('{submission}','{custom}','{student}','{{"text":"work"}}',NOW());
        UPDATE submission_attachments SET status='ready',verified_at=NOW() WHERE id='{attachment}';
        UPDATE submission_attachments SET status='submitted',submission_id='{submission}',finalized_at=NOW() WHERE id='{attachment}';
        UPDATE custom_assignments SET status='Submitted',submitted_at=NOW() WHERE id='{custom}';
        SET LOCAL app.user_id='{teacher_user}'; SET LOCAL app.user_role='Teacher';
    "#).as_str()).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM submission_attachments")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        1
    );
    tx.execute(format!("SET LOCAL app.user_id='{outsider}';").as_str())
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM submission_attachments")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        0
    );
    tx.execute(format!("SET LOCAL app.user_id='{student_user}';SET LOCAL app.user_role='Student';SAVEPOINT immutable;").as_str()).await.unwrap();
    assert!(
        sqlx::query("UPDATE submission_attachments SET status='removed',submission_id=NULL")
            .execute(&mut *tx)
            .await
            .is_err()
    );
    tx.execute("ROLLBACK TO immutable;").await.unwrap();
    assert_eq!(
        sqlx::query("DELETE FROM submission_attachments")
            .execute(&mut *tx)
            .await
            .unwrap()
            .rows_affected(),
        0
    );
    // A queue caller receives no submitted originals, even under the bounded
    // elevated scheduler context. No-school ordinary actor is denied above.
    tx.execute(format!("SET LOCAL app.user_id='{outsider}';SET LOCAL app.user_role='system_job';SET LOCAL app.school_id='';SET LOCAL app.elevated_operation='true';").as_str()).await.unwrap();
    assert!(
        sqlx::query("SELECT * FROM edutalent_internal.claim_submission_attachment_cleanup()")
            .fetch_optional(&mut *tx)
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
}
