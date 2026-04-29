use crate::{Command, HandlerResult};
use teloxide::{prelude::*, utils::command::BotCommands};

pub async fn start(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();
    bot.send_message(msg.chat.id, help_text).await?;
    Ok(())
}
