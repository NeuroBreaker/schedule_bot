use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::error::Error;

#[derive(Debug)]
struct Faculty {
    name: String,
    url: String,
}

impl Faculty {
    fn new(name: String, url: String) -> Faculty {
        Faculty { name, url }
    }
}

pub async fn init_db(db_url: &str) -> Result<PgPool, Box<dyn Error + Send + Sync>> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS faculties (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            url TEXT NOT NULL UNIQUE
        );
        "#
    )
    .execute(&pool)
    .await?;

    get_facults(&pool).await?;

    log::info!("Проверка/создание таблицы завершено.");
    Ok(pool)
}

pub async fn get_facults(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = "https://ssau.ru/rasp";
    let base_url = "https://ssau.ru";

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    log::info!("Запрашиваю данные с {}...", url);

    let response = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&response);
    let selector = Selector::parse(".faculties__item a").unwrap();

    let mut count = 0;

    for element in document.select(&selector) {
        let name = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        let link = element.value().attr("href").unwrap_or("").to_string();
        let full_url = format!("{}{}", base_url, link);

        if !name.is_empty() {
            sqlx::query("INSERT INTO faculties (name, url) VALUES ($1, $2) ON CONFLICT (url) DO NOTHING")
                .bind(&name)
                .bind(&full_url)
                .execute(pool)
                .await?;

            count += 1;
        }
    }

    log::info!("Успешно обработано и сохранено в БД {} записей", count);

    Ok(())
}

async fn get_course(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
    Ok(())
}
