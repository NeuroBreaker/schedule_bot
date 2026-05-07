mod bot;
mod db;
mod handler_tree;
mod handlers;
mod inline_keyboards;

#[tokio::main]
async fn main() {
    let _ = bot::run().await;
}
