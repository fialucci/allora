#![cfg(feature = "http")]
use allora::route::Route;
use allora::{ClosureProcessor, HttpInboundAdapter, InMemoryChannel, Message};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_echo_single_request_and_shutdown() {
    // Build an echo route: copy inbound body to out_msg unchanged.
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            let body = ex.in_msg.body_text().unwrap_or("");
            ex.out_msg = Some(Message::from_text(body));
            Ok(())
        }))
        .build();
    let channel = Arc::new(InMemoryChannel::new(route));

    // Use a fixed high port; in real tests consider dynamic port allocation.
    let addr: SocketAddr = "127.0.0.1:31001".parse().unwrap();
    let adapter = HttpInboundAdapter::new(addr, channel.clone());
    let server_handle = adapter.spawn_once();

    // Small delay to allow bind.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Prepare HTTP client request.
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(format!("http://{}", addr))
        .header("Content-Type", "text/plain")
        .body(hyper::Body::from("hello"))
        .unwrap();

    let resp = client.request(req).await.expect("response");
    assert_eq!(resp.status(), 200);
    let bytes = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&bytes[..], b"hello");

    // Ensure server task completes (graceful shutdown after single request).
    server_handle.await.expect("server task join");
}
