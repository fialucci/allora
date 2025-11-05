use allora::{
    patterns::correlation_initializer::CorrelationInitializer, route::Route, Exchange, Message,
};

#[cfg(feature = "async")]
#[tokio::test]
async fn adds_correlation_id_when_missing() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut ex = Exchange::new(Message::from_text("hello"));
    assert!(ex.in_msg.header("correlation_id").is_none());
    route.run(&mut ex).await.unwrap();
    assert!(ex.in_msg.header("correlation_id").is_some());
}
#[cfg(not(feature = "async"))]
#[test]
fn adds_correlation_id_when_missing() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut ex = Exchange::new(Message::from_text("hello"));
    assert!(ex.in_msg.header("correlation_id").is_none());
    route.run(&mut ex).unwrap();
    assert!(ex.in_msg.header("correlation_id").is_some());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn preserves_existing_correlation_id() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut ex = Exchange::new(Message::from_text("hello"));
    ex.in_msg.set_header("correlation_id", "existing");
    route.run(&mut ex).await.unwrap();
    assert_eq!(ex.in_msg.header("correlation_id"), Some("existing"));
}
#[cfg(not(feature = "async"))]
#[test]
fn preserves_existing_correlation_id() {
    let route = Route::new().add(CorrelationInitializer::default()).build();
    let mut ex = Exchange::new(Message::from_text("hello"));
    ex.in_msg.set_header("correlation_id", "existing");
    route.run(&mut ex).unwrap();
    assert_eq!(ex.in_msg.header("correlation_id"), Some("existing"));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn mirrors_header_when_configured() {
    let route = Route::new()
        .add(CorrelationInitializer::with_mirror("corr"))
        .build();
    let mut ex = Exchange::new(Message::from_text("hello"));
    route.run(&mut ex).await.unwrap();
    let cid = ex.in_msg.header("correlation_id").unwrap();
    assert_eq!(ex.in_msg.header("corr"), Some(cid));
}
#[cfg(not(feature = "async"))]
#[test]
fn mirrors_header_when_configured() {
    let route = Route::new()
        .add(CorrelationInitializer::with_mirror("corr"))
        .build();
    let mut ex = Exchange::new(Message::from_text("hello"));
    route.run(&mut ex).unwrap();
    let cid = ex.in_msg.header("correlation_id").unwrap();
    assert_eq!(ex.in_msg.header("corr"), Some(cid));
}
