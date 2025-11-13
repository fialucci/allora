use allora::{Allora, Channel, Exchange, Message, OutboundQueue};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Allora::new().run()?;

    if let Some(input) = runtime.channel_by_id("hello_channel") {
        input
            .send_async(Exchange::new(Message::from_text("Hello World!")))
            .await?;
        if let Some(ex) = input.try_receive_async().await {
            info!(body = ?ex.in_msg.body_text(), channel = "hello_channel", "Received message");
        }
    }
    Ok(())
}
