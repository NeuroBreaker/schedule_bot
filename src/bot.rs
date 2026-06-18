use std::{error::Error, process};

use crate::{db::init_db, handler_tree::handler_tree, handlers::User, types::schedule::Schedule};
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Commands are supported:")]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display this text.")]
    Start,
    #[command(description = "cancel command")]
    Cancel,
    #[command(description = "choose your institute and group")]
    Setup,
    #[command(description = "get your schedule")]
    Schedule,
}

#[derive(Default, Clone, Debug)]
pub enum State {
    #[default]
    Start,
    AwaitingInstitute,
    AwaitingCourse(User),
    AwaitingGroup(User),
    WeekSchedule(Schedule),
    DaySchedule(Schedule),
}

pub async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    let db_url = if let Ok(val) = std::env::var("DATABASE_URL") {
        val
    } else {
        log::error!("DATABASE_URL must be set in .env or environment");
        process::exit(1);
    };

    let pool = match init_db(&db_url).await {
        Ok(val) => {
            val
        }
        Err(err) => {
            let err_msg = err.to_string();
            log::error!("Database error: {err_msg}");
            process::exit(1);
        }
    };

    let storage = InMemStorage::<State>::new();

    Dispatcher::builder(bot, handler_tree())
        .dependencies(dptree::deps![storage, pool])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
