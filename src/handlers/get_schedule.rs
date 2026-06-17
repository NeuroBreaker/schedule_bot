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
                Пожалуйста, свяжитесь с разработчиком, и скажите ему, что он рукожоп\
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

async fn update_day_message(
    bot: &Bot,
    pool: &PgPool,
    qmsg: MaybeInaccessibleMessage,
    schedule: &mut Schedule,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let notify = if true
    /* schedule.is_changed().await */
    {
        //
        //
        //
        // НЕОБХОДИМ ФИКС schedule.format_day(weekly_storage)
        // А ТАКЖЕ schedule.is_changed()
        //
        //
        //
        let mut day_schedule: String = String::new();

        if let Ok(row) = schedule.get_db_row(&pool).await {
            if let Some(row) = row {
                let weekly_storage: serde_json::Value = row.get("schedule");
                day_schedule = schedule.format_day(weekly_storage).await;
            } else {
                day_schedule = "На этот день расписания нету".to_string();
            }
        }

        let keyboard = day_keyboard().await?;
        bot.edit_message_text(qmsg.chat().id, qmsg.id(), day_schedule)
            .reply_markup(keyboard)
            .parse_mode(ParseMode::Html)
            .await?;

        None
    } else {
        Some("Расписание не изменилось".to_string())
    };

    Ok(notify)
}

async fn update_week_message(
    bot: &Bot,
    pool: &PgPool,
    qmsg: MaybeInaccessibleMessage,
    schedule: &mut Schedule,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let notify = if schedule.is_changed().await {
        let mut week_schedule: String = String::new();

        if let Ok(row) = schedule.get_db_row(&pool).await {
            if let Some(row) = row {
                let weekly_storage: serde_json::Value = row.get("schedule");
                week_schedule = schedule.format_week(weekly_storage).await;
            } else {
                week_schedule = "На этот день расписания нету".to_string();
            }
        }

        let keyboard = week_keyboard().await?;
        bot.edit_message_text(qmsg.chat().id, qmsg.id(), week_schedule)
            .reply_markup(keyboard)
            .parse_mode(ParseMode::Html)
            .await?;

        None
    } else {
        Some("Расписание не изменилось".to_string())
    };

    Ok(notify)
}

pub async fn week_schedule_callback_handler(
    bot: Bot,
    pool: PgPool,
    dialogue: MyDialogue,
    q: CallbackQuery,
    mut schedule: Schedule,
) -> HandlerResult {
    let mut notify = None;
    if let Some(msg) = q.message
        && let Some(data) = q.data
    {
        match &*data {
            "previous week" => {
                if schedule.date.week > 1 {
                    schedule.date.week -= 1;
                }

                notify = update_week_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "update week" => {
                notify = update_week_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "next week" => {
                if schedule.date.week <= 65534 {
                    schedule.date.week += 1
                }

                notify = update_week_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "this week" => {
                schedule.date.week = 0;
                schedule.date.weekday = 0;

                notify = update_week_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "day" => {
                notify = update_day_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            _ => (),
        }
    }

    if let Some(messg) = notify {
        bot.answer_callback_query(q.id).text(messg).await?;
    } else {
        bot.answer_callback_query(q.id).await?;
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
    let mut notify = None;
    if let Some(msg) = q.message
        && let Some(data) = q.data
    {
        match &*data {
            "previous day" => {
                if schedule.date.weekday == 1 {
                    schedule.date.weekday = 7;
                    schedule.date.week -= 1;
                } else {
                    schedule.date.weekday -= 1;
                }

                update_day_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "update day" => {
                if schedule.is_changed().await {
                    update_day_message(&bot, &pool, msg, &mut schedule).await?;
                } else {
                    notify = Some("Расписание не изменилось".to_string());
                }
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "next day" => {
                if schedule.date.weekday == 7 {
                    schedule.date.weekday = 1;
                    schedule.date.week += 1;
                } else {
                    schedule.date.weekday += 1;
                }

                update_day_message(&bot, &pool, msg, &mut schedule).await?;
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "today" => {
                schedule.date.week = 0;
                schedule.date.weekday = 0;

                if schedule.is_changed().await {
                    update_day_message(&bot, &pool, msg, &mut schedule).await?;
                } else {
                    notify = Some("Расписание не изменилось".to_string());
                }
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "week" => {
                notify = update_week_message(&bot, &pool, msg, &mut schedule).await?;

                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            _ => (),
        }
    }

    if let Some(messg) = notify {
        bot.answer_callback_query(q.id).text(messg).await?;
    } else {
        bot.answer_callback_query(q.id).await?;
    }
    Ok(())
}
