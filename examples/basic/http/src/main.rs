use allora::{Result, Runtime};
use hyper::{Body, Client, Request, Response, Uri};
mod http_echo_service; // ensures service registered

#[tokio::main]
async fn main() -> Result<()> {
    // Attempt auto-discovery first
    let rt = Runtime::new().run()?;
    // Use first adapter
    let adapter = rt
        .http_inbound_adapters()
        .get(0)
        .expect("http inbound adapter in config")
        .clone();
    let _server = adapter.clone().spawn_serve();
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let uri: Uri = format!("http://{}{}", adapter.addr(), adapter.base_path())
        .parse()
        .unwrap();
    let resp = fire_one_request(uri, Body::from("World")).await?;
    let status = resp.status();
    let body_bytes = hyper::body::to_bytes(resp.into_body())
        .await
        .unwrap_or_default();
    let body = String::from_utf8_lossy(&body_bytes);
    println!("status={} body='{}'", status, body);
    Ok(())
}

async fn fire_one_request(uri: Uri, request: Body) -> Result<Response<Body>> {
    let client: Client<hyper::client::HttpConnector, Body> = Client::new();
    let req = Request::post(uri)
        .body(request)
        .map_err(|e| allora::Error::other(e.to_string()))?;
    let resp = client
        .request(req)
        .await
        .map_err(|e| allora::Error::other(e.to_string()))?;
    Ok(resp)
}
