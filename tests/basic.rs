use allora_core::{route::Route, Exchange, Message};

/// Smoke test: an empty Route should succeed and leave the Exchange unchanged.
#[tokio::test]
async fn route_empty_executes_ok() {
    let mut exchange = Exchange::new(Message::from_text("hello"));
    let route = Route::new().build();
    let res = route.run(&mut exchange).await;
    assert!(res.is_ok());
    assert!(exchange.out_msg.is_none());
    assert_eq!(exchange.in_msg.body_text(), Some("hello"));
}
