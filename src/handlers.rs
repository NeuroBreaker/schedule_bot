use crate::{
    bot::{Command, State},
    handler_tree::MyDialogue,
    inline_keyboards::{
        courses_keyboard, day_keyboard, groups_keyboard, instituts_keyboard, week_keyboard,
    },
    schedule::{Date, Schedule},
    utils::get_user_url,
};
use sqlx::{PgPool, Row};
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{LinkPreviewOptions, MaybeInaccessibleMessage, ParseMode},
    utils::command::BotCommands,
};

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Default, Clone, Debug)]
pub struct User {
    institute: String,
    course: String,
    group: String,
}

pub async fn message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    pool: PgPool,
) -> HandlerResult {
    let unrecognized_command_text = "Неизвестная команда, введите /help для просмотра доступных";
    let user_text = msg.text().unwrap_or("");

    match &*user_text.trim().to_lowercase() {
        "начать" => {
            setup_handler(bot, dialogue, msg, pool).await?;
        }
        "узнать расписание" => {
            schedule_handler(bot, dialogue, msg, pool).await?;
        }
        _ => {
            if user_text.starts_with('/') {
                bot.send_message(msg.chat.id, unrecognized_command_text)
                    .await?;
            } else {
                start_handler(bot, msg).await?;
            }
        }
    }

    Ok(())
}

pub async fn start_handler(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();

    let mut start_message = format!(
        "Приветствую, {}!\n\
        Я бот для просмотра расписания СамГУ\n\n\
        Помимо комманд, доступны фразы(не зависят от регистра):\n\n\
        начать\n\
        узнать расписание\n\n",
        msg.from.unwrap().first_name
    );

    start_message += &*help_text;

    start_message += "\n\nДля тех, кто хочет помочь с разработкой бота:\n\
        https://github.com/NeuroBreaker/schedule_bot";

    bot.send_message(msg.chat.id, start_message)
        .link_preview_options(LinkPreviewOptions {
            is_disabled: true,
            url: None,
            prefer_small_media: false,
            prefer_large_media: false,
            show_above_text: false,
        })
        .await?;

    Ok(())
}

pub async fn cancel_handler(dialogue: MyDialogue) -> HandlerResult {
    dialogue.exit().await?;
    Ok(())
}

pub async fn setup_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    pool: PgPool,
) -> HandlerResult {
    let keyboard = instituts_keyboard(&pool).await?;

    bot.send_message(msg.chat.id, "Выберите свой институт")
        .reply_markup(keyboard)
        .await?;

    dialogue.update(State::AwaitingInstitute).await?;

    Ok(())
}

pub async fn institute_callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    pool: PgPool,
    q: CallbackQuery,
) -> HandlerResult {
    if let Some(data) = q.data {
        let institute_id: i32 = data.parse().unwrap_or(0);

        let row = sqlx::query(r#"SELECT name FROM faculties WHERE id = $1"#)
            .bind(institute_id)
            .fetch_optional(&pool)
            .await?;

        if let Some(row) = row {
            let institute_name: String = row.get("name");

            let keyboard = courses_keyboard(&pool, &institute_name).await?;

            if let Some(msg) = q.message {
                bot.edit_message_text(
                    msg.chat().id,
                    msg.id(),
                    format!("Институт: {}\nВыберите курс", institute_name),
                )
                .reply_markup(keyboard)
                .await?;
            }

            let user = User {
                institute: institute_name,
                ..Default::default()
            };
            dialogue.update(State::AwaitingCourse(user)).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

pub async fn course_callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    mut user: User,
    pool: PgPool,
    q: CallbackQuery,
) -> HandlerResult {
    if let Some(data) = q.data {
        user.course = data;
        let keyboard = groups_keyboard(&pool, &user.institute, &user.course).await?;

        if let Some(msg) = q.message {
            bot.edit_message_text(msg.chat().id, msg.id(), "Выберите группу")
                .reply_markup(keyboard)
                .await?;

            dialogue.update(State::AwaitingGroup(user)).await?;
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

pub async fn group_callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    mut user: User,
    pool: PgPool,
    q: CallbackQuery,
) -> HandlerResult {
    if let Some(data) = q.data {
        user.group = data;

        if let Some(msg) = q.message {
            sqlx::query(
                r#"
                    INSERT INTO users (id, faculty_id)
                    SELECT $1, id
                    FROM faculties
                    WHERE name = $2 AND course = $3 AND "group" = $4
                    ON CONFLICT (id) DO UPDATE SET faculty_id = EXCLUDED.faculty_id
                "#,
            )
            .bind(q.from.id.0 as i64)
            .bind(&user.institute)
            .bind(&user.course)
            .bind(&user.group)
            .execute(&pool)
            .await?;

            dialogue.exit().await?;
            bot.edit_message_text(msg.chat().id, msg.id(), "Настройка успешно завершена")
                .await?;
        }
    }

    bot.answer_callback_query(q.id)
        .text(
            "Настройка закончена, можете узнать расписание\n\n\
            /schedule\n  или\nузнать расписание",
        )
        .await?;
    Ok(())
}

pub async fn schedule_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    pool: PgPool,
) -> HandlerResult {
    let mut date = Date::new();

    let user_id = msg.from.as_ref().unwrap().id.0 as i64;

    let week_schedule = if let Some(url) = get_user_url(&pool, user_id).await? {
        let schedule = Schedule::new(url, &mut date).await?;
        schedule.get_week().await
    } else {
        "Вас нету в базе данных бота\nВведите /setup для выбора факультета".to_string()
    };

    let keyboard = week_keyboard().await?;
    bot.send_message(msg.chat.id, week_schedule)
        .reply_markup(keyboard)
        .parse_mode(ParseMode::Html)
        .await?;

    dialogue.update(State::WeekSchedule(date)).await?;

    Ok(())
}

async fn update_day_message(
    bot: &Bot,
    qmsg: MaybeInaccessibleMessage,
    pool: PgPool,
    date: &mut Date,
    user_id: i64,
) -> HandlerResult {
    let day_schedule = if let Some(url) = get_user_url(&pool, user_id).await? {
        let schedule = Schedule::new(url, date).await?;
        schedule.get_day(date.weekday).await
    } else {
        "Вас нету в базе данных бота\nВведите /setup для выбора факультета".to_string()
    };

    let keyboard = day_keyboard().await?;
    bot.edit_message_text(qmsg.chat().id, qmsg.id(), day_schedule)
        .reply_markup(keyboard)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

async fn update_week_message(
    bot: &Bot,
    qmsg: MaybeInaccessibleMessage,
    pool: PgPool,
    date: &mut Date,
    user_id: i64,
) -> HandlerResult {
    let week_schedule = if let Some(url) = get_user_url(&pool, user_id).await? {
        let schedule = Schedule::new(url, date).await?;
        schedule.get_week().await
    } else {
        "Вас нету в базе данных бота\nВведите /setup для выбора факультета".to_string()
    };

    let keyboard = week_keyboard().await?;
    bot.edit_message_text(qmsg.chat().id, qmsg.id(), week_schedule)
        .reply_markup(keyboard)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}

pub async fn week_schedule_callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    pool: PgPool,
    mut date: Date,
) -> HandlerResult {
    if let Some(msg) = q.message
        && let Some(data) = q.data
    {
        let user_id = q.from.id.0 as i64;
        match &*data {
            "previous week" => {
                if date.week > 1 {
                    date.week -= 1;
                }

                update_week_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::WeekSchedule(date)).await?;
            }
            "update week" => {
                update_week_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::WeekSchedule(date)).await?;
            }
            "next week" => {
                if date.week <= 65534 {
                    date.week += 1
                }

                update_week_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::WeekSchedule(date)).await?;
            }
            "this week" => {
                let mut date = Date::new();

                update_week_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::WeekSchedule(date)).await?;
            }
            "day" => {
                update_day_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::DaySchedule(date)).await?;
            }
            _ => (),
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}

pub async fn day_schedule_callback_handler(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
    pool: PgPool,
    mut date: Date,
) -> HandlerResult {
    if let Some(msg) = q.message
        && let Some(data) = q.data
    {
        let user_id = q.from.id.0 as i64;

        match &*data {
            "previous day" => {
                if date.weekday == 1 {
                    date.weekday = 7;
                    date.week -= 1;
                } else {
                    date.weekday -= 1;
                }

                update_day_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::DaySchedule(date)).await?;
            }
            "update day" => {
                update_day_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::DaySchedule(date)).await?;
            }
            "next day" => {
                if date.weekday == 7 {
                    date.weekday = 1;
                    date.week += 1;
                } else {
                    date.weekday += 1;
                }

                update_day_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::DaySchedule(date)).await?;
            }
            "today" => {
                let mut date = Date::new();

                update_day_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::DaySchedule(date)).await?;
            }
            "week" => {
                update_week_message(&bot, msg, pool, &mut date, user_id).await?;
                dialogue.update(State::WeekSchedule(date)).await?;
            }
            _ => (),
        }
    }

    bot.answer_callback_query(q.id).await?;
    Ok(())
}
