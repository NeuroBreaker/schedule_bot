use crate::{Command, handlers};
use dptree::case;
use teloxide::{dispatching::UpdateHandler, prelude::*};

pub fn handler_tree() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    dptree::entry()
        .branch(Update::filter_message().branch(case![Command::Help].endpoint(handlers::start)))
}
