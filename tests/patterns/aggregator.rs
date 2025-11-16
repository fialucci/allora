use allora::{patterns::aggregator::Aggregator, route::Route, Exchange, Message, Payload};

#[cfg(feature = "async")]
fn run_route_async(route: &Route, exchange: &mut Exchange) -> allora::Result<()> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(route.run(exchange))
}
#[cfg(not(feature = "async"))]
fn run_route_async(route: &Route, exchange: &mut Exchange) -> allora::Result<()> {
    route.run(exchange)
}

#[test]
fn aggregates_three_messages() {
    let agg = Aggregator::new("corr", 3);
    let route = Route::new().add(agg).build();

    let mut last = None;
    for i in ["A", "B", "C"] {
        let mut exchange = Exchange::new(Message::from_text(i));
        exchange.in_msg.set_header("corr", "123");
        let res = route.run(&mut exchange);
        #[cfg(feature = "async")]
        let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
        assert!(res.is_ok());
        last = Some(exchange);
    }

    let out = last
        .unwrap()
        .out_msg
        .expect("Should have aggregated output");
    assert_eq!(out.body_text(), Some("ABC"));
}

#[test]
fn ignores_messages_without_correlation_header() {
    let agg = Aggregator::new("corr", 2);
    let route = Route::new().add(agg).build();
    for i in ["A", "B"] {
        let mut exchange = Exchange::new(Message::from_text(i));
        let res = route.run(&mut exchange);
        #[cfg(feature = "async")]
        let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
        assert!(res.is_ok());
        // Should never aggregate because header missing
        assert!(exchange.out_msg.is_none());
    }
}

#[test]
fn aggregates_multiple_batches_for_same_key() {
    let agg = Aggregator::new("corr", 2);
    let route = Route::new().add(agg).build();

    // First batch
    let mut ex1 = Exchange::new(Message::from_text("A"));
    ex1.in_msg.set_header("corr", "same");
    let res = route.run(&mut ex1);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    assert!(ex1.out_msg.is_none());

    let mut ex2 = Exchange::new(Message::from_text("B"));
    ex2.in_msg.set_header("corr", "same");
    let res = route.run(&mut ex2);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    assert_eq!(ex2.out_msg.as_ref().unwrap().body_text(), Some("AB"));

    // Second batch, store should have been cleared
    let mut ex3 = Exchange::new(Message::from_text("C"));
    ex3.in_msg.set_header("corr", "same");
    let res = route.run(&mut ex3);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    assert!(ex3.out_msg.is_none());

    let mut ex4 = Exchange::new(Message::from_text("D"));
    ex4.in_msg.set_header("corr", "same");
    let res = route.run(&mut ex4);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    assert_eq!(ex4.out_msg.as_ref().unwrap().body_text(), Some("CD"));
}

#[test]
fn non_text_messages_not_aggregated() {
    let agg = Aggregator::new("corr", 2);
    let route = Route::new().add(agg).build();

    let mut ex1 = Exchange::new(Message::new(Payload::Bytes(vec![1, 2, 3])));
    ex1.in_msg.set_header("corr", "mix");
    let res = route.run(&mut ex1);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    assert!(ex1.out_msg.is_none());

    let mut ex2 = Exchange::new(Message::from_text("A"));
    ex2.in_msg.set_header("corr", "mix");
    let res = route.run(&mut ex2);
    #[cfg(feature = "async")]
    let res = tokio::runtime::Runtime::new().unwrap().block_on(res);
    assert!(res.is_ok());
    // Should still be none because types differ
    assert!(ex2.out_msg.is_none());
}

#[test]
fn aggregator_clear_store_resets_bucket() {
    let controlling = Aggregator::new("corr", 2);
    let route = Route::new().add(controlling.clone()).build();
    let mut ex1 = Exchange::new(Message::from_text("A"));
    ex1.in_msg.set_header("corr", "x");
    run_route_async(&route, &mut ex1).unwrap();
    assert!(ex1.out_msg.is_none());
    controlling.clear_store();
    let mut ex2 = Exchange::new(Message::from_text("B"));
    ex2.in_msg.set_header("corr", "x");
    run_route_async(&route, &mut ex2).unwrap();
    assert!(
        ex2.out_msg.is_none(),
        "Clearing store should prevent completion on second message"
    );
}

#[test]
fn aggregator_mixed_non_text_group_does_not_emit() {
    let agg = Aggregator::new("corr", 2);
    let route = Route::new().add(agg).build();
    let mut ex1 = Exchange::new(Message::new(Payload::Bytes(vec![0, 1])));
    ex1.in_msg.set_header("corr", "m");
    run_route_async(&route, &mut ex1).unwrap();
    assert!(ex1.out_msg.is_none());
    let mut ex2 = Exchange::new(Message::from_text("T"));
    ex2.in_msg.set_header("corr", "m");
    run_route_async(&route, &mut ex2).unwrap();
    assert!(
        ex2.out_msg.is_none(),
        "Mixed payload types should not aggregate"
    );
}
