use allora::{
    patterns::content_router::ContentBasedRouter, processor::ClosureProcessor, route::Route, Error,
    Exchange, Message,
};

#[cfg(feature = "async")]
fn run_route(route: &Route, exchange: &mut Exchange) -> allora::Result<()> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(route.run(exchange))
}
#[cfg(not(feature = "async"))]
fn run_route(route: &Route, exchange: &mut Exchange) -> allora::Result<()> {
    route.run(exchange)
}

#[test]
fn content_router_routes_matching_value() {
    let p_hi = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("HI"));
        Ok(())
    });
    let p_bye = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("BYE"));
        Ok(())
    });
    let router = ContentBasedRouter::new("kind")
        .when("hi", Box::new(p_hi))
        .when("bye", Box::new(p_bye));
    let route = Route::new().add(router).build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    exchange.in_msg.set_header("kind", "hi");
    run_route(&route, &mut exchange).unwrap();
    assert_eq!(exchange.out_msg.unwrap().body_text(), Some("HI"));
}

#[test]
fn content_router_unmatched_value_errors() {
    let p_hi = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("HI"));
        Ok(())
    });
    let router = ContentBasedRouter::new("kind").when("hi", Box::new(p_hi));
    let route = Route::new().add(router).build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    exchange.in_msg.set_header("kind", "bye"); // no matching route
    let res = run_route(&route, &mut exchange);
    match res {
        Err(Error::Routing(msg)) => assert_eq!(msg, "no matching route"),
        _ => panic!("expected routing error"),
    }
    assert!(exchange.out_msg.is_none());
}

#[test]
fn content_router_missing_header_errors() {
    let p_hi = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("HI"));
        Ok(())
    });
    let router = ContentBasedRouter::new("kind").when("hi", Box::new(p_hi));
    let route = Route::new().add(router).build();
    let mut exchange = Exchange::new(Message::from_text("payload")); // no header set
    let res = run_route(&route, &mut exchange);
    assert!(matches!(res, Err(Error::Routing(_))));
}

#[test]
fn content_router_only_one_processor_executes() {
    let p_hi = ClosureProcessor::new(|exchange| {
        exchange.in_msg.set_header("which", "hi");
        Ok(())
    });
    let p_bye = ClosureProcessor::new(|exchange| {
        exchange.in_msg.set_header("which", "bye");
        Ok(())
    });
    let router = ContentBasedRouter::new("kind")
        .when("hi", Box::new(p_hi))
        .when("bye", Box::new(p_bye));
    let route = Route::new().add(router).build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    exchange.in_msg.set_header("kind", "bye");
    run_route(&route, &mut exchange).unwrap();
    assert_eq!(exchange.in_msg.header("which"), Some("bye"));
}

#[test]
#[should_panic(expected = "header name must not be empty")]
fn content_router_panics_on_empty_header_name() {
    let _r = ContentBasedRouter::new("");
}

#[test]
#[should_panic(expected = "route value must not be empty")]
fn content_router_panics_on_empty_value() {
    let router = ContentBasedRouter::new("kind");
    let _router = router.when("", Box::new(ClosureProcessor::new(|_| Ok(()))));
}
