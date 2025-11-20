use allora::{patterns::filter::Filter, Exchange, Message};

#[tokio::test]
async fn filter_predicate() {
    let filt = Filter::new(|ex| ex.in_msg.body_text() == Some("pass"));
    let mut ex = Exchange::new(Message::from_text("pass"));
    filt.process(&mut ex).await.unwrap();
    assert_eq!(ex.in_msg.body_text(), Some("pass"));
}

// Test custom error message path
#[test]
fn filter_custom_error_message() {
    let route = Route::new()
        .add(Filter::with_error(
            |exchange| exchange.in_msg.body_text() == Some("ACCEPT"),
            "custom reject",
        ))
        .build();
    let mut exchange = Exchange::new(Message::from_text("REJECT"));
    let res = run_route(&route, &mut exchange);
    match res {
        Err(Error::Routing(msg)) => assert_eq!(msg, "custom reject"),
        _ => panic!("expected Routing error"),
    }
}

// Test that filter allows downstream processor when accepted
#[test]
fn filter_allows_downstream_processor() {
    let filter = Filter::new(|exchange| exchange.in_msg.body_text() == Some("go"));
    let proc = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("ok"));
        Ok(())
    });
    let route = Route::new().add(filter).add(proc).build();
    let mut exchange = Exchange::new(Message::from_text("go"));
    run_route(&route, &mut exchange).unwrap();
    assert_eq!(exchange.out_msg.unwrap().body_text(), Some("ok"));
}

// Test that filter stops route before downstream processor when rejected
#[test]
fn filter_blocks_downstream_processor_on_reject() {
    let filter = Filter::new(|exchange| exchange.in_msg.body_text() == Some("go"));
    let proc = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("should_not"));
        Ok(())
    });
    let route = Route::new().add(filter).add(proc).build();
    let mut exchange = Exchange::new(Message::from_text("stop"));
    let res = run_route(&route, &mut exchange);
    assert!(res.is_err());
    assert!(
        exchange.out_msg.is_none(),
        "out_msg should remain None when filter rejects"
    );
}

// Chained filters: first passes, second fails
#[test]
fn chained_filters_second_rejects() {
    let f1 = Filter::new(|exchange| exchange.in_msg.body_text().is_some());
    let f2 = Filter::with_error(
        |exchange| exchange.in_msg.body_text() == Some("expected"),
        "second failed",
    );
    let proc = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("done"));
        Ok(())
    });
    let route = Route::new().add(f1).add(f2).add(proc).build();
    let mut exchange = Exchange::new(Message::from_text("unexpected"));
    let res = run_route(&route, &mut exchange);
    match res {
        Err(Error::Routing(msg)) => assert_eq!(msg, "second failed"),
        _ => panic!("expected second filter rejection"),
    }
    assert!(exchange.out_msg.is_none());
}

// Ensure filter does not mutate exchange (apart from route error) by comparing headers before/after
#[test]
fn filter_does_not_mutate_exchange() {
    let filter = Filter::new(|exchange| exchange.in_msg.body_text() == Some("ok"));
    let mut exchange = Exchange::new(Message::from_text("fail"));
    let original_headers = exchange.in_msg.headers.clone();
    let route = Route::new().add(filter).build();
    let _ = run_route(&route, &mut exchange); // ignore result
    assert_eq!(
        exchange.in_msg.headers, original_headers,
        "Filter must not mutate headers"
    );
}
