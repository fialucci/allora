use allora::{Allora, Channel, Exchange, Message};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Allora::new().run()?;
    let direct = runtime.channel_typed_or_panic::<allora::DirectChannel>("hello_channel");
    // TODO: implement service to receive message and append " World!" to it
    direct.subscribe(|ex| {
        info!(body=?ex.in_msg.body_text(), channel="hello_channel", kind="direct", "Direct subscriber received message");
        Ok(())
    });

    direct
        .send_async(Exchange::new(Message::from_text("Hello")))
        .await?;
    Ok(())
}
