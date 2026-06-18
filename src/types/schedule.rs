use chrono::{DateTime, Datelike, FixedOffset, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgRow};
use std::error::Error;
use xxhash_rust::const_xxh3::xxh3_64;

type MyError = Box<dyn Error + Send + Sync>;

#[derive(Default, Clone, Debug)]
pub struct Date {
    pub week: u16,
    pub weekday: u8,
}

struct Selectors {
    weekday: Selector,
    week: Selector,
    time: Selector,
    disc: Selector,
    place: Selector,
    teacher: Selector,
    groups: Selector,
    lesson_type: Selector,
}

impl Selectors {
    pub fn new() -> Self {
        Self {
            weekday: Selector::parse(".weekday-nav__item").unwrap(),
            week: Selector::parse(".week-nav-current_week").unwrap(),
            time: Selector::parse(".schedule__time-item").unwrap(),
            disc: Selector::parse(".schedule__discipline").unwrap(),
            place: Selector::parse(".schedule__place").unwrap(),
            teacher: Selector::parse(".schedule__teacher").unwrap(),
            groups: Selector::parse(".schedule__groups").unwrap(),
            lesson_type: Selector::parse(".schedule__lesson-type-chip").unwrap(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Lesson {
    time: String,
    discipline: String,
    place: String,
    teacher: String,
    subgroup: String,
    lesson_type: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Day {
    title: String,
    lessons: Vec<Lesson>,
}

impl Day {
    pub fn is_empty(&self) -> bool {
        self.lessons.is_empty()
    }
}

#[derive(Clone, Debug)]
struct SiteData {
    url: String,
    client: Client,
    //last_modified: String,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub date: Date,
    site: SiteData,
}

impl Schedule {
    pub fn new(url: String) -> Schedule {
        Schedule {
            date: Date::default(),
            site: SiteData {
                url,
                client: Client::new(),
            },
        }
    }

    async fn compute_hash(&self, json: &serde_json::Value) -> i64 {
        let serialized = serde_json::to_vec(json).unwrap();
        xxh3_64(&serialized) as i64
    }

    async fn build_days(&self, document: Html, weekday_selector: &Selector) -> Vec<String> {
        document
            .select(&weekday_selector)
            .map(|el| {
                let day = el
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string()
                    .to_uppercase();

                match day {
                    s if s.contains("ПН") => s.replace("ПН", "Понедельник"),
                    s if s.contains("ВТ") => s.replace("ВТ", "Вторник"),
                    s if s.contains("СР") => s.replace("СР", "Среда"),
                    s if s.contains("ЧТ") => s.replace("ЧТ", "Четверг"),
                    s if s.contains("ПТ") => s.replace("ПТ", "Пятница"),
                    s if s.contains("СБ") => s.replace("СБ", "Суббота"),
                    s if s.contains("ВС") => s.replace("ВС", "Воскресенье"),
                    _ => day,
                }
            })
            .collect()
    }

    async fn push_into_db(
        &self,
        pool: &PgPool,
        storage: &Vec<Day>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let json = serde_json::to_value(storage)?;

        sqlx::query(
            r#"
                INSERT INTO schedules (week, schedule, hash, faculty_id)
                SELECT $1, $2, $3, id
                FROM faculties
                WHERE url = $4
                ON CONFLICT (faculty_id, week) DO UPDATE
                    SET schedule = EXCLUDED.schedule,
                        hash = EXCLUDED.hash
                    WHERE schedules.hash != EXCLUDED.hash
            "#,
        )
        .bind(self.date.week as i64)
        .bind(&json)
        .bind(self.compute_hash(&json).await)
        .bind(&self.site.url)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_db_row(&self, pool: &PgPool) -> Result<Option<PgRow>, MyError> {
        let row = sqlx::query(
            r#"
                SELECT s.schedule
                FROM schedules s
                JOIN faculties f ON f.id = s.faculty_id
                WHERE f.url = $1 AND s.week = $2
            "#,
        )
        .bind(&self.site.url)
        .bind(self.date.week as i64)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    async fn assign_current_week(&mut self) -> Result<(), MyError> {
        let response = self
            .site
            .client
            .get(&self.site.url)
            .send()
            .await?
            .text()
            .await?;
        let document = Html::parse_document(&response);

        let week_item_selector = Selector::parse(".week-nav-current_week").unwrap();

        let current_week_str = &document
            .select(&week_item_selector)
            .map(|e| e.text().collect::<String>().trim().to_string())
            .collect::<String>()[0..2];

        self.date.week = current_week_str.parse::<u8>().unwrap_or(0) as u16;

        Ok(())
    }

    async fn fetch_html(&self) -> Result<Html, MyError> {
        let mut url = self.site.url.clone();

        if self.date.week != 0 {
            url += &format!("&selectedWeek={}", self.date.week);
        }

        if self.date.weekday != 0 {
            url += &format!("&selectedWeekday={}", self.date.weekday);
        }

        let response = self.site.client.get(url).send().await?;
        let response_text = response.text().await?;
        let document = Html::parse_document(&response_text);

        Ok(document)
    }

    async fn parse(&mut self) -> Result<Vec<Day>, MyError> {
        if self.date.week == 0 {
            self.assign_current_week().await?;
        }

        let document = self.fetch_html().await?;

        let selectors = Selectors::new();
        let container_selector = Selector::parse(".schedule__items > div").unwrap();
        let lesson_selector = Selector::parse(".schedule__lesson").unwrap();

        let mut weekly_storage: Vec<Day> = vec![Day::default(); 6];
        let mut lesson_time = String::new();
        let mut day_index = 0;

        for element in document.select(&container_selector) {
            let class_attr = element.value().attr("class").unwrap_or("");

            if class_attr.contains("schedule__time") {
                let times: Vec<_> = element
                    .select(&selectors.time)
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
                        .select(&selectors.disc)
                        .next()
                        .map(|e| e.text().collect::<String>().trim().to_string())
                        .unwrap_or_default();

                    if !discipline.is_empty() {
                        let place = lesson_node
                            .select(&selectors.place)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_else(|| "---".to_string());

                        let teacher = lesson_node
                            .select(&selectors.teacher)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        let subgroup = lesson_node
                            .select(&selectors.groups)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        let lesson_type = lesson_node
                            .select(&selectors.lesson_type)
                            .next()
                            .map(|e| e.text().collect::<String>().trim().to_string())
                            .unwrap_or_default();

                        if let Some(day) = weekly_storage.get_mut(day_index) {
                            day.lessons.push(Lesson {
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
        let days: Vec<String> = document
            .select(&selectors.weekday)
            .map(|el| {
                let day = el
                    .text()
                    .collect::<String>()
                    .trim()
                    .to_string()
                    .to_uppercase();

                match day {
                    s if s.contains("ПН") => s.replace("ПН", "Понедельник"),
                    s if s.contains("ВТ") => s.replace("ВТ", "Вторник"),
                    s if s.contains("СР") => s.replace("СР", "Среда"),
                    s if s.contains("ЧТ") => s.replace("ЧТ", "Четверг"),
                    s if s.contains("ПТ") => s.replace("ПТ", "Пятница"),
                    s if s.contains("СБ") => s.replace("СБ", "Суббота"),
                    s if s.contains("ВС") => s.replace("ВС", "Воскресенье"),
                    _ => day,
                }
            })
            .collect();

        //let days = self.build_days(document, &selectors.weekday).await;
        // ПОЧЕМУ-ТО ВЫЗЫВАЕТ ОШИБКУ ПРИ КОМПИЛЯЦИИ

        for (i, day) in weekly_storage.iter_mut().enumerate() {
            if let Some(t) = days.get(i) {
                day.title = t.clone();
            }
        }

        Ok(weekly_storage)
    }

    pub async fn fetch_and_save(&mut self, pool: &PgPool) {
        if self.date.weekday == 0 {
            let offset = FixedOffset::east_opt(4 * 60 * 60).unwrap();
            let timezone: DateTime<FixedOffset> = Utc::now().with_timezone(&offset);
            self.date.weekday = timezone.weekday() as u8 + 1;
        }

        let mut i = 1;
        loop {
            match self.parse().await {
                Ok(weekly_storage) => {
                    let push_rslt = self.push_into_db(pool, &weekly_storage).await;

                    if let Err(err) = push_rslt {
                        let err_msg = err.to_string();
                        log::error!("push_into_db() error: {}", err_msg);
                    } else {
                        break;
                    }
                }
                Err(err) => {
                    log::warn!("parsing error: {}\nTrying again... ({})", err, i);
                }
            }

            i += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn is_changed(&mut self, old_hash: i64, json: serde_json::Value) -> bool {
        old_hash != self.compute_hash(&json).await
    }

    async fn format_lessons(&self, schedule_text: &mut String, day: &Day) {
        schedule_text.push_str(&format!("<b><i>{}</i></b>\n", day.title));

        if day.is_empty() {
            schedule_text.push_str("<b>Ничего нету</b>\n");
        }

        for lesson in &day.lessons {
            schedule_text.push_str(&format!(
                "<b>{}</b> ({})\n",
                lesson.discipline, lesson.lesson_type
            ));
            schedule_text.push_str(&format!("      🏢 {}\n", lesson.place));
            schedule_text.push_str(&format!("      🕒 {}\n", lesson.time));

            if !lesson.teacher.is_empty() {
                schedule_text.push_str(&format!("      👤 {}\n", lesson.teacher));
            }

            if !lesson.subgroup.is_empty() {
                schedule_text.push_str(&format!("      👥 <i>{}</i>\n", lesson.subgroup));
            }
            schedule_text.push('\n');
        }
        schedule_text.push_str("───────────────────────────\n");
    }

    pub async fn format_week(&self, json: serde_json::Value) -> String {
        let weekly_storage: Vec<Day> = match serde_json::from_value(json) {
            Ok(schedule) => schedule,
            Err(err) => {
                log::error!("{}", err);
                return "Ошибка при чтении расписания\nПередайте разрабу, что он бездарен"
                    .to_string();
            }
        };

        let mut schedule_text: String = String::new();
        for day_lessons in weekly_storage.iter() {
            if day_lessons.is_empty() {
                continue;
            }

            self.format_lessons(&mut schedule_text, day_lessons).await;
        }

        if schedule_text.is_empty() {
            "На эту неделю расписания нет.".to_string()
        } else {
            schedule_text
        }
    }

    pub async fn format_day(&self, json: serde_json::Value) -> String {
        let weekly_storage: Vec<Day> = match serde_json::from_value(json) {
            Ok(schedule) => schedule,
            Err(err) => {
                log::error!("{}", err);
                return "Ошибка при чтении расписания\nПередайте разрабу, что он бездарен"
                    .to_string();
            }
        };

        let day_vec = &weekly_storage.get(self.date.weekday as usize - 1);

        let mut schedule_text = String::new();
        if let Some(day_schedule) = day_vec {
            self.format_lessons(&mut schedule_text, day_schedule).await;
        }

        if schedule_text.is_empty() {
            "На этот день расписания нет.".to_string()
        } else {
            schedule_text
        }
    }
}
