use allora::{
    patterns::correlation_initializer::CorrelationInitializer, route::Route, Exchange, Message,
};

#[cfg(feature = "async")]
#[tokio::test]
async fn adds_correlation_id_when_missing() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    assert!(exchange.in_msg.header("correlation_id").is_none());
    route.run(&mut exchange).await.unwrap();
    assert!(exchange.in_msg.header("correlation_id").is_some());
}
#[cfg(not(feature = "async"))]
#[test]
fn adds_correlation_id_when_missing() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    assert!(exchange.in_msg.header("correlation_id").is_none());
    route.run(&mut exchange).unwrap();
    assert!(exchange.in_msg.header("correlation_id").is_some());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn preserves_existing_correlation_id() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    exchange.in_msg.set_header("correlation_id", "existing");
    route.run(&mut exchange).await.unwrap();
    assert_eq!(exchange.in_msg.header("correlation_id"), Some("existing"));
}
#[cfg(not(feature = "async"))]
#[test]
fn preserves_existing_correlation_id() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    exchange.in_msg.set_header("correlation_id", "existing");
    route.run(&mut exchange).unwrap();
    assert_eq!(exchange.in_msg.header("correlation_id"), Some("existing"));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn mirrors_header_when_configured() {
    let route = Route::new()
        .add(CorrelationInitializer::with_mirror("corr"))
        .build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    route.run(&mut exchange).await.unwrap();
    let cid = exchange.in_msg.header("correlation_id").unwrap();
    assert_eq!(exchange.in_msg.header("corr"), Some(cid));
}
#[cfg(not(feature = "async"))]
#[test]
fn mirrors_header_when_configured() {
    let route = Route::new()
        .add(CorrelationInitializer::with_mirror("corr"))
        .build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    route.run(&mut exchange).unwrap();
    let cid = exchange.in_msg.header("correlation_id").unwrap();
    assert_eq!(exchange.in_msg.header("corr"), Some(cid));
}
