use allora::{patterns::recipient_list::RecipientList, Exchange, Message};

#[tokio::test]
async fn recipient_list_invokes_all() {
    let hits = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let h1 = hits.clone();
    let h2 = hits.clone();
    let rl = RecipientList::new(vec![
        Box::new(move |_ex: &mut Exchange| {
            h1.lock().unwrap().push("one");
            Ok(())
        }),
        Box::new(move |_ex: &mut Exchange| {
            h2.lock().unwrap().push("two");
            Ok(())
        }),
    ]);
    let mut ex = Exchange::new(Message::from_text("demo"));
    rl.process_sync(&mut ex).unwrap();
    assert_eq!(*hits.lock().unwrap(), vec!["one", "two"]);
}
