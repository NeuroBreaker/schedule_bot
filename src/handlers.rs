use crate::{
    bot::{Command, State},
    handler_tree::MyDialogue,
    inline_keyboards::get_institute_markup,
};
use std::error::Error;
use teloxide::{prelude::*, utils::command::BotCommands};

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

struct User {

}

pub async fn message_handler(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let unrecognized_command_text = "Unrecognized command. Say what?";
    let user_text = msg.text().unwrap_or("");

    match &*user_text.trim().to_lowercase() {
        "бросить кубик" => {
            bot.send_dice(msg.chat.id).await?;
        }
        "узнать расписание" => {
            institute_handler(bot, msg).await?;
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
        бросить кубик\n\
        узнать расписание\n\n",
        msg.from.unwrap().first_name);

    start_message += &*help_text;

    bot.send_message(msg.chat.id, start_message).await?;

    Ok(())
}

//pub async fn cancel_handler(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
//    dialogue.exit().await?;
//    Ok(())
//}

pub async fn drop_dice(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_dice(msg.chat.id).await?;
    Ok(())
}

pub async fn institute_handler(bot: Bot, msg: Message) -> HandlerResult {
    let keyboard = get_institute_markup(bot.clone(), msg.clone()).await?;

    bot.send_message(msg.chat.id, "Выберите свой институт")
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn schedule_handler(bot: Bot, msg: Message) -> HandlerResult {
    Ok(())
}

//pub async fn wait_institute(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
//    Ok(())
//}
