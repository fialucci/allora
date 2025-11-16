use allora::{Allora, Channel, Exchange, Message};

mod hello_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Allora::new().run()?;
    let direct = runtime.channel_typed_or_panic::<allora::DirectChannel>("hello_channel");

    direct
        .send_async(Exchange::new(Message::from_text("Hello")))
        .await?;
    Ok(())
}
