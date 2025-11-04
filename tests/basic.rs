use allora::{route::Route, Exchange, Message};

/// Smoke test: an empty Route should succeed and leave the Exchange unchanged.
#[test]
fn route_empty_executes_ok() {
    let mut ex = Exchange::new(Message::from_text("hello"));
    let route = Route::new().build();
    let res = route.run(&mut ex);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    assert!(ex.out_msg.is_none());
    assert_eq!(ex.in_msg.body_text(), Some("hello"));
}
