mod bot;
mod db;
mod handler_tree;
mod handlers;
mod inline_keyboards;
mod schedule;
mod utils;

#[tokio::main]
async fn main() {
    let _ = bot::run().await;
}
