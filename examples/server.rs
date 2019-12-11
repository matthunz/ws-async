#[tokio::main]
async fn main() -> hyper::Result<()> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut server = ws_async::Server::bind(&addr);

    while let Some(ws) = server.next_socket().await? {
        println!("New Connection!");
    }

    Ok(())
}
