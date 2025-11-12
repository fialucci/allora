use allora::build;

#[test]
fn build_runtime_channels_only_success() {
    let rt = build("tests/fixtures/allora.yml").expect("build runtime");
    assert_eq!(rt.channel_count(), 3);
    assert!(rt.channel_by_id("inbound.orders").is_some());
    assert!(rt.channel_by_id("processed.orders").is_some());
}

#[test]
fn build_runtime_with_filters_included() {
    let rt = build("tests/fixtures/allora.yml").expect("build runtime with filters");
    assert_eq!(rt.channel_count(), 3);
    assert_eq!(rt.filter_count(), 1);
    assert!(rt.channel_by_id("inbound.orders").is_some());
    assert!(rt.filters().len() == 1);
}
