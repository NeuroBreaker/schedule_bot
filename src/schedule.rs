use chrono::{DateTime, Datelike, FixedOffset, Utc};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use std::error::Error;

type MyError = Box<dyn Error + Send + Sync>;

#[derive(Default, Clone, Debug)]
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

#[derive(Default, Clone, Debug)]
struct SiteData {
    url: String,
    client: Client,
    //last_modified: String,
}

#[derive(Default, Clone, Debug)]
pub struct Schedule {
    pub date: Date,
    site: SiteData,
    days: Vec<String>,
    weekly_storage: Vec<Vec<Lesson>>,
}

impl Schedule {
    async fn parse(&mut self) -> Result<(), MyError> {
        let mut url = self.site.url.clone();
        if self.date.week != 0 {
            url += &format!("&selectedWeek={}", self.date.week);
        }

        if self.date.weekday != 0 {
            url += &format!("&selectedWeekday={}", self.date.weekday);
        }

        let response = self.site.client.get(url).send().await?.text().await?;
        let document = Html::parse_document(&response);
        let container_selector = Selector::parse(".schedule__items > div").unwrap();
        let date_item_selector = Selector::parse(".weekday-nav__item").unwrap();
        let week_item_selector = Selector::parse(".week-nav-current_week").unwrap();
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
        self.weekly_storage = weekly_storage;

        self.days = document
            .select(&date_item_selector)
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
                    _ => day,
                }
            })
            .collect();

        if self.date.week == 0 {
            let current_week_str = &document
                .select(&week_item_selector)
                .map(|e| e.text().collect::<String>().trim().to_string())
                .collect::<String>()[0..2];

            self.date.week = current_week_str.parse::<u8>().unwrap_or(0) as u16;
        }

        Ok(())
    }

    pub async fn new(url: String) -> Result<Schedule, Box<dyn Error + Send + Sync>> {
        let mut date = Date::new();

        if date.weekday == 0 {
            let offset = FixedOffset::east_opt(4 * 60 * 60).unwrap();
            let timezone: DateTime<FixedOffset> = Utc::now().with_timezone(&offset);
            date.weekday = timezone.weekday() as u8 + 1;
        }

        let mut schedule = Schedule {
            date,
            site: SiteData {
                url,
                client: Client::new(),
            },
            ..Default::default()
        };

        schedule.parse().await?;

        Ok(schedule)
    }

    pub async fn is_changed(&mut self) -> bool {
        let previous_weely_storage = self.weekly_storage.clone();

        let _ = self.parse().await;

        previous_weely_storage != self.weekly_storage
    }

    async fn push_schedule(pool: &PgPool) -> Result<(), Box<dyn Error + Send + Sync>> {
        sqlx::query(r#"
            UPDATE 
        "#)
        .execute(pool)
        .await?;

        Ok(())
    }
    
    //async fn get_lessons(pool: &PgPool) {
    //
    //}

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

    pub async fn format_week(&self) -> String {
        let mut schedule_text = String::new();

        for (i, day_lessons) in self.weekly_storage.iter().enumerate() {
            if day_lessons.is_empty() {
                continue;
            }

            self.format_lessons(&mut schedule_text, i, day_lessons)
                .await;
        }

        if schedule_text.is_empty() {
            "На эту неделю расписания нет.".to_string()
        } else {
            schedule_text
        }
    }

    pub async fn format_day(&self) -> String {
        let mut schedule_text = String::new();
        let day_vec = &self.weekly_storage.get(self.date.weekday as usize - 1);

        if let Some(day_schedule) = day_vec {
            self.format_lessons(&mut schedule_text, self.date.weekday as usize - 1, day_schedule)
                .await;
        }

        if schedule_text.is_empty() {
            "На этот день расписания нет.".to_string()
        } else {
            schedule_text
        }
    }
}
