use reqwest::{self, Client};
use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;

#[derive(Serialize, Debug)]
struct Article {
    title: String,
    url: String,
}

#[tokio::test]
async fn parser() -> Result<(), Box<dyn Error>> {
    // 1. URL страницы для парсинга
    let url = "https://ssau.ru/rasp";

    // 2. Делаем HTTP-запрос
    // Добавляем User-Agent, чтобы сайт не принял нас за простого бота
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build()?;

    let response = client.get(url).send().await?.text().await?;

    // 3. Создаем парсер HTML
    let document = Html::parse_document(&response);

    // Определяем CSS-селекторы (например, ищем теги <a> внутри <h2 class="post-title">)
    let article_selector = Selector::parse("h2.h2-text faculties__title").unwrap();
    let link_selector = Selector::parse("a").unwrap();

    let mut articles: Vec<Article> = Vec::new();

    // 4. Перебираем найденные элементы
    for element in document.select(&article_selector) {
        // Извлекаем текст заголовка
        let title = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        // Извлекаем ссылку
        let url = element
            .select(&link_selector)
            .next()
            .and_then(|el| el.value().attr("href"))
            .unwrap_or("")
            .to_string();

        if !title.is_empty() {
            articles.push(Article { title, url });
        }
    }

    // 5. Сериализация в JSON
    let json_data = serde_json::to_string_pretty(&articles)?;

    // Выводим результат в консоль
    println!("{}", json_data);

    // (Опционально) Записываем в файл
    std::fs::write("output.json", json_data)?;
    println!("\nДанные сохранены в output.json");

    Ok(())
}

#[derive(Serialize, Debug)]
struct Faculty {
    name: String,
    url: String,
}

#[tokio::test]
async fn get_facults() -> Result<(), Box<dyn Error>> {
    let url = "https://ssau.ru/rasp";
    let base_url = "https://ssau.ru";

    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()?;

    log::info!("Запрашиваю данные с {}...", url);

    let response = client.get(url).send().await?.text().await?;

    let document = Html::parse_document(&response);

    // 5. Создаем селектор для поиска элементов
    // Ищем тег <a> внутри блоков с классом .faculties__item
    let selector = Selector::parse(".faculties__item a").unwrap();

    let mut faculties: Vec<Faculty> = Vec::new();

    for element in document.select(&selector) {
        // Извлекаем текст (название факультета)
        let name = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();

        // Извлекаем атрибут href
        let link = element.value().attr("href").unwrap_or("").to_string();

        if !name.is_empty() {
            faculties.push(Faculty {
                name,
                url: format!("{}{}", base_url, link),
            });
        }
    }

    let json_result = serde_json::to_string_pretty(&faculties)?;

    log::info!("--- Результат парсинга (JSON) ---");
    log::info!("{}", json_result);

    std::fs::write("ssau_faculties.json", json_result)?;

    log::info!("\nГотово! Данные сохранены в файл ssau_faculties.json");

    Ok(())
}
