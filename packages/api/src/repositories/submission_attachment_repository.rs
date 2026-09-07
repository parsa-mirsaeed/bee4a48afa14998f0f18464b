//! Attachment intents are committed before Storage writes. Every object has a
//! durable owner even when a request or application process dies mid-upload.
use crate::domain::UserInfo;
use crate::rls_context::AuthorizedPool;
use crate::server_functions::submission_attachment_functions::*;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum AttachmentError {
    #[error("attachment access denied")]
    Forbidden,
    #[error("invalid attachment")]
    Invalid,
    #[error("attachment state changed")]
    Conflict,
    #[error("attachment limits exceeded")]
    Limit,
    #[error("storage unavailable")]
    Storage,
    #[error("attachment database operation: {0}")]
    Database(#[from] sqlx::Error),
}
impl AttachmentError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Forbidden => "attachment_forbidden",
            Self::Invalid => "attachment_invalid",
            Self::Conflict => "attachment_conflict",
            Self::Limit => "attachment_limit",
            Self::Storage | Self::Database(_) => "attachment_unavailable",
        }
    }
}
pub type Result<T> = std::result::Result<T, AttachmentError>;

// Student writes must serialize against submission/grading without granting the
// Student UPDATE rights on custom_assignments. The bounded database entry point
// validates the transaction-local actor/school context and locks only the exact
// published assignment as the migration owner. Read-only authorization remains
// ordinary RLS-filtered SQL.
pub async fn authorize_student(
    pool: &AuthorizedPool,
    user: &UserInfo,
    assignment: Uuid,
    writable: bool,
) -> Result<(Uuid, Uuid)> {
    if user.role != "Student" {
        return Err(AttachmentError::Forbidden);
    }
    let actor = Uuid::parse_str(&user.id).map_err(|_| AttachmentError::Forbidden)?;
    let row = if writable {
        sqlx::query(
            "SELECT student_id, school_id, graded_at FROM edutalent_internal.lock_student_submission_assignment($1)",
        )
        .bind(assignment)
        .fetch_optional(pool)
        .await?
        .ok_or(AttachmentError::Forbidden)?
    } else {
        sqlx::query(
            r#"
            SELECT ca.student_id, cs.school_id, ca.graded_at
            FROM custom_assignments ca
            JOIN assignments a ON a.id=ca.assignment_id
            JOIN class_sections cs ON cs.id=a.class_section_id
            JOIN students s ON s.id=ca.student_id
            JOIN users u ON u.id=s.user_id
            JOIN roles r ON r.id=u.role_id
            JOIN enrollments e ON e.student_id=s.id AND e.class_section_id=cs.id
            WHERE ca.id=$1 AND u.id=$2 AND u.is_active AND r.name::text='Student'
              AND s.school_id=u.school_id AND cs.school_id=u.school_id
              AND a.status='Published'::assignment_status
            "#,
        )
        .bind(assignment)
        .bind(actor)
        .fetch_optional(pool)
        .await?
        .ok_or(AttachmentError::Forbidden)?
    };
    if writable
        && row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("graded_at")
            .is_some()
    {
        return Err(AttachmentError::Conflict);
    }
    Ok((row.get("student_id"), row.get("school_id")))
}

pub fn validate_metadata(input: &AttachmentIntent) -> Result<String> {
    if input.byte_size <= 0 || input.byte_size > MAX_ATTACHMENT_BYTES as i64 {
        return Err(AttachmentError::Limit);
    }
    if input.sha256.len() != 64
        || !input
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(AttachmentError::Invalid);
    }
    // Preserve a recognizable basename as data. Never use this in a storage key,
    // URL, Content-Disposition header, or HTML interpolation.
    let name: String = input
        .filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|c| {
            !c.is_control() && !matches!(*c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
        .collect();
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 255 {
        return Err(AttachmentError::Invalid);
    }
    let extension = name
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(
        (input.media_type.as_str(), extension.as_str()),
        ("application/pdf", "pdf") | ("image/jpeg", "jpg" | "jpeg") | ("image/png", "png")
    ) {
        return Err(AttachmentError::Invalid);
    }
    Ok(name)
}

pub fn from_row(row: &PgRow) -> Result<SubmissionAttachment> {
    Ok(SubmissionAttachment {
        id: row.try_get("id")?,
        filename: row.try_get("filename")?,
        media_type: row.try_get("media_type")?,
        byte_size: row.try_get("byte_size")?,
        sha256: row.try_get("sha256")?,
        status: match row.try_get::<String, _>("status")?.as_str() {
            "pending" => AttachmentStatus::Pending,
            "ready" => AttachmentStatus::Ready,
            "submitted" => AttachmentStatus::Submitted,
            _ => return Err(AttachmentError::Conflict),
        },
    })
}

pub async fn reserve(
    pool: &AuthorizedPool,
    user: &UserInfo,
    input: AttachmentIntent,
) -> Result<SubmissionAttachment> {
    let filename = validate_metadata(&input)?;
    let (student, school) = authorize_student(pool, user, input.assignment_id, true).await?;
    if let Some(row) = sqlx::query("SELECT * FROM submission_attachments WHERE custom_assignment_id=$1 AND request_id=$2 FOR UPDATE")
        .bind(input.assignment_id).bind(input.request_id).fetch_optional(pool).await? {
        let existing=from_row(&row)?;
        if existing.filename!=filename || existing.media_type!=input.media_type || existing.byte_size!=input.byte_size || existing.sha256!=input.sha256 { return Err(AttachmentError::Conflict); }
        return Ok(existing);
    }
    let counts=sqlx::query("SELECT count(*) AS count, COALESCE(sum(byte_size),0)::bigint AS bytes FROM submission_attachments WHERE custom_assignment_id=$1 AND status<>'removed'")
        .bind(input.assignment_id).fetch_one(pool).await?;
    if counts.get::<i64, _>("count") >= MAX_SUBMISSION_FILES as i64
        || counts.get::<i64, _>("bytes") + input.byte_size > MAX_SUBMISSION_FILE_BYTES as i64
    {
        return Err(AttachmentError::Limit);
    }
    let row=sqlx::query(r#"INSERT INTO submission_attachments
        (custom_assignment_id, student_id, school_id, request_id, filename, media_type, byte_size, sha256)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"#)
        .bind(input.assignment_id).bind(student).bind(school).bind(input.request_id).bind(filename)
        .bind(input.media_type).bind(input.byte_size).bind(input.sha256).fetch_one(pool).await?;
    from_row(&row)
}

pub async fn list(
    pool: &AuthorizedPool,
    user: &UserInfo,
    assignment: Uuid,
) -> Result<Vec<SubmissionAttachment>> {
    if !matches!(user.role.as_str(), "Student" | "Teacher") {
        return Err(AttachmentError::Forbidden);
    }
    // RLS applies exact Student ownership or current Teacher/class/school scope.
    sqlx::query("SELECT * FROM submission_attachments WHERE custom_assignment_id=$1 AND status<>'removed' ORDER BY created_at,id")
        .bind(assignment).fetch_all(pool).await?.iter().map(from_row).collect()
}

pub async fn remove(pool: &AuthorizedPool, user: &UserInfo, id: Uuid) -> Result<()> {
    let assignment = sqlx::query_scalar::<_, Uuid>(
        "SELECT custom_assignment_id FROM submission_attachments WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AttachmentError::Forbidden)?;
    authorize_student(pool, user, assignment, true).await?;
    let changed=sqlx::query("UPDATE submission_attachments SET status='removed', cleanup_after=NOW() WHERE id=$1 AND status IN ('pending','ready','removed')")
        .bind(id).execute(pool).await?.rows_affected();
    if changed != 1 {
        return Err(AttachmentError::Conflict);
    }
    Ok(())
}

pub fn object_key(school: Uuid, attachment: Uuid) -> String {
    format!("{school}/{attachment}")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> AttachmentIntent {
        AttachmentIntent {
            assignment_id: Uuid::nil(),
            request_id: Uuid::nil(),
            filename: "homework.pdf".into(),
            media_type: "application/pdf".into(),
            byte_size: 42,
            sha256: "a".repeat(64),
        }
    }
    #[test]
    fn validates_filename_type_size_and_hash_without_trusting_a_path() {
        let mut i = input();
        assert_eq!(validate_metadata(&i).unwrap(), "homework.pdf");
        i.filename = "../../private/\u{202e}homework.pdf".into();
        assert_eq!(validate_metadata(&i).unwrap(), "homework.pdf");
        i.media_type = "image/png".into();
        assert!(validate_metadata(&i).is_err());
        i = input();
        i.byte_size = 0;
        assert!(validate_metadata(&i).is_err());
        i.byte_size = MAX_ATTACHMENT_BYTES as i64 + 1;
        assert!(validate_metadata(&i).is_err());
        i = input();
        i.sha256 = "x".repeat(64);
        assert!(validate_metadata(&i).is_err());
        assert!(!object_key(Uuid::nil(), Uuid::nil()).contains("homework"));
    }
}
