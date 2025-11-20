use allora::{
    patterns::content_router::ContentBasedRouter, processor::ClosureProcessor, Exchange, Message,
};

#[tokio::test]
async fn content_router_selects_branch() {
    let a = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("routed", "a");
        Ok(())
    });
    let b = ClosureProcessor::new(|ex: &mut Exchange| {
        ex.in_msg.set_header("routed", "b");
        Ok(())
    });
    let router = ContentBasedRouter::new("kind")
        .when("a", Box::new(a))
        .when("b", Box::new(b));
    let mut ex = Exchange::new(Message::from_text("A"));
    ex.in_msg.set_header("kind", "a");
    router.process(&mut ex).await.unwrap();
    assert_eq!(ex.in_msg.header("routed"), Some("a"));
}
