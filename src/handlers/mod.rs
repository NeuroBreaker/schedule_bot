use std::error::Error;

pub mod default;
pub mod get_schedule;
pub mod setup;

pub use default::*;
pub use get_schedule::*;
pub use setup::*;

type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

#[derive(Default, Clone, Debug)]
pub struct User {
    institute: String,
    course: String,
    group: String,
}
