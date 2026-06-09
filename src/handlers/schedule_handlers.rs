use sqlx::PgPool;
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{MaybeInaccessibleMessage, ParseMode},
};

use crate::{
    bot::State,
    handler_tree::MyDialogue,
    handlers::HandlerResult,
    utils::inline_keyboards::{day_keyboard, week_keyboard},
    types::schedule::Schedule,
    utils::get_user_url,
};

pub async fn schedule_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    pool: PgPool,
) -> HandlerResult {
    let user_id = msg.from.as_ref().unwrap().id.0 as i64;

    if let Some(url) = get_user_url(&pool, user_id).await? {
        let schedule = Schedule::new(url).await?;
        let week_schedule = schedule.format_week().await;

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
    qmsg: MaybeInaccessibleMessage,
    schedule: &mut Schedule,
) -> HandlerResult {
    let day_schedule = schedule.format_day().await;



    Ok(())
}

async fn update_week_message(
    bot: &Bot,
    qmsg: MaybeInaccessibleMessage,
    schedule: &mut Schedule,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let notify = if schedule.is_changed().await {
        let week_schedule = schedule.format_week().await;

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

                notify = update_week_message(&bot, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "update week" => {
                notify = update_week_message(&bot, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "next week" => {
                if schedule.date.week <= 65534 {
                    schedule.date.week += 1
                }

                notify = update_week_message(&bot, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "this week" => {
                schedule.date.week = 0;
                schedule.date.weekday = 0;

                notify = update_week_message(&bot, msg, &mut schedule).await?;
                dialogue.update(State::WeekSchedule(schedule)).await?;
            }
            "day" => {
                update_day_message(&bot, msg, &mut schedule).await?;
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

                update_day_message(&bot, msg, &mut schedule).await?;
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "update day" => {
                if schedule.is_changed().await {
                    update_day_message(&bot, msg, &mut schedule).await?;
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

                update_day_message(&bot, msg, &mut schedule).await?;
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "today" => {
                schedule.date.week = 0;
                schedule.date.weekday = 0;

                if schedule.is_changed().await {
                    update_day_message(&bot, msg, &mut schedule).await?;
                } else {
                    notify = Some("Расписание не изменилось".to_string());
                }
                dialogue.update(State::DaySchedule(schedule)).await?;
            }
            "week" => {
                let week_schedule = schedule.format_week().await;

                let keyboard = week_keyboard().await?;
                bot.edit_message_text(msg.chat().id, msg.id(), week_schedule)
                    .reply_markup(keyboard)
                    .parse_mode(ParseMode::Html)
                    .await?;

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
