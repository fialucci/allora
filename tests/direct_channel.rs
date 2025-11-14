use allora::{Channel, DirectChannel, DirectChannelBuilder, Error, Exchange, Message};
use std::sync::{Arc, Mutex};

#[test]
fn direct_channel_dispatch_order_and_success() {
    let dc = DirectChannel::new();
    let calls: Arc<Mutex<Vec<&'static str>>> = Arc::new(Mutex::new(Vec::new()));
    let c1 = calls.clone();
    dc.subscribe(move |ex: Exchange| {
        assert_eq!(ex.in_msg.body_text(), Some("ping"));
        c1.lock().unwrap().push("a");
        Ok(())
    });
    let c2 = calls.clone();
    dc.subscribe(move |ex: Exchange| {
        assert_eq!(ex.in_msg.body_text(), Some("ping"));
        c2.lock().unwrap().push("b");
        Ok(())
    });
    dc.send(Exchange::new(Message::from_text("ping"))).unwrap();
    assert_eq!(
        *calls.lock().unwrap(),
        vec!["a", "b"],
        "subscribers should be invoked in registration order"
    );
}

#[test]
fn direct_channel_error_short_circuits() {
    let dc = DirectChannelBuilder::new()
        .subscriber(|_ex| Err(Error::processor("fail-first")))
        .subscriber(|_ex| {
            panic!("second subscriber should not run after error");
        })
        .build();
    let err = dc
        .send(Exchange::new(Message::from_text("x")))
        .expect_err("expected error");
    match err {
        Error::Processor(msg) => assert!(msg.contains("fail-first")),
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "async")]
#[test]
fn direct_channel_async_dispatch() {
    let dc = DirectChannel::new();
    let hits = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let hits_c = hits.clone();
    dc.subscribe(move |_ex| {
        *hits_c.lock().unwrap() += 1;
        Ok(())
    });
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        dc.send_async(Exchange::new(Message::from_text("async")))
            .await
            .unwrap();
    });
    assert_eq!(*hits.lock().unwrap(), 1);
}
