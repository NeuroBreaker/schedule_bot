use std::error::Error;

use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};
use crate::handler_tree::handler_tree;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "Commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display this text.")]
    Start,
    //#[command(description = "cancel the current operation")]
    //Cancel,
    #[command(description = "choose your institute and group")]
    Institute,
    #[command(description = "get your schedule")]
    Schedule,
    #[command(description = "drop the dice")]
    Dice,
}

#[derive(Default, Clone, Debug)]
pub enum State {
    #[default]
    Start,
}

pub async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    pretty_env_logger::init();
    log::info!("Starting command bot...");
    dotenv::dotenv().ok();

    let bot = Bot::from_env();

    let storage = InMemStorage::<State>::new();

    Dispatcher::builder(bot, handler_tree())
        .dependencies(dptree::deps![storage])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
