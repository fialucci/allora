use allora::{
    error::{Error, Result},
    processor::{ClosureProcessor, Processor, SyncProcessor},
    route::Route,
    Exchange, Message,
};

// --- Sync tests (run when async feature NOT enabled) ---
#[cfg(not(feature = "async"))]
#[test]
fn closure_processor_mutates_exchange_sync() {
    let p = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("processed", "yes");
        ex.out_msg = Some(Message::from_text("done"));
        Ok(())
    });
    let mut ex = Exchange::new(Message::from_text("input"));
    p.process_sync(&mut ex).unwrap();
    assert_eq!(ex.in_msg.header("processed"), Some("yes"));
    assert_eq!(ex.out_msg.unwrap().body_text(), Some("done"));
}

#[cfg(not(feature = "async"))]
#[test]
fn route_stops_on_error_sync() {
    let good = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("step", "1");
        Ok(())
    });
    let bad =
        ClosureProcessor::new(|_ex: &mut Exchange| Err(Error::Processor("fail".into())));
    let after = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("should_not_run", "x");
        Ok(())
    });
    let route = Route::new().add(good).add(bad).add(after).build();
    let mut ex = Exchange::new(Message::from_text("start"));
    let err = route.run(&mut ex).unwrap_err();
    assert!(matches!(err, Error::Processor(_)));
    assert_eq!(ex.in_msg.header("step"), Some("1"));
    assert!(ex.in_msg.header("should_not_run").is_none());
}

// --- Async tests (default) ---
#[cfg(feature = "async")]
#[tokio::test]
async fn closure_processor_mutates_exchange_async() {
    let p = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("processed", "yes");
        ex.out_msg = Some(Message::from_text("done"));
        Ok(())
    });
    let mut ex = Exchange::new(Message::from_text("input"));
    p.process_sync(&mut ex).unwrap(); // still sync processor adapted
    assert_eq!(ex.in_msg.header("processed"), Some("yes"));
    assert_eq!(ex.out_msg.as_ref().unwrap().body_text(), Some("done"));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn route_stops_on_error_async() {
    let good = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("step", "1");
        Ok(())
    });
    let bad =
        ClosureProcessor::new(|_ex: &mut Exchange| Err(Error::Processor("fail".into())));
    let after = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("should_not_run", "x");
        Ok(())
    });
    let route = Route::new().add(good).add(bad).add(after).build();
    let mut ex = Exchange::new(Message::from_text("start"));
    let err = route.run(&mut ex).await.unwrap_err();
    assert!(matches!(err, Error::Processor(_)));
    assert_eq!(ex.in_msg.header("step"), Some("1"));
    assert!(ex.in_msg.header("should_not_run").is_none());
}

// Direct async processor implementation example
#[cfg(feature = "async")]
#[derive(Debug)]
struct DirectAsync;

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl Processor for DirectAsync {
    async fn process(&self, ex: &mut Exchange) -> Result<()> {
        // Simulate async work (no actual delay to keep tests fast)
        ex.in_msg.set_header("direct_async", "ok");
        Ok(())
    }
}

#[cfg(feature = "async")]
#[tokio::test]
async fn direct_async_processor_runs() {
    let route = Route::new().add(DirectAsync).build();
    let mut ex = Exchange::new(Message::from_text("start"));
    route.run(&mut ex).await.unwrap();
    assert_eq!(ex.in_msg.header("direct_async"), Some("ok"));
}

// Test the alias constructor `closure`
#[cfg(feature = "async")]
#[tokio::test]
async fn closure_alias_constructor() {
    let p = ClosureProcessor::closure(|ex: &mut Exchange| {
        ex.in_msg.set_header("alias", "used");
        Ok(())
    });
    let mut ex = Exchange::new(Message::from_text("alias"));
    p.process_sync(&mut ex).unwrap();
    assert_eq!(ex.in_msg.header("alias"), Some("used"));
}

#[cfg(not(feature = "async"))]
#[test]
fn closure_alias_constructor_sync() {
    let p = ClosureProcessor::closure(|ex: &mut Exchange| {
        ex.in_msg.set_header("alias", "used");
        Ok(())
    });
    let mut ex = Exchange::new(Message::from_text("alias"));
    p.process_sync(&mut ex).unwrap();
    assert_eq!(ex.in_msg.header("alias"), Some("used"));
}
