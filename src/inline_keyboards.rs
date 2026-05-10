use reqwest::{self, Client};
use scraper::{Html, Selector};
use sqlx::{PgPool, Row};
use std::error::Error;

use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};
use url::Url;


pub async fn get_institute_markup(
    pool: &PgPool
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query("SELECT name FROM faculties ORDER BY name")
        .fetch_all(pool)
        .await?;

    let mut buttons = Vec::new();

    for row in rows {
        let name: String = row.get("name");
        let button = InlineKeyboardButton::callback(name.clone(), name);
        buttons.push(button);
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
        .chunks(2)
        .map(|chunk| chunk.to_vec())
        .collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}

pub async fn get_course_markup(
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let rows = sqlx::query("SELECT name FROM faculties ORDER BY name")
        .fetch_all(pool)
        .await?;

    let mut buttons = Vec::new();

    for row in rows {
        let name: String = row.get("name");
        let button = InlineKeyboardButton::callback(name.clone(), name);
        buttons.push(button);
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
        .chunks(2)
        .map(|chunk| chunk.to_vec())
        .collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}

//pub async fn get_group_markup(
//) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
//    let mut buttons = Vec::new();
//
//    for g in groups {
//        if let Ok(url) = url::parse(&g.url) {
//            let button = InlineKeyboardButton::url(g.url, url);
//            buttons.push(button);
//        }
//    }
//
//    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
//        .chunks(2)
//        .map(|chunk| chunk.to_vec())
//        .collect();
//
//    Ok(InlineKeyboardMarkup::new(keyboard))
//}
