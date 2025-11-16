use allora::{
    patterns::recipient_list::RecipientList, processor::ClosureProcessor, route::Route, Error,
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
fn recipient_list_applies_in_order() {
    let p1 = ClosureProcessor::new(|exchange| {
        exchange.in_msg.set_header("order", "1");
        Ok(())
    });
    let p2 = ClosureProcessor::new(|exchange| {
        exchange.in_msg.set_header("order", "2");
        Ok(())
    });
    let p3 = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("final"));
        Ok(())
    });
    let list = RecipientList::new().add(p1).add(p2).add(p3);
    let route = Route::new().add(list).build();
    let mut exchange = Exchange::new(Message::from_text("start"));
    run_route(&route, &mut exchange).unwrap();
    assert_eq!(exchange.in_msg.header("order"), Some("2")); // overwritten by second
    assert_eq!(exchange.out_msg.unwrap().body_text(), Some("final"));
}

#[test]
fn recipient_list_short_circuits_on_error() {
    let ok = ClosureProcessor::new(|exchange| {
        exchange.in_msg.set_header("seen", "true");
        Ok(())
    });
    let fail = ClosureProcessor::new(|_| Err(Error::processor("stop")));
    let after = ClosureProcessor::new(|exchange| {
        exchange.in_msg.set_header("after", "nope");
        Ok(())
    });
    let list = RecipientList::new().add(ok).add(fail).add(after);
    let route = Route::new().add(list).build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    let res = run_route(&route, &mut exchange);
    assert!(res.is_err());
    assert!(exchange.in_msg.header("seen").is_some());
    assert!(exchange.in_msg.header("after").is_none());
}

#[test]
fn recipient_list_noop_when_empty() {
    let list = RecipientList::new();
    let route = Route::new().add(list).build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    run_route(&route, &mut exchange).unwrap();
    assert!(exchange.out_msg.is_none());
    assert!(exchange.in_msg.header("order").is_none());
}

#[test]
fn recipient_list_last_mutation_wins_on_out_msg() {
    let p1 = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("first"));
        Ok(())
    });
    let p2 = ClosureProcessor::new(|exchange| {
        exchange.out_msg = Some(Message::from_text("second"));
        Ok(())
    });
    let list = RecipientList::new().add(p1).add(p2);
    let route = Route::new().add(list).build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    run_route(&route, &mut exchange).unwrap();
    assert_eq!(exchange.out_msg.unwrap().body_text(), Some("second"));
}
