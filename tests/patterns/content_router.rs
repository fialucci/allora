use allora::{patterns::content_router::ContentRouter, Exchange, Message};

#[tokio::test]
async fn content_router_selects_branch() {
    let router = ContentRouter::new(|ex| {
        if ex.in_msg.body_text() == Some("A") {
            "a"
        } else {
            "b"
        }
    });
    let mut ex = Exchange::new(Message::from_text("A"));
    router.process_sync(&mut ex).unwrap();
    assert_eq!(ex.in_msg.body_text(), Some("A"));
}
