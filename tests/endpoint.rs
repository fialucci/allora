#[cfg(feature = "http")]
use allora::adapter::Adapter;
#[cfg(feature = "http")]
use allora::channel::ChannelBuilder;
use allora::endpoint::{Endpoint, EndpointBuilder};
#[cfg(feature = "http")]
use allora::Channel;
use allora::{Exchange, Message};
#[cfg(feature = "http")]
use std::sync::Arc;

#[cfg(not(feature = "async"))]
#[test]
fn endpoint_fifo_sync() {
    let ep = EndpointBuilder::in_out().in_memory().build();
    ep.send(Exchange::new(Message::from_text("first"))).unwrap();
    ep.send(Exchange::new(Message::from_text("second")))
        .unwrap();
    ep.send(Exchange::new(Message::from_text("third"))).unwrap();
    assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("first"));
    assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("second"));
    assert_eq!(ep.try_receive().unwrap().in_msg.body_text(), Some("third"));
    assert!(ep.try_receive().is_none());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn endpoint_fifo_async() {
    let ep = EndpointBuilder::in_out().in_memory().build();
    ep.send_async(Exchange::new(Message::from_text("A")))
        .await
        .unwrap();
    ep.send_async(Exchange::new(Message::from_text("B")))
        .await
        .unwrap();
    assert_eq!(
        ep.try_receive_async().await.unwrap().in_msg.body_text(),
        Some("A")
    );
    assert_eq!(
        ep.try_receive_async().await.unwrap().in_msg.body_text(),
        Some("B")
    );
    assert!(ep.try_receive_async().await.is_none());
}

#[cfg(all(not(feature = "async"), feature = "http"))]
#[test]
fn endpoint_source_http_sync() {
    let route = allora::route::Route::new().build();
    let channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .build(),
    );
    let adapter = Arc::new(
        Adapter::inbound()
            .http()
            .host("127.0.0.1")
            .port(31050)
            .id("http-adapter-1")
            .channel(channel.clone())
            .build(),
    );
    let ep = EndpointBuilder::in_out()
        .in_memory()
        .channel(channel.clone())
        .source_http(&adapter, "POST", "/hooks/github")
        .id("ep-http")
        .build();
    let mut ex = Exchange::new(Message::from_text("payload"));
    ex.in_msg.set_header("source.kind", "pre-set");
    ep.send(ex).unwrap();
    let received = ep.try_receive().expect("received one");
    assert_eq!(received.in_msg.header("source.kind"), Some("pre-set"));
    assert_eq!(
        received.in_msg.header("source.adapter_id"),
        Some(adapter.id())
    );
    assert_eq!(received.in_msg.header("source.http.method"), Some("POST"));
    assert_eq!(
        received.in_msg.header("source.http.path"),
        Some("/hooks/github")
    );
}

#[cfg(all(not(feature = "async"), feature = "http"))]
#[test]
fn endpoint_source_channel_sync() {
    let route = allora::route::Route::new().build();
    let channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("channel-42")
            .build(),
    );
    let ep = EndpointBuilder::in_out()
        .in_memory()
        .source_channel(&channel)
        .build();
    ep.send(Exchange::new(Message::from_text("data"))).unwrap();
    let received = ep.try_receive().unwrap();
    assert_eq!(received.in_msg.header("source.kind"), Some("channel"));
    assert_eq!(
        received.in_msg.header("source.channel_id"),
        Some(channel.id())
    );
}

#[cfg(all(not(feature = "async"), feature = "http"))]
#[test]
fn endpoint_in_only_source_channel_sync() {
    let route = allora::route::Route::new().build();
    let channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("channel-inonly")
            .build(),
    );
    let ep = EndpointBuilder::in_only()
        .in_memory()
        .source_channel(&channel)
        .build();
    ep.send(Exchange::new(Message::from_text("ignored")))
        .unwrap();
    assert!(ep.try_receive().is_none());
}

#[cfg(all(feature = "async", feature = "http"))]
#[tokio::test]
async fn endpoint_source_http_async() {
    let route = allora::route::Route::new().build();
    let channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .build(),
    );
    let adapter = Arc::new(
        Adapter::inbound()
            .http()
            .host("127.0.0.1")
            .port(31051)
            .id("http-adapter-2")
            .channel(channel.clone())
            .build(),
    );
    let ep = EndpointBuilder::in_out()
        .in_memory()
        .channel(channel.clone())
        .source_http(&adapter, "PUT", "/api/items")
        .build();
    let mut ex = Exchange::new(Message::from_text("payload"));
    ex.in_msg.set_header("source.kind", "custom");
    ep.send_async(ex).await.unwrap();
    let received = ep.try_receive_async().await.expect("received one");
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

#[cfg(all(feature = "async", feature = "http"))]
#[tokio::test]
async fn endpoint_source_channel_async() {
    let route = allora::route::Route::new().build();
    let channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("chan-99")
            .build(),
    );
    let ep = EndpointBuilder::in_out()
        .in_memory()
        .source_channel(&channel)
        .build();
    ep.send_async(Exchange::new(Message::from_text("async")))
        .await
        .unwrap();
    let received = ep.try_receive_async().await.unwrap();
    assert_eq!(received.in_msg.header("source.kind"), Some("channel"));
    assert_eq!(
        received.in_msg.header("source.channel_id"),
        Some(channel.id())
    );
}

#[cfg(all(feature = "async", feature = "http"))]
#[tokio::test]
async fn endpoint_in_only_source_channel_async() {
    let route = allora::route::Route::new().build();
    let channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("chan-inonly")
            .build(),
    );
    let ep = EndpointBuilder::in_only()
        .in_memory()
        .source_channel(&channel)
        .build();
    ep.send_async(Exchange::new(Message::from_text("ignored")))
        .await
        .unwrap();
    assert!(ep.try_receive_async().await.is_none());
}

#[cfg(not(feature = "async"))]
#[test]
fn endpoint_custom_id() {
    let ep = EndpointBuilder::in_out()
        .in_memory()
        .id("custom-ep")
        .build();
    assert_eq!(ep.id(), "custom-ep");
}

#[cfg(all(not(feature = "async"), feature = "http"))]
#[test]
fn endpoint_attach_http_sync() {
    use allora::processor::ClosureProcessor;
    use allora::route::Route;
    // channel + route for endpoint
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("dyn"));
            Ok(())
        }))
        .build();
    let ep_channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("ep-chan-dyn")
            .build(),
    );
    let endpoint = EndpointBuilder::in_out()
        .in_memory()
        .channel(ep_channel.clone())
        .id("ep-dyn")
        .build();
    // adapter primary channel (can be same or separate; using ep_channel for simplicity)
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(34010)
        .channel(ep_channel.clone())
        .build();
    endpoint.attach_http(&adapter, "POST", "/dyn");
    let handle = adapter.spawn_serve();
    std::thread::sleep(std::time::Duration::from_millis(120));
    let client = hyper::Client::new();
    let req = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri("http://127.0.0.1:34010/dyn")
        .body(hyper::Body::from("in"))
        .unwrap();
    let resp = futures::executor::block_on(client.request(req)).unwrap();
    assert_eq!(resp.status(), 200);
    let body = futures::executor::block_on(hyper::body::to_bytes(resp.into_body())).unwrap();
    assert_eq!(&body[..], b"dyn");
    handle.abort();
}

#[cfg(all(not(feature = "async"), feature = "http"))]
#[test]
fn endpoint_attach_http_any_sync() {
    use allora::processor::ClosureProcessor;
    use allora::route::Route;
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("any"));
            Ok(())
        }))
        .build();
    let ep_channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("ep-chan-any")
            .build(),
    );
    let endpoint = EndpointBuilder::in_out()
        .in_memory()
        .channel(ep_channel.clone())
        .id("ep-any")
        .build();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(34011)
        .channel(ep_channel.clone())
        .build();
    endpoint.attach_http_any(&adapter, "/any");
    let handle = adapter.spawn_serve();
    std::thread::sleep(std::time::Duration::from_millis(120));
    let client = hyper::Client::new();
    for method in [hyper::Method::GET, hyper::Method::POST] {
        let req = hyper::Request::builder()
            .method(method.clone())
            .uri("http://127.0.0.1:34011/any")
            .body(hyper::Body::from("x"))
            .unwrap();
        let resp = futures::executor::block_on(client.request(req)).unwrap();
        assert_eq!(resp.status(), 200);
        let body = futures::executor::block_on(hyper::body::to_bytes(resp.into_body())).unwrap();
        assert_eq!(&body[..], b"any");
    }
    handle.abort();
}

#[cfg(all(feature = "async", feature = "http"))]
#[tokio::test]
async fn endpoint_attach_http_async() {
    use allora::processor::ClosureProcessor;
    use allora::route::Route;
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("dynA"));
            Ok(())
        }))
        .build();
    let ep_channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("ep-chan-dynA")
            .build(),
    );
    let endpoint = EndpointBuilder::in_out()
        .in_memory()
        .channel(ep_channel.clone())
        .id("ep-dynA")
        .build();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(34012)
        .channel(ep_channel.clone())
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
    assert_eq!(&body[..], b"dynA");
    handle.abort();
}

#[cfg(all(feature = "async", feature = "http"))]
#[tokio::test]
async fn endpoint_attach_http_any_async() {
    use allora::processor::ClosureProcessor;
    use allora::route::Route;
    let route = Route::new()
        .add(ClosureProcessor::new(|ex| {
            ex.out_msg = Some(Message::from_text("anyA"));
            Ok(())
        }))
        .build();
    let ep_channel = Arc::new(
        ChannelBuilder::point_to_point()
            .in_memory()
            .route(route)
            .id("ep-chan-anyA")
            .build(),
    );
    let endpoint = EndpointBuilder::in_out()
        .in_memory()
        .channel(ep_channel.clone())
        .id("ep-anyA")
        .build();
    let adapter = Adapter::inbound()
        .http()
        .host("127.0.0.1")
        .port(34013)
        .channel(ep_channel.clone())
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
        assert_eq!(&body[..], b"anyA");
    }
    handle.abort();
}
