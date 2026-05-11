use crate::{
    bot::{Command, State, User},
    handler_tree::MyDialogue,
    inline_keyboards::{courses_keyboard, groups_keyboard, instituts_keyboard, week_keyboard}, schedule::week,
};
use sqlx::{PgPool, Row};
use std::error::Error;
use teloxide::{prelude::*, types::{LinkPreviewOptions, ParseMode}, utils::command::BotCommands};

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

pub async fn message_handler(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    pool: PgPool,
) -> HandlerResult {
    let unrecognized_command_text = "Unrecognized command. Say what?";
    let user_text = msg.text().unwrap_or("");

    match &*user_text.trim().to_lowercase() {
        "начать" => {
            setup_handler(bot, dialogue, msg, pool).await?;
        }
        "узнать расписание" => {
            schedule_handler(bot, msg, pool).await?;
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
    
    start_message += 
        "\n\nДля тех, кто хочет помочь с разработкой бота:\n\
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

            if let Some(msg) = q.message
            {
                bot.edit_message_text(msg.chat().id, msg.id(), format!("Институт: {}\nВыберите курс", institute_name))
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

        if let Some(msg) = q.message
        {
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
                "#
            )
            .bind(q.from.id.0 as i64)
            .bind(&user.institute)
            .bind(&user.course)
            .bind(&user.group)
            .execute(&pool)
            .await?;

            dialogue.exit().await?;
            bot.edit_message_text(msg.chat().id, msg.id(), "Настройка успешно завершена").await?;
        }
    }


    bot.answer_callback_query(q.id)
        .text("Настройка закончена, можете узнать расписание\n\n\
            /schedule\n  или\nузнать расписание").await?;
    Ok(())
}

pub async fn schedule_handler(
    bot: Bot,
    msg: Message,
    pool: PgPool
) -> HandlerResult {
    let keyboard = week_keyboard().await?;
    let user_id = msg.from.as_ref().unwrap().id.0 as i64;
    let result = week(user_id, &pool).await?;

    bot.send_message(msg.chat.id, result)
        .reply_markup(keyboard)
        .parse_mode(ParseMode::Html)
        .await?;

    Ok(())
}
