use sqlx::{PgPool, postgres::PgPoolOptions};
use std::error::Error;

pub mod get_faculties;

pub use get_faculties::*;

#[derive(Default, Clone, Debug)]
struct Faculty {
    name: String,
    course: String,
    group: String,
    url: String,
}

pub async fn init_db(db_url: &str) -> Result<PgPool, Box<dyn Error + Send + Sync>> {
    log::info!("Начало инициализации бд");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(db_url)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS faculties (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            course TEXT NOT NULL,
            "group" TEXT NOT NULL,
            url TEXT NOT NULL UNIQUE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id BIGINT PRIMARY KEY,
            faculty_id INTEGER REFERENCES faculties(id)
        );
        "#,
    )
    .execute(&pool)
    .await?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM faculties")
        .fetch_one(&pool)
        .await?;

    if count.0 == 0 {
        log::info!("База данных пуста, запускаю парсинг");
        get_faculties(&pool).await?;
    } else {
        log::info!("Данные в базе уже есть, пропуск парсинга");
    }

    log::info!("Инициализация завершена успешно");
    Ok(pool)
}
