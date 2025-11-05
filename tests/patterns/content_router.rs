use allora::{
    patterns::content_router::ContentBasedRouter, processor::ClosureProcessor, route::Route,
    Error, Exchange, Message,
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

#[test]
fn content_router_routes_matching_value() {
    let p_hi = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("HI"));
        Ok(())
    });
    let p_bye = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("BYE"));
        Ok(())
    });
    let router = ContentBasedRouter::new("kind")
        .when("hi", Box::new(p_hi))
        .when("bye", Box::new(p_bye));
    let route = Route::new().add(router).build();
    let mut ex = Exchange::new(Message::from_text("payload"));
    ex.in_msg.set_header("kind", "hi");
    run_route(&route, &mut ex).unwrap();
    assert_eq!(ex.out_msg.unwrap().body_text(), Some("HI"));
}

#[test]
fn content_router_unmatched_value_errors() {
    let p_hi = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("HI"));
        Ok(())
    });
    let router = ContentBasedRouter::new("kind").when("hi", Box::new(p_hi));
    let route = Route::new().add(router).build();
    let mut ex = Exchange::new(Message::from_text("payload"));
    ex.in_msg.set_header("kind", "bye"); // no matching route
    let res = run_route(&route, &mut ex);
    match res {
        Err(Error::Routing(msg)) => assert_eq!(msg, "no matching route"),
        _ => panic!("expected routing error"),
    }
    assert!(ex.out_msg.is_none());
}

#[test]
fn content_router_missing_header_errors() {
    let p_hi = ClosureProcessor::new(|ex| {
        ex.out_msg = Some(Message::from_text("HI"));
        Ok(())
    });
    let router = ContentBasedRouter::new("kind").when("hi", Box::new(p_hi));
    let route = Route::new().add(router).build();
    let mut ex = Exchange::new(Message::from_text("payload")); // no header set
    let res = run_route(&route, &mut ex);
    assert!(matches!(res, Err(Error::Routing(_))));
}

#[test]
fn content_router_only_one_processor_executes() {
    let p_hi = ClosureProcessor::new(|ex| {
        ex.in_msg.set_header("which", "hi");
        Ok(())
    });
    let p_bye = ClosureProcessor::new(|ex| {
        ex.in_msg.set_header("which", "bye");
        Ok(())
    });
    let router = ContentBasedRouter::new("kind")
        .when("hi", Box::new(p_hi))
        .when("bye", Box::new(p_bye));
    let route = Route::new().add(router).build();
    let mut ex = Exchange::new(Message::from_text("payload"));
    ex.in_msg.set_header("kind", "bye");
    run_route(&route, &mut ex).unwrap();
    assert_eq!(ex.in_msg.header("which"), Some("bye"));
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
