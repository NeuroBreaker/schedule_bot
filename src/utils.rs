use sqlx::{PgPool, Row};

pub async fn get_user_url(pool: &PgPool, user_id: i64) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query(
        r#"
            SELECT f.url
            FROM faculties f
            JOIN users u ON f.id = u.faculty_id
            WHERE u.id = $1
            "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.get("url")))
}
