use allora_runtime::spec::{HttpInboundAdapterSpec, HttpInboundAdaptersSpec};

#[test]
fn http_inbound_adapters_spec_programmatic_add_two() {
    let spec = HttpInboundAdaptersSpec::new(1)
        .add(HttpInboundAdapterSpec::with_id(
            "http.recv",
            "0.0.0.0",
            8080,
            "/recv",
            vec!["POST".into()],
            "inbound.recv",
        ))
        .add(HttpInboundAdapterSpec::new(
            "127.0.0.1",
            8081,
            "/health",
            vec!["GET".into()],
            "inbound.health",
        ));
    assert_eq!(spec.version(), 1);
    assert_eq!(spec.adapters().len(), 2);
    assert_eq!(spec.adapters()[0].id(), Some("http.recv"));
    assert!(spec.adapters()[1].id().is_none());
}
