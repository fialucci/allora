use allora::channel::PollableChannel;
use allora::{Allora, Channel, Exchange, Message};

mod hello_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Allora::new().run()?;

    rt.channel_typed_or_panic::<allora::DirectChannel>("hello_channel")
        .send_async(Exchange::new(Message::from_text("World")))
        .await?;

    let ex = rt
        .channel_typed_or_panic::<allora::QueueChannel>("processed_channel")
        .try_receive_async()
        .await
        .expect("processed message");

    println!("Message: {}", ex.in_msg.body_text().unwrap_or("(empty)"));
    Ok(())
}
