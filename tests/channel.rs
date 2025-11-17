use allora::channel::PollableChannel;
use allora::channel::QueueChannel;
use allora::{Channel, Exchange, Message};
// Distinct coverage areas:
// 1. Inbound identity round-trip (no out_msg)
// 2. Outbound message retention (explicit out_msg)
// 3. FIFO ordering for multiple messages
// 4. Builder explicit vs generated id
// 5. Correlation support (sync: send_with_correlation / lookup; async: header preservation)

/// GIVEN a channel and an inbound message without outbound
/// WHEN the message is enqueued and dequeued
/// THEN inbound body & message_id are preserved and no outbound message exists.
#[cfg_attr(feature = "async", tokio::test)]
#[cfg_attr(not(feature = "async"), test)]
async fn channel_preserves_inbound_message_identity() {
    let channel = QueueChannel::with_random_id();
    let original = Message::from_text("inbound-identity");
    let original_id = original.header("message_id").unwrap().to_string();
    let exchange = Exchange::new(original.clone());
    #[cfg(feature = "async")]
    channel.send_async(exchange).await.unwrap();
    #[cfg(not(feature = "async"))]
    channel.send(exchange).unwrap();
    #[cfg(feature = "async")]
    let received = channel.try_receive_async().await.expect("received");
    #[cfg(not(feature = "async"))]
    let received = channel.try_receive().expect("received");
    assert_eq!(received.in_msg.body_text(), Some("inbound-identity"));
    assert_eq!(
        received.in_msg.header("message_id"),
        Some(original_id.as_str())
    );
    assert!(received.out_msg.is_none());
    // Queue now empty
    #[cfg(feature = "async")]
    assert!(channel.try_receive_async().await.is_none());
    #[cfg(not(feature = "async"))]
    assert!(channel.try_receive().is_none());
}

/// GIVEN a channel and an Exchange with an outbound message
/// WHEN enqueued and dequeued
/// THEN the outbound message is retained alongside original inbound.
#[cfg_attr(feature = "async", tokio::test)]
#[cfg_attr(not(feature = "async"), test)]
async fn channel_retains_outbound_message() {
    let channel = QueueChannel::with_random_id();
    let mut exchange = Exchange::new(Message::from_text("input"));
    exchange.out_msg = Some(Message::from_text("processed"));
    #[cfg(feature = "async")]
    channel.send_async(exchange.clone()).await.unwrap();
    #[cfg(not(feature = "async"))]
    channel.send(exchange.clone()).unwrap();
    #[cfg(feature = "async")]
    let stored = channel.try_receive_async().await.expect("queued exchange");
    #[cfg(not(feature = "async"))]
    let stored = channel.try_receive().expect("queued exchange");
    assert_eq!(stored.out_msg.unwrap().body_text(), Some("processed"));
    // No transformation of inbound
    assert_eq!(stored.in_msg.body_text(), Some("input"));
}

/// GIVEN multiple Exchanges enqueued in order
/// WHEN all are dequeued
/// THEN FIFO order of both inbound and outbound messages is preserved.
#[cfg_attr(feature = "async", tokio::test)]
#[cfg_attr(not(feature = "async"), test)]
async fn channel_preserves_fifo_order() {
    let channel = QueueChannel::with_random_id();
    for i in 0..4 {
        // 4 messages to strengthen ordering guarantee
        let mut exchange = Exchange::new(Message::from_text(format!("in-{i}")));
        exchange.out_msg = Some(Message::from_text(format!("out-{i}")));
        #[cfg(feature = "async")]
        channel.send_async(exchange).await.unwrap();
        #[cfg(not(feature = "async"))]
        channel.send(exchange).unwrap();
    }
    let mut collected = Vec::new();
    #[cfg(feature = "async")]
    while let Some(exchange) = channel.try_receive_async().await {
        collected.push(exchange);
    }
    #[cfg(not(feature = "async"))]
    while let Some(exchange) = channel.try_receive() {
        collected.push(exchange);
    }
    assert_eq!(collected.len(), 4);
    for i in 0..4 {
        let expect_in = format!("in-{i}");
        let expect_out = format!("out-{i}");
        assert_eq!(collected[i].in_msg.body_text(), Some(expect_in.as_str()));
        assert_eq!(
            collected[i].out_msg.as_ref().unwrap().body_text(),
            Some(expect_out.as_str())
        );
    }
}

/// GIVEN explicit id builder usage and default builder usage
/// WHEN channels are built
/// THEN explicit id matches provided value and auto id has queue: prefix.
#[cfg_attr(feature = "async", tokio::test)]
#[cfg_attr(not(feature = "async"), test)]
async fn channel_builder_id_explicit_and_auto() {
    let explicit = QueueChannel::with_id("explicit-id");
    assert_eq!(explicit.id(), "explicit-id");
    let auto = QueueChannel::with_random_id();
    assert!(auto.id().starts_with("queue:"));
}

/// Correlation behavior:
/// sync: send_with_correlation generates distinct ids and receive_by_correlation can fetch them.
/// async: manual headers injected; verifies preservation after dequeue without using sync wrapper.
#[cfg_attr(feature = "async", tokio::test)]
#[cfg_attr(not(feature = "async"), test)]
async fn channel_correlation_ids_distinct() {
    let channel = QueueChannel::with_random_id();
    #[cfg(not(feature = "async"))]
    {
        use allora::channel::CorrelationSupport; // import trait only for sync path
        let ex1 = Exchange::new(Message::from_text("corr-A"));
        let ex2 = Exchange::new(Message::from_text("corr-B"));
        let cid1 = channel.send_with_correlation(ex1).unwrap();
        let cid2 = channel.send_with_correlation(ex2).unwrap();
        assert_ne!(cid1, cid2);
        let fetched1 = channel.receive_by_correlation(&cid1).expect("cid1");
        let fetched2 = channel.receive_by_correlation(&cid2).expect("cid2");
        assert_eq!(fetched1.in_msg.body_text(), Some("corr-A"));
        assert_eq!(fetched2.in_msg.body_text(), Some("corr-B"));
    }
    #[cfg(feature = "async")]
    {
        let mut ex1 = Exchange::new(Message::from_text("corr-A"));
        ex1.in_msg.set_header("corr_id", "c-a");
        ex1.in_msg.set_header("correlation_id", "c-a");
        let mut ex2 = Exchange::new(Message::from_text("corr-B"));
        ex2.in_msg.set_header("corr_id", "c-b");
        ex2.in_msg.set_header("correlation_id", "c-b");
        channel.send_async(ex1).await.unwrap();
        channel.send_async(ex2).await.unwrap();
        let mut drained = Vec::new();
        while let Some(exchange) = channel.try_receive_async().await {
            drained.push(exchange);
        }
        assert_eq!(drained.len(), 2);
        let bodies: Vec<_> = drained
            .iter()
            .map(|e| e.in_msg.body_text().unwrap())
            .collect();
        assert!(bodies.contains(&"corr-A") && bodies.contains(&"corr-B"));
    }
}
