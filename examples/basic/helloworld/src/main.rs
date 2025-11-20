use allora::{
    Channel, DirectChannel, Exchange, Message, PollableChannel, QueueChannel, Result, Runtime,
};

mod hello_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new().run()?;

    let input_channel = rt.channel::<DirectChannel>("input_channel");
    let output_channel = rt.channel::<QueueChannel>("output_channel");

    input_channel
        .send(Exchange::new(Message::from_text("World")))
        .await?;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let ex = output_channel
        .try_receive()
        .await
        .expect("processed message");

    println!("Message: {}", ex.in_msg.body_text().unwrap_or("(empty)"));
    Ok(())
}
