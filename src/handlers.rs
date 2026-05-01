use crate::Command;
use std::error::Error;
use teloxide::{prelude::*, utils::command::BotCommands};

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

pub async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let start_message = "Приветствую, пользователь!\
Я бот для просмотра расписания СамГУ\
\
Введите /help для просмотра доступных команд";
    bot.send_message(msg.chat.id, start_message).await?;
    Ok(())
}

pub async fn help(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();
    bot.send_message(msg.chat.id, help_text).await?;
    Ok(())
}
