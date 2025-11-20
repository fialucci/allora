use allora_core::channel::{ChannelRef, QueueChannel};
use allora_core::endpoint::{Endpoint, EndpointBuilder};
use allora_core::{Exchange, Message};
use allora_http::endpoint_http_ext::{HttpEndpointExt, HttpInOutEndpointBuilderExt};

#[tokio::test]
async fn endpoint_fifo() {
    let ep = EndpointBuilder::in_out().queue().build();
    ep.send(Exchange::new(Message::from_text("A")))
        .await
        .unwrap();
    ep.send(Exchange::new(Message::from_text("B")))
        .await
        .unwrap();
    assert_eq!(
        ep.try_receive().await.unwrap().in_msg.body_text(),
        Some("A")
    );
    assert_eq!(
        ep.try_receive().await.unwrap().in_msg.body_text(),
        Some("B")
    );
    assert!(ep.try_receive().await.is_none());
}

#[tokio::test]
async fn endpoint_source_http() {
    let channel_arc = Arc::new(QueueChannel::with_random_id());
    let channel_ref: ChannelRef = channel_arc.clone();
    let adapter = Arc::new(
        allora_core::adapter::Adapter::inbound()
            .http()
            .host("127.0.0.1")
            .port(31051)
            .id("http-adapter-2")
            .channel(channel_ref.clone())
            .build(),
    );
    let ep = EndpointBuilder::in_out()
        .queue()
        .channel(channel_ref.clone())
        .source_http(&adapter, "PUT", "/api/items")
        .build();
    let mut exchange = Exchange::new(Message::from_text("payload"));
    exchange.in_msg.set_header("source.kind", "custom");
    ep.send(exchange).await.unwrap();
    let received = ep.try_receive().await.expect("received one");
    assert_eq!(received.in_msg.header("source.kind"), Some("custom"));
    assert_eq!(
        received.in_msg.header("source.adapter_id"),
        Some(adapter.id())
    );
    assert_eq!(received.in_msg.header("source.http.method"), Some("PUT"));
    assert_eq!(
        received.in_msg.header("source.http.path"),
        Some("/api/items")
    );
}

#[tokio::test]
async fn endpoint_source_channel() {
    let channel: Arc<QueueChannel> = Arc::new(QueueChannel::with_id("chan-99"));
    let ep = EndpointBuilder::in_out()
        .queue()
        .source_channel(&channel)
        .build();
    ep.send(Exchange::new(Message::from_text("async")))
        .await
        .unwrap();
    let received = ep.try_receive().await.unwrap();
    assert_eq!(received.in_msg.header("source.kind"), Some("channel"));
    assert_eq!(
        received.in_msg.header("source.channel_id"),
        Some(channel.id())
    );
}

#[tokio::test]
async fn endpoint_in_only_source_channel() {
    let channel_arc: Arc<QueueChannel> = Arc::new(QueueChannel::with_id("chan-inonly"));
    let ep = EndpointBuilder::in_only()
        .queue()
        .source_channel(&channel_arc)
        .build();
    ep.send(Exchange::new(Message::from_text("ignored")))
        .await
        .unwrap();
    assert!(ep.try_receive().await.is_none());
}

#[tokio::test]
async fn endpoint_custom_id() {
    let ep = EndpointBuilder::in_out().queue().id("custom-ep").build();
    assert_eq!(ep.id(), "custom-ep");
}

#[tokio::test]
async fn endpoint_attach_http() {
    let ep_channel_arc = Arc::new(QueueChannel::with_id("ep-chan-dynA"));
    let ep_channel_ref: ChannelRef = ep_channel_arc.clone();
    let endpoint = EndpointBuilder::in_out()
        .queue()
        .channel(ep_channel_ref.clone())
        .id("ep-dynA")
        .build();
    let adapter = allora_core::adapter::Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(34012)
        .channel(ep_channel_ref.clone())
        .build();
    endpoint.attach_http(&adapter, "PUT", "/dynA");
    let handle = adapter.spawn_serve();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::PUT)
        .uri("http://127.0.0.1:34012/dynA")
        .body(hyper::Body::from("in"))
        .unwrap();
    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&body[..], b"in");
    handle.abort();
}

#[tokio::test]
async fn endpoint_attach_http_any() {
    let ep_channel_arc = Arc::new(QueueChannel::with_id("ep-chan-anyA"));
    let ep_channel_ref: ChannelRef = ep_channel_arc.clone();
    let endpoint = EndpointBuilder::in_out()
        .queue()
        .channel(ep_channel_ref.clone())
        .id("ep-anyA")
        .build();
    let adapter = allora_core::adapter::Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(34013)
        .channel(ep_channel_ref.clone())
        .build();
    endpoint.attach_http_any(&adapter, "/anyA");
    let handle = adapter.spawn_serve();
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let client = hyper::Client::new();
    for method in [hyper::Method::DELETE, hyper::Method::PATCH] {
        let req = hyper::Request::builder()
            .method(method.clone())
            .uri("http://127.0.0.1:34013/anyA")
            .body(hyper::Body::from("x"))
            .unwrap();
        let resp = client.request(req).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
        assert_eq!(&body[..], b"x");
    }
    handle.abort();
}
