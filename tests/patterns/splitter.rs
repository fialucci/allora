use allora::{message::Payload, patterns::splitter::Splitter, route::Route, Exchange, Message};

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
fn splitter_empty_vector_leaves_out_msg_none() {
    let splitter = Splitter::new(|_ex| Vec::new());
    let route = Route::new().add(splitter).build();
    let mut ex = Exchange::new(Message::from_text("irrelevant"));
    run_route(&route, &mut ex).unwrap();
    assert!(ex.out_msg.is_none());
}

#[test]
fn splitter_first_message_selected() {
    let splitter = Splitter::new(|ex| {
        ex.in_msg
            .body_text()
            .map(|t| t.split(',').map(|w| Message::from_text(w.trim())).collect())
            .unwrap_or_default()
    });
    let route = Route::new().add(splitter).build();
    let mut ex = Exchange::new(Message::from_text("A,B,C"));
    run_route(&route, &mut ex).unwrap();
    assert_eq!(ex.out_msg.unwrap().body_text(), Some("A"));
}

#[test]
fn splitter_mixed_payloads_only_first_used() {
    let splitter = Splitter::new(|_ex| {
        vec![
            Message::from_text("FIRST"),
            Message::new(Payload::Bytes(vec![1, 2])),
            Message::from_text("THIRD"),
        ]
    });
    let route = Route::new().add(splitter).build();
    let mut ex = Exchange::new(Message::from_text("seed"));
    run_route(&route, &mut ex).unwrap();
    assert_eq!(ex.out_msg.unwrap().body_text(), Some("FIRST"));
}
