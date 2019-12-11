use futures::TryStreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut server = ws_async::Server::bind(&addr);

    while let Some(ref mut ws) = server.next_socket().await? {
        while let Some(ref mut frame) = ws.next_frame().await? {
            let payload = frame.stream().try_collect().await?;
            dbg!(payload);
        }
    }

    Ok(())
}
