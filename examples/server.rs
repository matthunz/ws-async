use tokio::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8080".parse()?;
    let mut server = ws_async::Server::bind(&addr);

    while let Some(ref mut ws) = server.next_socket().await? {
        while let Some(frame) = ws.next_frame().await? {
            let mut payload = frame.into_payload();

            while let Some(res) = payload.next().await {
                let bytes = res?;
                let s = std::str::from_utf8(&bytes)?;
                dbg!(s);
            }
        }
    }

    Ok(())
}
