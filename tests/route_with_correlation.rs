use allora::{route::Route, Exchange, Message};

#[cfg(feature = "async")]
#[tokio::test]
async fn route_with_correlation_adds_id() {
    let route = Route::with_correlation(None).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    assert!(exchange.in_msg.header("correlation_id").is_none());
    route.run(&mut exchange).await.unwrap();
    assert!(exchange.in_msg.header("correlation_id").is_some());
}

#[cfg(not(feature = "async"))]
#[test]
fn route_with_correlation_adds_id() {
    let route = Route::with_correlation(None).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    assert!(exchange.in_msg.header("correlation_id").is_none());
    route.run(&mut exchange).unwrap();
    assert!(exchange.in_msg.header("correlation_id").is_some());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn route_with_correlation_mirrors() {
    let route = Route::with_correlation(Some("corr")).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    route.run(&mut exchange).await.unwrap();
    let cid = exchange.in_msg.header("correlation_id").unwrap();
    assert_eq!(exchange.in_msg.header("corr"), Some(cid));
}

#[cfg(not(feature = "async"))]
#[test]
fn route_with_correlation_mirrors() {
    let route = Route::with_correlation(Some("corr")).build();
    let mut exchange = Exchange::new(Message::from_text("hello"));
    route.run(&mut exchange).unwrap();
    let cid = exchange.in_msg.header("correlation_id").unwrap();
    assert_eq!(exchange.in_msg.header("corr"), Some(cid));
}
