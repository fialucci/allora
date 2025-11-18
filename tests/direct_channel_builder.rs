use allora::{Channel, DirectChannel, Exchange, Message};

#[tokio::test]
async fn direct_channel_explicit_id_and_dispatch() {
    let dc = DirectChannel::with_id("builder-direct");
    assert_eq!(dc.id(), "builder-direct");
    assert_eq!(Channel::kind(&dc), "direct");
    let hits = std::sync::Arc::new(std::sync::Mutex::new(0));
    let hits_cl = hits.clone();
    dc.subscribe(move |_ex| {
        *hits_cl.lock().unwrap() += 1;
        Ok(())
    });
    dc.send(Exchange::new(Message::from_text("ping")))
        .await
        .unwrap();
    assert_eq!(*hits.lock().unwrap(), 1);
}
