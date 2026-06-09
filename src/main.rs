mod bot;
mod db;
mod handler_tree;
mod handlers;
mod utils;
mod types;

#[tokio::main]
async fn main() {
    let _ = bot::run().await;
}
