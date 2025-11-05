#![cfg(feature = "http")]
use allora::{
    processor::ClosureProcessor, route::Route, HttpInboundAdapter, InMemoryChannel, Message,
};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_injects_correlation_and_echoes_it() {
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            // Correlation should already be set by adapter before processors run.
            let cid = ex
                .in_msg
                .header("correlation_id")
                .expect("correlation id present");
            ex.out_msg = Some(Message::from_text(cid));
            Ok(())
        }))
        .build();
    let channel = Arc::new(InMemoryChannel::new(route));
    let addr: SocketAddr = "127.0.0.1:31002".parse().unwrap();
    let adapter = HttpInboundAdapter::new(addr, channel);
    let handle = adapter.spawn_once();

    // Allow bind
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://{}", addr))
        .header("Content-Type", "text/plain")
        .body(hyper::Body::from("test"))
        .unwrap();
    let resp = client.request(req).await.expect("http response");
    assert_eq!(resp.status(), 200);
    let bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    let cid = std::str::from_utf8(&bytes).unwrap();
    // Validate UUID v4 format roughly (length + hyphen positions + hex chars)
    assert_eq!(cid.len(), 36);
    for (i, ch) in cid.chars().enumerate() {
        if [8, 13, 18, 23].contains(&i) {
            assert_eq!(ch, '-');
        } else {
            assert!(ch.is_ascii_hexdigit());
        }
    }
    // Await server completion via Future impl
    handle.await.expect("server completed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_inbound_handle_wait_method() {
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("ok"));
            Ok(())
        }))
        .build();
    let channel = Arc::new(InMemoryChannel::new(route));
    let addr: SocketAddr = "127.0.0.1:31003".parse().unwrap();
    let adapter = HttpInboundAdapter::new(addr, channel);
    let handle = adapter.spawn_once();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://{}", addr))
        .body(hyper::Body::from("ignored"))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&body[..], b"ok");
    // Use explicit wait method instead of awaiting the Future directly
    handle
        .wait()
        .await
        .expect("server completed via wait method");
}
