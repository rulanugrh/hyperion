use be_chat_rust::server::Server;

#[tokio::main]
async fn main() {
    env_logger::init();
    let server = Server::new(4000);
    server.run().await;
}
