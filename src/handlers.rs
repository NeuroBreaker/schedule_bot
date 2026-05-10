use crate::{
    bot::{Command, State, User}, db::init_db, handler_tree::MyDialogue, inline_keyboards::get_institute_markup
};
use std::error::Error;
use sqlx::PgPool;
use teloxide::{prelude::*, utils::command::BotCommands};

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

pub async fn message_handler(bot: Bot, dialogue: MyDialogue, msg: Message, pool: PgPool) -> HandlerResult {
    let unrecognized_command_text = "Unrecognized command. Say what?";
    let user_text = msg.text().unwrap_or("");

    match &*user_text.trim().to_lowercase() {
        "выбрать институт" => {
            institute_handler(bot, dialogue, msg).await?;
        }
        _ => {
            if user_text.starts_with('/') {
                bot.send_message(msg.chat.id, unrecognized_command_text).await?;
            } else {
                start_handler(bot, msg).await?;
            }
        }
    }

    Ok(())
}

pub async fn start_handler(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();

    let mut start_message = format!("Приветствую, {}!\n\
        Я бот для просмотра расписания СамГУ\n\n\
        Помимо комманд, доступны фразы(не зависят от регистра):\n\n\
        узнать расписание\n\n",
        msg.from.unwrap().first_name);

    start_message += &*help_text;

    bot.send_message(msg.chat.id, start_message).await?;

    Ok(())
}

pub async fn cancel_handler(dialogue: MyDialogue) -> HandlerResult {
    dialogue.exit().await?;
    Ok(())
}

pub async fn institute_handler(bot: Bot, dialogue: MyDialogue, pool: PgPool, msg: Message) -> HandlerResult {
    let keyboard = get_institute_markup(&pool).await?;

    bot.send_message(msg.chat.id, "Выберите свой институт")
        .reply_markup(keyboard)
        .await?;
    
    let mut user = User { ..Default::default() };
    dialogue.update(State::AwaitingCourse(user));

    Ok(())
}

pub async fn course_handler(bot: Bot, msg: Message) -> HandlerResult {
    Ok(())
}

pub async fn group_handler(bot: Bot, msg: Message) -> HandlerResult {
    Ok(())
}

pub async fn schedule_handler(bot: Bot, msg: Message) -> HandlerResult {
    Ok(())
}
