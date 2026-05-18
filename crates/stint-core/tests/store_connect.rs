mod common;

#[tokio::test]
async fn connect_creates_and_migrates_database() {
    let env = common::setup().await;

    // Settings table should exist (we'll do a no-op query)
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM settings")
        .fetch_one(env.store.pool())
        .await
        .expect("select settings count");
    assert_eq!(rows, 0);
}
