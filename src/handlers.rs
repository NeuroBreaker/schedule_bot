use crate::{Command, State, handler_tree::MyDialogue};
use std::error::Error;
use teloxide::{dispatching::dialogue, prelude::*, utils::command::BotCommands};

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

pub async fn message_handler(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let unrecognized_command_text = "Unrecognized command. Say what?";
    let help_text = Command::descriptions().to_string();

    let user_text = msg.text().unwrap_or("");

    let result_text = if user_text.starts_with('/') {
        unrecognized_command_text.to_string()
    } else {
        help_text
    };

    match &*user_text.trim().to_lowercase() {
        "бросить кубик" => {
            bot.send_message(msg.chat.id, "Вы уверены?").await?;
            dialogue.update(State::Dice).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, result_text).await?;
        }
    } 

    Ok(())
}

pub async fn start_handler(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();

    let start_message = "Приветствую, пользователь!\n\
        Я бот для просмотра расписания СамГУ\n\n\
        Введите /help для просмотра доступных команд";

    bot.send_message(msg.chat.id, help_text).await?;

    Ok(())
}

pub async fn cancel_handler(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    dialogue.exit().await?;
    Ok(())
}

pub async fn dice_handler(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let user_text = msg.text().unwrap_or("");
    let user_text_l = user_text.trim().to_lowercase();
    if user_text_l == "yes" || user_text_l == "да" {
        bot.send_dice(msg.chat.id).await?;
    } else {
        dialogue.exit().await?;
    }
    Ok(())
}

pub async fn username_info(bot: Bot, msg: Message) -> HandlerResult {
    Ok(())
}
