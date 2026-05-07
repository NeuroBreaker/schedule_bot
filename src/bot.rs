use std::{error::Error, process};

use crate::{db::init_db, handler_tree::handler_tree};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Commands are supported:")]
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
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env or environment");

    let pool = init_db(&db_url).await?;

    let storage = InMemStorage::<State>::new();

    Dispatcher::builder(bot, handler_tree())
        .dependencies(dptree::deps![
            pool,
            storage
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
