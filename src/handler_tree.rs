use crate::{Command, State, handlers::*};
use dptree::case;
use std::error::Error;
use teloxide::{prelude::*, dispatching::{dialogue::{InMemStorage, Dialogue}, UpdateHandler}};

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

pub fn handler_tree() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    let command_handler = Update::filter_message()
        .filter_command::<Command>()
        .branch(case![Command::Help].endpoint(start_handler))
        .branch(case![Command::Start].endpoint(start_handler))
        .branch(case![Command::Dice].endpoint(dice_handler));

    let message_handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(case![State::Start].endpoint(message_handler))
        .branch(case![State::Dice].endpoint(dice_handler));

    //let callback_handler = Update::filter_callback_query()
    //    .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
    //    .branch(case![State::Receive].endpoint());

    dptree::entry()
        .branch(command_handler)
        .branch(message_handler)
        //.branch(callback_handler)
}
