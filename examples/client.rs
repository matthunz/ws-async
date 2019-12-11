#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ws_async::Client::new();

    let uri = "ws://127.0.0.1:8080".parse()?;
    let mut ws = client.connect(uri).await?;

    while let Some(frame) = ws.next_frame().await? {
        let mut payload = frame.into_payload();

        while let Some(bytes) = payload.next_bytes().await? {
            dbg!(bytes);
        }
    }

    Ok(())
}
