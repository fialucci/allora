use allora::{
    processor::ClosureProcessor, route::Route, Channel, Exchange, InMemoryChannel, Message,
    OutboundQueue,
};

#[test]
fn channel_runs_route_and_sets_out_message() {
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("processed"));
            Ok(())
        }))
        .build();
    let channel = InMemoryChannel::new(route);
    let ex = Exchange::new(Message::from_text("input"));
    let processed = channel.dispatch(ex).expect("dispatch succeeds");
    assert_eq!(processed.out_msg.unwrap().body_text(), Some("processed"));
}

#[test]
fn channel_preserves_input_when_route_no_output() {
    let route = Route::new().build();
    let channel = InMemoryChannel::new(route);
    let ex = Exchange::new(Message::from_text("keep"));
    let processed = channel.dispatch(ex).expect("dispatch succeeds");
    assert!(processed.out_msg.is_none());
    assert_eq!(processed.in_msg.body_text(), Some("keep"));
}

#[test]
fn channel_enqueue_and_receive() {
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("processed"));
            Ok(())
        }))
        .build();
    let channel = InMemoryChannel::new(route);
    for _ in 0..3 {
        let ex = Exchange::new(Message::from_text("go"));
        let _ = channel.dispatch(ex).unwrap();
    }
    // Collect all processed exchanges
    let mut collected = Vec::new();
    #[cfg(not(feature = "async"))]
    while let Some(ex) = channel.try_receive() {
        collected.push(ex);
    }
    #[cfg(feature = "async")]
    while let Some(ex) = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(channel.try_receive_async())
    {
        collected.push(ex);
    }
    assert_eq!(collected.len(), 3);
    assert!(collected
        .iter()
        .all(|e| e.out_msg.as_ref().unwrap().body_text() == Some("processed")));
}
