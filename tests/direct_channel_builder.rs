use allora::{channel::ChannelBuilder, Channel};

#[test]
fn point_to_point_stage_direct_builds_direct_channel() {
    let dc = ChannelBuilder::point_to_point()
        .direct()
        .id("builder-direct")
        .build();
    // Type inference ensures dc is DirectChannel; confirm id prefix or set id.
    assert_eq!(dc.id(), "builder-direct");
    // Ensure kind accessor if implemented (currently only via ChannelInfo for DirectChannel).
    assert_eq!(allora::Channel::kind(&dc), "direct");
    // Dispatch semantics: subscribe & send
    let hits = std::sync::Arc::new(std::sync::Mutex::new(0));
    let hits_cl = hits.clone();
    dc.subscribe(move |_ex| {
        *hits_cl.lock().unwrap() += 1;
        Ok(())
    });
    dc.send(allora::Exchange::new(allora::Message::from_text("ping")))
        .unwrap();
    assert_eq!(*hits.lock().unwrap(), 1);
}
