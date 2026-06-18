use sqlx::PgPool;
use teloxide::{prelude::*, types::LinkPreviewOptions, utils::command::BotCommands};

use crate::{
    bot::{Command, State},
    handler_tree::MyDialogue,
    handlers::{HandlerResult, schedule_handler, setup_handler},
};

pub async fn start_handler(bot: Bot, msg: Message) -> HandlerResult {
    let help_text = Command::descriptions().to_string();

    let mut start_message = format!(
        "Приветствую, {}!\n\
        Я бот для просмотра расписания СамГУ\n\n\
        Краткое руководство:\n\
        Выбираете факультет и группу через /setup\n\
        (Ваш выбор сохраняется до повторного вызова /setup)\n\
        После выбора получаете расписание через /schedule\n\n",
        msg.from.unwrap().first_name
    );

    start_message.push_str(&help_text);

    start_message.push_str("\n\nПомимо комманд, доступны фразы(не зависят от регистра):\n\n\
        - начать\n\
        - расписание\n\n\
        Связь с разработчиком: @rodionr07\n\
        Для тех, кто желает внести свой вклад в разработку бота:\n\
        https://github.com/NeuroBreaker/schedule_bot");

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
    dialogue.update(State::Start).await?;
    Ok(())
}

// Обработка любого текста, что не входит в команды
pub async fn message_handler(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    pool: PgPool,
) -> HandlerResult {
    let user_text = msg.text().unwrap_or("");

    match &*user_text.trim().to_lowercase() {
        "начать" => {
            setup_handler(bot, dialogue, msg, pool).await?;
        }
        "расписание" => {
            schedule_handler(bot, msg, dialogue, pool).await?;
        }
        _ => {
            if user_text.starts_with('/') {
                let unrecognized_command_text =
                    "Неизвестная команда, введите /help для просмотра доступных";
                bot.send_message(msg.chat.id, unrecognized_command_text)
                    .await?;
            } else {
                start_handler(bot, msg).await?;
            }
        }
    }

    Ok(())
}
