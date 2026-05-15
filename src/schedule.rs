use chrono::{DateTime, Local};
use reqwest::Client;
use scraper::{Html, Selector};
use sqlx::{PgPool, Row};
use std::error::Error;

#[derive(Default, Clone, Debug)]
pub struct Date {
    pub week: i16,
    pub day: i16,
}

impl Date {
    pub fn new() -> Date {
        Date { week: 0, day: 0 }
    }
}

#[derive(Clone, Debug)]
struct Lesson {
    time: String,
    discipline: String,
    place: String,
    teacher: String,
    subgroup: String,
    lesson_type: String,
}

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

// Return Result<String> of week schedule for html parsing
pub async fn week(
    user_id: i64,
    date: &Date,
    pool: &PgPool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    let result = if let Some(mut url) = get_user_url(pool, user_id).await? {
        if date.week != 0 {
            url += &format!("&selectedWeek={}", date.week);
        }

        let response = client.get(url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let container_selector = Selector::parse(".schedule__items > div").unwrap();
        let date_item_selector = Selector::parse(".weekday-nav__item").unwrap();
        let time_item_selector = Selector::parse(".schedule__time-item").unwrap();
        let lesson_selector = Selector::parse(".schedule__lesson").unwrap();
        let disc_selector = Selector::parse(".schedule__discipline").unwrap();
        let place_selector = Selector::parse(".schedule__place").unwrap();
        let teacher_selector = Selector::parse(".schedule__teacher").unwrap();
        let groups_selector = Selector::parse(".schedule__groups").unwrap();
        let type_selector = Selector::parse(".schedule__lesson-type-chip").unwrap();

        let mut weekly_storage: Vec<Vec<Lesson>> = vec![vec![]; 6];
        let mut lesson_time = String::new();
        let mut day_index = 0;

        for element in document.select(&container_selector) {
            let class_attr = element.value().attr("class").unwrap_or("");

            if class_attr.contains("schedule__time") {
                let times: Vec<_> = element
                    .select(&time_item_selector)
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .collect();
                if times.len() >= 2 {
                    lesson_time = format!("{} - {}", times[0], times[1]);
                }
                day_index = 0;
                continue;
            }

            if class_attr.contains("schedule__item") && !class_attr.contains("schedule__head") {
                for lesson_node in element.select(&lesson_selector) {
                    let discipline = lesson_node
                        .select(&disc_selector)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();

                    if !discipline.is_empty() {
                        let place = lesson_node
                            .select(&place_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_else(|| "---".to_string());

                        let teacher = lesson_node
                            .select(&teacher_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        let subgroup = lesson_node
                            .select(&groups_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        let lesson_type = lesson_node
                            .select(&type_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        if let Some(day_storage) = weekly_storage.get_mut(day_index) {
                            day_storage.push(Lesson {
                                time: lesson_time.clone(),
                                discipline,
                                place,
                                teacher,
                                subgroup,
                                lesson_type,
                            });
                        }
                    }
                }
                day_index += 1;
            }
        }

        let mut day_names = vec![];
        for element in document.select(&date_item_selector) {
            let day = element.text().collect::<String>().trim().to_string();

            day_names.push(day);
        }

        let mut schedule_text = String::new();

        if date.week == 0 {
            let local: DateTime<Local> = Local::now();
            date.week = local.weekday();
            date.day = ;
        }

        schedule_text.push_str("────────────────────\n");
        for (i, day_lessons) in weekly_storage.iter().enumerate() {
            if day_lessons.is_empty() {
                continue;
            }

            schedule_text.push_str(&format!("{}\n", day_names[i].to_uppercase()));
            for lesson in day_lessons {
                schedule_text.push_str(&format!(
                    "<b>{}</b> ({})\n{}\n",
                    lesson.discipline, lesson.lesson_type, lesson.place
                ));
                schedule_text.push_str(&format!("      🕒 {}\n", lesson.time));

                if !lesson.teacher.is_empty() {
                    schedule_text.push_str(&format!("      👤 {}\n", lesson.teacher));
                }

                if !lesson.subgroup.is_empty() {
                    schedule_text.push_str(&format!("      👥 <i>{}</i>\n", lesson.subgroup));
                }
                schedule_text.push('\n');
            }
            schedule_text.push_str("────────────────────\n");
        }

        if schedule_text.is_empty() {
            "На этой неделе пар нет.".to_string()
        } else {
            schedule_text
        }
    } else {
        "Вас нету в базе данных бота\nВведите /setup для выбора факультета".to_string()
    };

    Ok(result)
}

pub async fn day(
    user_id: i64,
    date: &Date,
    pool: &PgPool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let client = Client::new();

    let result = if let Some(mut url) = get_user_url(pool, user_id).await? {
        if date.week != 0 {
            url += &format!("&selectedWeek={}", date.week);
        }

        let response = client.get(url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let container_selector = Selector::parse(".schedule__items > div").unwrap();
        let date_item_selector = Selector::parse(".weekday-nav__item").unwrap();
        let time_item_selector = Selector::parse(".schedule__time-item").unwrap();
        let lesson_selector = Selector::parse(".schedule__lesson").unwrap();
        let disc_selector = Selector::parse(".schedule__discipline").unwrap();
        let place_selector = Selector::parse(".schedule__place").unwrap();
        let teacher_selector = Selector::parse(".schedule__teacher").unwrap();
        let groups_selector = Selector::parse(".schedule__groups").unwrap();
        let type_selector = Selector::parse(".schedule__lesson-type-chip").unwrap();

        let mut weekly_storage: Vec<Vec<Lesson>> = vec![vec![]; 6];
        let mut lesson_time = String::new();
        let mut day_index = 0;

        for element in document.select(&container_selector) {
            let class_attr = element.value().attr("class").unwrap_or("");

            if class_attr.contains("schedule__time") {
                let times: Vec<_> = element
                    .select(&time_item_selector)
                    .map(|e| e.text().collect::<String>().trim().to_string())
                    .collect();
                if times.len() >= 2 {
                    lesson_time = format!("{} - {}", times[0], times[1]);
                }
                day_index = 0;
                continue;
            }

            if class_attr.contains("schedule__item") && !class_attr.contains("schedule__head") {
                for lesson_node in element.select(&lesson_selector) {
                    let discipline = lesson_node
                        .select(&disc_selector)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();

                    if !discipline.is_empty() {
                        let place = lesson_node
                            .select(&place_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_else(|| "---".to_string());

                        let teacher = lesson_node
                            .select(&teacher_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        let subgroup = lesson_node
                            .select(&groups_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        let lesson_type = lesson_node
                            .select(&type_selector)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        if let Some(day_storage) = weekly_storage.get_mut(day_index) {
                            day_storage.push(Lesson {
                                time: lesson_time.clone(),
                                discipline,
                                place,
                                teacher,
                                subgroup,
                                lesson_type,
                            });
                        }
                    }
                }
                day_index += 1;
            }
        }

        let mut day_names = vec![];
        for element in document.select(&date_item_selector) {
            let day = element.text().collect::<String>().trim().to_string();

            day_names.push(day);
        }

        let mut schedule_text = String::new();

        for (i, day_lessons) in weekly_storage.iter().enumerate() {
            if day_names[i].contains(&date.day.to_string()) {
                schedule_text.push_str(&format!("\n"));

                for lesson in day_lessons {
                    schedule_text.push_str(&format!(
                        "<b>{}</b> ({})\n{}\n",
                        lesson.discipline, lesson.lesson_type, lesson.place
                    ));
                    schedule_text.push_str(&format!("      🕒 {}\n", lesson.time));

                    if !lesson.teacher.is_empty() {
                        schedule_text.push_str(&format!("      👤 {}\n", lesson.teacher));
                    }

                    if !lesson.subgroup.is_empty() {
                        schedule_text.push_str(&format!("      👥 <i>{}</i>\n", lesson.subgroup));
                    }
                    schedule_text.push('\n');
                }
                schedule_text.push_str("────────────────────\n");
            }
        }

        if schedule_text.is_empty() {
            "Сегодня пар нет.".to_string()
        } else {
            schedule_text
        }
    } else {
        "Вас нету в базе данных бота\nВведите /setup для выбора факультета".to_string()
    };

    Ok(result)
}
