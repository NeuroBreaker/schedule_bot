use sqlx::{PgPool, Row};
use std::error::Error;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

pub async fn instituts_keyboard(
    pool: &PgPool,
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query("SELECT name, MIN(id) as id FROM faculties GROUP BY name ORDER BY name")
        .fetch_all(pool)
        .await?;

    let mut buttons = Vec::new();

    for row in rows {
        let name: String = row.get("name");
        let id: i32 = row.get("id");

        let button = InlineKeyboardButton::callback(name, id.to_string());
        buttons.push(button);
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> =
        buttons.chunks(1).map(|chunk| chunk.to_vec()).collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}

pub async fn courses_keyboard(
    pool: &PgPool,
    institute_name: &str,
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query("SELECT DISTINCT course FROM faculties WHERE name = $1 ORDER BY course")
        .bind(institute_name)
        .fetch_all(pool)
        .await?;

    let mut buttons = Vec::new();

    for row in rows {
        let course: String = row.get("course");
        let button = InlineKeyboardButton::callback(course.clone(), course);
        buttons.push(button);
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> =
        buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}

pub async fn groups_keyboard(
    pool: &PgPool,
    institute_name: &str,
    course: &str,
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query(
        r#"SELECT DISTINCT "group" FROM faculties
            WHERE name = $1 AND course = $2 ORDER BY "group""#,
    )
    .bind(institute_name)
    .bind(course)
    .fetch_all(pool)
    .await?;

    let mut buttons = Vec::new();

    for row in rows {
        let group: String = row.get("group");
        let button = InlineKeyboardButton::callback(group.clone(), group);
        buttons.push(button);
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> =
        buttons.chunks(2).map(|chunk| chunk.to_vec()).collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}
