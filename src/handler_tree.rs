use crate::{Command, State, handlers};
use dptree::case;
use std::error::Error;
use teloxide::{prelude::*, dispatching::{dialogue::{InMemStorage, Dialogue}, UpdateHandler}};

pub fn handler_tree() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    dptree::entry()
        .enter_dialogue::<Update, InMemStorage<State>, State>()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                    .branch(case![Command::Help].endpoint(handlers::help))
                .branch(case![State::Start].endpoint(handlers::start))
        )
        //.branch(
        //    Update::filter_callback_query()
        //        .branch(case![State::Receive].endpoint(handlers::receive))
        //)
}
