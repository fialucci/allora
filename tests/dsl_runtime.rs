use allora::build;

#[test]
fn build_runtime_channels_only_success() {
    let rt = build("tests/fixtures/allora.yml").expect("build runtime");
    assert_eq!(rt.channel_count(), 3);
    assert!(rt.channel_by_id("inbound.orders").is_some());
    assert!(rt.channel_by_id("processed.orders").is_some());
}
