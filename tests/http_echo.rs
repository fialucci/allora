#![cfg(feature = "http")]
use allora_core::adapter::Adapter;
use allora_core::channel::{ChannelRef, PollableChannel, QueueChannel};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_echo_single_request_and_shutdown() {
    let queue_channel = Arc::new(QueueChannel::with_random_id());
    let channel_ref: ChannelRef = queue_channel.clone();

    // Use a fixed high port; in real tests consider dynamic port allocation.
    let addr: SocketAddr = "127.0.0.1:31001".parse().unwrap();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(31001)
        .channel(channel_ref.clone())
        .build();
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

    let exchange = queue_channel
        .try_receive()
        .await
        .expect("exchange enqueued");
    assert_eq!(exchange.in_msg.body_text(), Some("hello"));
    assert_eq!(exchange.in_msg.header("http.method"), Some("POST"));
    assert_eq!(exchange.in_msg.header("http.path"), Some("/"));
    assert_eq!(
        exchange.in_msg.header("http.content_type"),
        Some("text/plain")
    );
    // Correlation headers should be present
    let corr_id = exchange.in_msg.header("corr_id").expect("corr_id present");
    assert_eq!(corr_id.len(), 36, "corr_id should be UUID v4 length");
    assert!(
        corr_id.chars().all(|c| c == '-' || c.is_ascii_hexdigit()),
        "corr_id must be hex+hyphens"
    );
    // Mirror header (correlation_id) should match
    assert_eq!(exchange.in_msg.header("correlation_id"), Some(corr_id));
    // Queue should now be empty
    assert!(queue_channel.try_receive().await.is_none());

    // Ensure the server task completes (graceful shutdown after a single request).
    server_handle.await.expect("server task join");
}
