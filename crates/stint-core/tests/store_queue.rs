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

#[tokio::test]
async fn mark_abandoned_parks_row_far_in_future_and_resurrect_revives_it() {
    let env = common::setup().await;
    let q = Queue::new(env.store.clone());

    q.enqueue(QueueOp::CreateEntry, "{}", None).await.unwrap();
    let due = q.take_due(10).await.unwrap();
    let id = due[0].id;
    q.mark_abandoned(id, "validation rejected").await.unwrap();

    // Abandoned rows are not due — take_due returns empty.
    assert!(q.take_due(10).await.unwrap().is_empty());
    // The row itself still exists.
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sync_queue")
        .fetch_one(env.store.pool())
        .await
        .unwrap();
    assert_eq!(count, 1);

    // Resurrect resets next_try_at to now and clears attempts.
    let revived = q.resurrect_abandoned().await.unwrap();
    assert_eq!(revived, 1);

    let due_again = q.take_due(10).await.unwrap();
    assert_eq!(
        due_again.len(),
        1,
        "row should be picked up after resurrect"
    );
    assert_eq!(due_again[0].id, id);
    assert_eq!(due_again[0].attempts, 0);
}

#[tokio::test]
async fn delete_for_entry_drops_all_queued_ops_for_a_uuid() {
    let env = common::setup().await;
    let q = Queue::new(env.store.clone());

    q.enqueue(QueueOp::CreateEntry, "{}", Some("entry-a"))
        .await
        .unwrap();
    q.enqueue(QueueOp::UpdateEntry, "{}", Some("entry-a"))
        .await
        .unwrap();
    q.enqueue(QueueOp::CreateEntry, "{}", Some("entry-b"))
        .await
        .unwrap();

    let removed = q.delete_for_entry("entry-a").await.unwrap();
    assert_eq!(removed, 2);

    let remaining = q.take_due(10).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].entry_uuid.as_deref(), Some("entry-b"));
}

#[tokio::test]
async fn resurrect_abandoned_does_not_touch_normally_backed_off_rows() {
    let env = common::setup().await;
    let q = Queue::new(env.store.clone());

    q.enqueue(QueueOp::CreateEntry, "{}", None).await.unwrap();
    let due = q.take_due(10).await.unwrap();
    let id = due[0].id;
    // Plain transient failure — backoff is short, well under the 30-day
    // cutoff resurrect uses to identify abandoned rows.
    q.mark_failed(id, "transient 500").await.unwrap();

    let revived = q.resurrect_abandoned().await.unwrap();
    assert_eq!(revived, 0, "transient backoff is not abandonment");
}
