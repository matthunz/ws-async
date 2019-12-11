#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let mut server = ws_async::Server::bind(&addr);

    while let Some(ref mut ws) = server.next_socket().await? {
        while let Some(ref mut frame) = ws.next_frame().await? {
            while let Some(bytes) = frame.next_bytes().await? {
                dbg!(bytes);
            }
        }
    }

    Ok(())
}
