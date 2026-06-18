use sqlx::{PgPool, Row};
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{MaybeInaccessibleMessage, ParseMode},
};

use crate::{
    bot::State,
    handler_tree::MyDialogue,
    handlers::HandlerResult,
    types::schedule::Schedule,
    utils::get_user_url,
    utils::inline_keyboards::{day_keyboard, week_keyboard},
};

enum Required {
    Week,
    Day,
}

pub async fn schedule_handler(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    pool: PgPool,
) -> HandlerResult {
    let user_id = msg.from.as_ref().unwrap().id.0 as i64;

    if let Some(url) = get_user_url(&pool, user_id).await? {
        let mut schedule: Schedule = Schedule::new(url);
        schedule.fetch_and_save(&pool).await;

        let week_schedule: String = if let Ok(row) = schedule.get_db_row(&pool).await {
            if let Some(row) = row {
                let weekly_storage: serde_json::Value = row.get("schedule");
                schedule.format_week(weekly_storage).await
            } else {
                "На эту неделю расписания нету".to_string()
            }
        } else {
            "\
                Ошибка при получении данных из бд\n\
                Пожалуйста, передайте разработчику, что он бездарен\
            "
            .to_string()
        };

        let keyboard = week_keyboard().await?;

        bot.send_message(msg.chat.id, week_schedule)
            .reply_markup(keyboard)
            .parse_mode(ParseMode::Html)
            .await?;

        dialogue.update(State::WeekSchedule(schedule)).await?;
    } else {
        bot.send_message(
            msg.chat.id,
            "Вас нету в базе данных бота\nВведите /setup для выбора факультета",
        )
        .await?;
        dialogue.exit().await?;
    };

    Ok(())
}

async fn update_message(
    bot: &Bot,
    pool: &PgPool,
    qmsg: MaybeInaccessibleMessage,
    schedule: &mut Schedule,
    required: &Required,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if /* schedule.is_changed(1).await */ true {
        // НЕОБХОДИМ ФИКС schedule.format_day(weekly_storage)
        // А ТАКЖЕ schedule.is_changed()
        let mut schedule_msg: String = String::new();

        if let Ok(row) = schedule.get_db_row(pool).await {
            if let Some(row) = row {
                let weekly_storage: serde_json::Value = row.get("schedule");
                match required {
                    Required::Day => {
                        schedule_msg = schedule.format_day(weekly_storage).await;
                    }
                    Required::Week => {
                        schedule_msg = schedule.format_week(weekly_storage).await;
                    }
                }
            } else {
                schedule_msg = "На этот день расписания нету".to_string();
            }
        }

        let keyboard = day_keyboard().await?;
        bot.edit_message_text(qmsg.chat().id, qmsg.id(), schedule_msg)
            .reply_markup(keyboard)
            .parse_mode(ParseMode::Html)
            .await?;

    }

    Ok(())
}

pub async fn week_schedule_callback_handler(
    bot: Bot,
    pool: PgPool,
    dialogue: MyDialogue,
    q: CallbackQuery,
    mut schedule: Schedule,
) -> HandlerResult {
    if let Some(msg) = q.message
        && let Some(data) = q.data
    {
        let mut required = Required::Week;
        match &*data {
            "previous week" => {
                if schedule.date.week > 1 {
                    schedule.date.week -= 1;
                }
            }
            "update week" => (), 
            "next week" => {
                if schedule.date.week <= 65534 {
                    schedule.date.week += 1
                }
            }
            "this week" => {
                schedule.date.week = 0;
                schedule.date.weekday = 0;
            }
            "day" => {
                required = Required::Day;
            }
            _ => (),
        }
        match update_message(&bot, &pool, msg, &mut schedule, &required).await {
            Ok(_) => {
                bot.answer_callback_query(q.id).await?;
            }
            Err(_) => {
                bot.answer_callback_query(q.id).text("Ошибка обновления(скорее всего текст не изменился)").await?;
            }
        }

        match &required {
            Required::Day => {
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            Required::Week => {
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
        }
    }

    Ok(())
}

pub async fn day_schedule_callback_handler(
    bot: Bot,
    pool: PgPool,
    dialogue: MyDialogue,
    q: CallbackQuery,
    mut schedule: Schedule,
) -> HandlerResult {
    if let Some(msg) = q.message
        && let Some(data) = q.data
    {
        let mut required = Required::Day;
        match &*data {
            "previous day" => {
                if schedule.date.weekday == 1 {
                    schedule.date.weekday = 7;
                    schedule.date.week -= 1;
                } else {
                    schedule.date.weekday -= 1;
                }
            }
            "update day" => (),
            "next day" => {
                if schedule.date.weekday == 7 {
                    schedule.date.weekday = 1;
                    schedule.date.week += 1;
                } else {
                    schedule.date.weekday += 1;
                }
            }
            "today" => {
                schedule.date.week = 0;
                schedule.date.weekday = 0;
            }
            "week" => {
                required = Required::Week;
            }
            _ => (),
        }

        match update_message(&bot, &pool, msg, &mut schedule, &required).await {
            Ok(_) => {
                bot.answer_callback_query(q.id).await?;
            }
            Err(_) => {
                bot.answer_callback_query(q.id).text("Ошибка обновления(скорее всего текст не изменился)").await?;
            }
        }

        match &required {
            Required::Day => {
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            Required::Week => {
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
        }
    }

    Ok(())
}
