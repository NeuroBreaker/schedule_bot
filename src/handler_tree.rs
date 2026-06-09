use crate::{
    bot::{Command, State},
    handlers::*,
};
use dptree::case;
use std::error::Error;
use teloxide::{
    dispatching::{
        UpdateHandler,
        dialogue::{Dialogue, InMemStorage},
    },
    prelude::*,
};

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

pub fn handler_tree() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    let command_handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .filter_command::<Command>()
        .branch(case![Command::Help].endpoint(start_handler))
        .branch(case![Command::Start].endpoint(start_handler))
        .branch(case![Command::Cancel].endpoint(cancel_handler))
        .branch(case![Command::Setup].endpoint(setup_handler))
        .branch(case![Command::Schedule].endpoint(schedule_handler));

    let message_handler = Update::filter_message()
        .enter_dialogue::<Message, InMemStorage<State>, State>()
        .branch(case![State::Start].endpoint(message_handler));

    let callback_handler = Update::filter_callback_query()
        .enter_dialogue::<CallbackQuery, InMemStorage<State>, State>()
        .branch(case![State::AwaitingInstitute].endpoint(institute_callback_handler))
        .branch(case![State::AwaitingCourse(user)].endpoint(course_callback_handler))
        .branch(case![State::AwaitingGroup(user)].endpoint(group_callback_handler))
        .branch(case![State::WeekSchedule(schedule)].endpoint(week_schedule_callback_handler))
        .branch(case![State::DaySchedule(schedule)].endpoint(day_schedule_callback_handler));

    dptree::entry()
        .branch(command_handler)
        .branch(callback_handler)
        .branch(message_handler)
}
