use reqwest::Client;
use scraper::{Html, Selector};
use std::error::Error;
use sqlx::{PgPool, Row};

pub async fn get_user_url(pool: &PgPool, user_id: i64) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query(
            r#"
            SELECT f.url
            FROM faculties f
            JOIN users u ON f.id = u.faculty_id
            WHERE u.id = $1
            "#
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get("url")))
}

pub async fn week(user_id: i64, pool: &PgPool) -> Result<String, Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    let mut result = if let Some(url) = get_user_url(pool, user_id).await? {
        //let response = client.get(url).send().await?.text().await?;
        //let document = Html::parse_document(&response);
        //let selector = Selector::parse("").unwrap();

        //for element in document.select(&selector) {
        //    let 
        //}
        //schedule

        "Какое-то расписание".to_string()
    } else {
        "Вас нету в базе данных бота\nВведите /setup для выбора факультета".to_string()
    };
    
    Ok(result)
}
