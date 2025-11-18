use allora::{
    error::{Error, Result},
    processor::{ClosureProcessor, Processor},
    route::Route,
    Exchange, Message,
};

#[tokio::test]
async fn closure_processor_mutates_exchange() {
    let p = ClosureProcessor::new(|exchange: &mut Exchange| {
        exchange.in_msg.set_header("processed", "yes");
        exchange.out_msg = Some(Message::from_text("done"));
        Ok(())
    });
    let mut exchange = Exchange::new(Message::from_text("input"));
    p.process(&mut exchange).await.unwrap();
    assert_eq!(exchange.in_msg.header("processed"), Some("yes"));
    assert_eq!(exchange.out_msg.as_ref().unwrap().body_text(), Some("done"));
}

#[tokio::test]
async fn route_stops_on_error() {
    let good = ClosureProcessor::new(|exchange: &mut Exchange| {
        exchange.in_msg.set_header("step", "1");
        Ok(())
    });
    let bad = ClosureProcessor::new(|_ex: &mut Exchange| Err(Error::Processor("fail".into())));
    let after = ClosureProcessor::new(|exchange: &mut Exchange| {
        exchange.in_msg.set_header("should_not_run", "x");
        Ok(())
    });
    let route = Route::new().add(good).add(bad).add(after).build();
    let mut exchange = Exchange::new(Message::from_text("start"));
    let err = route.run(&mut exchange).await.unwrap_err();
    assert!(matches!(err, Error::Processor(_)));
    assert_eq!(exchange.in_msg.header("step"), Some("1"));
    assert!(exchange.in_msg.header("should_not_run").is_none());
}

#[derive(Debug)]
struct DirectAsync;

#[async_trait::async_trait]
impl Processor for DirectAsync {
    async fn process(&self, exchange: &mut Exchange) -> Result<()> {
        exchange.in_msg.set_header("direct_async", "ok");
        Ok(())
    }
}

#[tokio::test]
async fn direct_async_processor_runs() {
    let route = Route::new().add(DirectAsync).build();
    let mut exchange = Exchange::new(Message::from_text("start"));
    route.run(&mut exchange).await.unwrap();
    assert_eq!(exchange.in_msg.header("direct_async"), Some("ok"));
}

#[tokio::test]
async fn closure_alias_constructor() {
    let p = ClosureProcessor::closure(|exchange: &mut Exchange| {
        exchange.in_msg.set_header("alias", "used");
        Ok(())
    });
    let mut exchange = Exchange::new(Message::from_text("alias"));
    p.process(&mut exchange).await.unwrap();
    assert_eq!(exchange.in_msg.header("alias"), Some("used"));
}
