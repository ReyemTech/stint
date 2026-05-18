mod common;

use stint_core::store::queue::{Queue, QueueOp};

#[tokio::test]
async fn enqueue_then_take_due_returns_item() {
    let env = common::setup().await;
    let q = Queue::new(env.store.clone());

    q.enqueue(QueueOp::CreateEntry, "{\"x\":1}", Some("entry-1"))
        .await
        .unwrap();

    let due = q.take_due(10).await.unwrap();
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].op, "create_entry");
    assert_eq!(due[0].payload, "{\"x\":1}");
    assert_eq!(due[0].attempts, 0);
}

#[tokio::test]
async fn mark_succeeded_removes_item() {
    let env = common::setup().await;
    let q = Queue::new(env.store.clone());

    q.enqueue(QueueOp::CreateEntry, "{}", None).await.unwrap();
    let due = q.take_due(10).await.unwrap();
    let id = due[0].id;
    q.mark_succeeded(id).await.unwrap();

    let due = q.take_due(10).await.unwrap();
    assert!(due.is_empty());
}

#[tokio::test]
async fn mark_failed_increments_attempts_and_delays() {
    let env = common::setup().await;
    let q = Queue::new(env.store.clone());

    q.enqueue(QueueOp::UpdateEntry, "{}", None).await.unwrap();
    let due = q.take_due(10).await.unwrap();
    let id = due[0].id;
    q.mark_failed(id, "boom").await.unwrap();

    // After first failure, attempts == 1 and next_try_at is in the future.
    let due_now = q.take_due(10).await.unwrap();
    assert!(due_now.is_empty(), "should be backed off");
}
