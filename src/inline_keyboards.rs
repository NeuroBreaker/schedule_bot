use reqwest::{self, Client};
use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;

use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
};
use url::Url;

#[derive(Serialize, Debug)]
struct Faculty {
    name: String,
    url: String,
}

impl Faculty {
    fn new(name: String, url: String) -> Faculty {
        Faculty { name, url }
    }
}

async fn get_facults() -> Result<Vec<Faculty>, Box<dyn Error + Send + Sync>> {
    let url = "https://ssau.ru/rasp";
    let base_url = "https://ssau.ru";

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    log::info!("Запрашиваю данные с {}...", url);

    let response = client.get(url).send().await?.text().await?;

    let document = Html::parse_document(&response);

    let selector = Selector::parse(".faculties__item a").unwrap();

    let mut faculties: Vec<Faculty> = Vec::new();

    for element in document.select(&selector) {
        let name = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        let link = element.value().attr("href").unwrap_or("").to_string();

        if !name.is_empty() {
            faculties.push(Faculty::new(
                name,
                format!("{}{}", base_url, link),
            ));
        }
    }

    //let json_result = serde_json::to_string_pretty(&faculties)?;

    //log::info!("--- Результат парсинга (JSON) ---");
    //log::info!("{}", json_result);

    //std::fs::write("ssau_faculties.json", json_result)?;
    //log::info!("\nГотово! Данные сохранены в файл ssau_faculties.json");

    Ok(faculties)
}

pub async fn get_institute_markup(
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let faculties = get_facults().await?;
    let mut buttons = Vec::new();

    for f in faculties {
        if let Ok(url) = Url::parse(&f.url) {
            let button = InlineKeyboardButton::url(f.name, url);
            buttons.push(button);
        }
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
        .chunks(2)
        .map(|chunk| chunk.to_vec())
        .collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}

pub async fn get_group_markup(
) -> Result<InlineKeyboardMarkup, Box<dyn Error + Send + Sync>> {
    let mut buttons = Vec::new();

    for g in groups {
        if let Ok(url) = url::parse(&g.url) {
            let button = InlineKeyboardButton::url(g.url, url);
            buttons.push(button);
        }
    }

    let keyboard: Vec<Vec<InlineKeyboardButton>> = buttons
        .chunks(2)
        .map(|chunk| chunk.to_vec())
        .collect();

    Ok(InlineKeyboardMarkup::new(keyboard))
}
