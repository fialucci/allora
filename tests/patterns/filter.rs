use allora::{
    patterns::filter::Filter, processor::ClosureProcessor, route::Route, Error, Exchange, Message,
};

#[cfg(feature = "async")]
fn run_route(route: &Route, ex: &mut Exchange) -> allora::Result<()> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(route.run(ex))
}
#[cfg(not(feature = "async"))]
fn run_route(route: &Route, ex: &mut Exchange) -> allora::Result<()> {
    route.run(ex)
}

// Test custom error message path
#[test]
fn filter_custom_error_message() {
    let route = Route::new()
        .add(Filter::with_error(
            |ex| ex.in_msg.body_text() == Some("ACCEPT"),
            "custom reject",
        ))
        .build();
    let mut ex = Exchange::new(Message::from_text("REJECT"));
    let res = run_route(&route, &mut ex);
    match res {
        Err(Error::Routing(msg)) => assert_eq!(msg, "custom reject"),
        _ => panic!("expected Routing error"),
    }
}

// Test that filter allows downstream processor when accepted
#[test]
fn filter_allows_downstream_processor() {
    let filter = Filter::new(|ex| ex.in_msg.body_text() == Some("go"));
    let proc = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("ok"));
        Ok(())
    });
    let route = Route::new().add(filter).add(proc).build();
    let mut ex = Exchange::new(Message::from_text("go"));
    run_route(&route, &mut ex).unwrap();
    assert_eq!(ex.out_msg.unwrap().body_text(), Some("ok"));
}

// Test that filter stops route before downstream processor when rejected
#[test]
fn filter_blocks_downstream_processor_on_reject() {
    let filter = Filter::new(|ex| ex.in_msg.body_text() == Some("go"));
    let proc = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("should_not"));
        Ok(())
    });
    let route = Route::new().add(filter).add(proc).build();
    let mut ex = Exchange::new(Message::from_text("stop"));
    let res = run_route(&route, &mut ex);
    assert!(res.is_err());
    assert!(
        ex.out_msg.is_none(),
        "out_msg should remain None when filter rejects"
    );
}

// Chained filters: first passes, second fails
#[test]
fn chained_filters_second_rejects() {
    let f1 = Filter::new(|ex| ex.in_msg.body_text().is_some());
    let f2 = Filter::with_error(
        |ex| ex.in_msg.body_text() == Some("expected"),
        "second failed",
    );
    let proc = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("done"));
        Ok(())
    });
    let route = Route::new().add(f1).add(f2).add(proc).build();
    let mut ex = Exchange::new(Message::from_text("unexpected"));
    let res = run_route(&route, &mut ex);
    match res {
        Err(Error::Routing(msg)) => assert_eq!(msg, "second failed"),
        _ => panic!("expected second filter rejection"),
    }
    assert!(ex.out_msg.is_none());
}

// Ensure filter does not mutate exchange (apart from route error) by comparing headers before/after
#[test]
fn filter_does_not_mutate_exchange() {
    let filter = Filter::new(|ex| ex.in_msg.body_text() == Some("ok"));
    let mut ex = Exchange::new(Message::from_text("fail"));
    let original_headers = ex.in_msg.headers.clone();
    let route = Route::new().add(filter).build();
    let _ = run_route(&route, &mut ex); // ignore result
    assert_eq!(
        ex.in_msg.headers, original_headers,
        "Filter must not mutate headers"
    );
}
