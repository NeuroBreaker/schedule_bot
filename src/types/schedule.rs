use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, Utc, Weekday};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use xxhash_rust::const_xxh3::xxh3_64;
use std::error::Error;

type MyError = Box<dyn Error + Send + Sync>;

#[derive(Clone, Debug)]
pub struct Date {
    pub week: u16,
    pub weekday: u8,
}

impl Date {
    pub fn new() -> Date {
        Date {
            week: 0,
            weekday: 0,
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

#[derive(Clone, Debug)]
struct SiteData {
    url: String,
    //last_modified: String,
}

#[derive(Clone, Debug)]
pub struct Schedule {
    pub date: Date,
    site: SiteData,
    days: Vec<String>,
}

impl Schedule {
    pub fn new(url: String) -> Schedule {
        Schedule {
            date: Date::new(),
            site: SiteData {
                url,
            },
            days: vec!["".to_string()],
        }
    }
    async fn compute_hash(&self, storage: &Vec<Vec<Lesson>>) -> i64 {
        let serialized = serde_json::to_vec(storage).unwrap();
        xxh3_64(&serialized) as i64
    }

    fn build_days(week: u16) -> Vec<String> {
        let current = Utc::now();
        let year = if week > 50 && current.iso_week().week() < 5 {
            current.year() - 1
        } else {
            current.year()
        };

        let monday = NaiveDate::from_isoywd_opt(year, week as u32, Weekday::Mon).unwrap();

        let names = ["Понедельник", "Вторник", "Среда", "Четверг", "Пятница", "Суббота"];

        (0..6)
            .map(|i| {
                let date = monday + chrono::Duration::days(i);
                format!("{} {}", names[i as usize], date.format("%d.%m"))
            })
            .collect()
    }
    
    async fn push_into_db(&self, pool: &PgPool, storage: &Vec<Vec<Lesson>>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let json = serde_json::to_value(storage)?;

        sqlx::query(
            r#"
                INSERT INTO schedules (week, schedule, hash, faculty_id)
                SELECT $1, $2, $3, id
                FROM faculties
                WHERE url = $4
                ON CONFLICT (faculty_id) DO UPDATE
                    SET data = EXCLUDED.data,
                        hash = EXCLUDED.hash
                    WHERE schedules.hash != EXCLUDED.hash
            "#
        )
        .bind(self.date.week as i64)
        .bind(json)
        .bind(self.compute_hash(storage).await)
        .bind(&self.site.url)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn get_db_row(&self, pool: &PgPool) -> Result<Option<PgRow>, MyError> {
        let row = sqlx::query(r#"
                SELECT s.data
                FROM schedules s
                JOIN faculties f ON f.id = s.faculty_id
                WHERE f.url = $1 AND s.week = $2
            "#
        )
        .bind(&self.site.url)
        .bind(self.date.week as i64)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    async fn parse(&mut self, pool: &PgPool) -> Result<(), MyError> {
        let mut url = self.site.url.clone();
        if self.date.week != 0 {
            url += &format!("&selectedWeek={}", self.date.week);
        }

        if self.date.weekday != 0 {
            url += &format!("&selectedWeekday={}", self.date.weekday);
        }

        let client = Client::new();

        let response = client.get(url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let container_selector = Selector::parse(".schedule__items > div").unwrap();
        let week_item_selector = Selector::parse(".week-nav-current_week").unwrap();
        let time_item_selector = Selector::parse(".schedule__time-item").unwrap();
        let lesson_selector = Selector::parse(".schedule__lesson").unwrap();
        let disc_selector = Selector::parse(".schedule__discipline").unwrap();
        let place_selector = Selector::parse(".schedule__place").unwrap();
        let teacher_selector = Selector::parse(".schedule__teacher").unwrap();
        let groups_selector = Selector::parse(".schedule__groups").unwrap();
        let type_selector = Selector::parse(".schedule__lesson-type-chip").unwrap();

        let mut weekly_storage: Vec<Vec<Lesson>> = vec![vec![]; 7];
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
        self.push_into_db(pool, &weekly_storage).await?;

        if self.date.week == 0 {
            let current_week_str = &document
                .select(&week_item_selector)
                .map(|e| e.text().collect::<String>().trim().to_string())
                .collect::<String>()[0..2];

            self.date.week = current_week_str.parse::<u8>().unwrap_or(0) as u16;
        }

        Ok(())
    }

    pub async fn fetch_and_save(&mut self, pool: &PgPool) {
        if self.date.weekday == 0 {
            let offset = FixedOffset::east_opt(4 * 60 * 60).unwrap();
            let timezone: DateTime<FixedOffset> = Utc::now().with_timezone(&offset);
            self.date.weekday = timezone.weekday() as u8 + 1;
        }

        let mut i = 0;
        loop {
            if self.parse(pool).await.is_ok() {
                break;
            } else {
                log::warn!("parsing error, trying again");
                log::warn!("{}", i);
            }
            
            i += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn day_is_changed(&mut self) -> bool {
        true
    }

    pub async fn is_changed(&mut self) -> bool {
        true
    }

    async fn format_lessons(
        &self,
        schedule_text: &mut String,
        i: usize,
        day_lessons: &Vec<Lesson>,
    ) {
        schedule_text.push_str(&format!("{}\n", self.days[i]));

        for lesson in day_lessons {
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

    pub async fn format_week(&self, pool: &PgPool) -> String {
        let mut schedule_text = String::new();
        
        if let Ok(row) = self.get_db_row(pool).await {
            if let Some(row) = row {
                let weekly_storage: serde_json::Value = row.get("schedule");

                schedule_text = weekly_storage.to_string();
            }
        }


        //for (i, day_lessons) in self.weekly_storage.iter().enumerate() {
        //    if day_lessons.is_empty() {
        //        continue;
        //    }
        //
        //    self.format_lessons(&mut schedule_text, i, day_lessons)
        //        .await;
        //}

        if schedule_text.is_empty() {
            "На эту неделю расписания нет.".to_string()
        } else {
            schedule_text
        }
    }

    pub async fn format_day(&self, pool: &PgPool) -> String {
        let mut schedule_text = String::new();

        if let Ok(row) = self.get_db_row(pool).await {
            if let Some(row) = row {
                let weekly_storage: serde_json::Value = row.get("schedule");

                schedule_text = weekly_storage.to_string();
            }
        }

        //let day_vec = &self.weekly_storage.get(self.date.weekday as usize - 1);

        //if let Some(day_schedule) = day_vec {
        //    self.format_lessons(&mut schedule_text, self.date.weekday as usize - 1, day_schedule)
        //        .await;
        //}

        if schedule_text.is_empty() {
            "На этот день расписания нет.".to_string()
        } else {
            schedule_text
        }
    }
}
