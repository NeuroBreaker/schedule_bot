use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::error::Error;

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

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS schedules (
                week INT
                schedule JSONB
                hash BIGINT
                faculty_id INTEGER REFERENCES faculties(id)
            )
        "#,
    )
    .execute(&pool)
    .await?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM faculties")
        .fetch_one(&pool)
        .await?;

    if count.0 == 0 {
        log::info!("База данных пуста, запускаю парсинг");
        push_faculties(&pool).await?;
    } else {
        log::info!("Данные в базе уже есть, пропуск парсинга");
    }

    log::info!("Инициализация завершена успешно");
    Ok(pool)
}

pub async fn push_faculties(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
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
            .collect::<String>()
            .trim()
            .to_string();

        let link = element.value().attr("href").unwrap_or("").to_string();
        let full_url = format!("{}{}", base_url, link);

        if !name.is_empty() {
            let faculty = Faculty {
                name: name.clone(),
                ..Default::default()
            };

            push_course(&full_url, faculty, &mut count, &client, pool).await?;
        }
    }

    log::info!("В БД добавлено {} записей", count);

    Ok(())
}

async fn push_course(
    url: &str,
    mut faculty: Faculty,
    count: &mut i32,
    client: &Client,
    pool: &PgPool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let base_url = "https://ssau.ru";

    let response = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&response);
    let selector = Selector::parse(".nav-course__item a").unwrap();

    for element in document.select(&selector) {
        let course = element
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        let link = element.value().attr("href").unwrap_or("").to_string();
        let url = format!("{}{}", base_url, link);

        if !course.is_empty() {
            faculty.course = course;

            push_group(&url, faculty.clone(), count, client, pool).await?;
        }
    }

    Ok(())
}

async fn push_group(
    url: &str,
    mut faculty: Faculty,
    count: &mut i32,
    client: &Client,
    pool: &PgPool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let base_url = "https://ssau.ru";

    let response = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&response);
    let selector = Selector::parse("a.group-catalog__group").unwrap();

    for element in document.select(&selector) {
        let group = element
            .text()
            .collect::<String>()
            .trim()
            .to_string();

        let link = element.value().attr("href").unwrap_or("").to_string();
        let url = format!("{}{}", base_url, link);

        if !group.is_empty() {
            faculty.group = group;
            faculty.url = url;

            sqlx::query(r#"INSERT INTO faculties (name, course, "group", url) VALUES ($1, $2, $3, $4) ON CONFLICT (url) DO NOTHING"#)
                .bind(&faculty.name)
                .bind(&faculty.course)
                .bind(&faculty.group)
                .bind(&faculty.url)
                .execute(pool)
                .await?;

            *count += 1;
        }
    }

    Ok(())
}
