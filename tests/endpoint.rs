use allora::endpoint::Endpoint;
use allora::{endpoint::InMemoryEndpoint, Exchange, Message};
// bring trait methods into scope

#[cfg(not(feature = "async"))]
#[test]
fn endpoint_fifo_sync() {
    let ep = InMemoryEndpoint::new();
    ep.send(Exchange::new(Message::from_text("first"))).unwrap();
    ep.send(Exchange::new(Message::from_text("second")))
        .unwrap();
    ep.send(Exchange::new(Message::from_text("third"))).unwrap();
    assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("first"));
    assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("second"));
    assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("third"));
    assert!(ep.try_receive().is_none());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn endpoint_fifo_async() {
    let ep = InMemoryEndpoint::new();
    ep.send_async(Exchange::new(Message::from_text("A")))
        .await
        .unwrap();
    ep.send_async(Exchange::new(Message::from_text("B")))
        .await
        .unwrap();
    assert_eq!(
        ep.try_receive_async().await.unwrap().in_msg.body_text(),
        Some("A")
    );
    assert_eq!(
        ep.try_receive_async().await.unwrap().in_msg.body_text(),
        Some("B")
    );
    assert!(ep.try_receive_async().await.is_none());
}
