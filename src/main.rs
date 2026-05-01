use teloxide::{dispatching::dialogue::InMemStorage, prelude::*, utils::command::BotCommands};

mod handler_tree;
mod handlers;

use handler_tree::handler_tree;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
pub enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "display this text.")]
    Start,
    #[command(description = "cancel the current operation")]
    Cancel,
    #[command(description = "drop the dice")]
    Dice,
}

#[derive(Default, Clone, Debug)]
pub enum State {
    #[default]
    Start,
    Dice,
}

#[tokio::main]
async fn main() {
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
}
