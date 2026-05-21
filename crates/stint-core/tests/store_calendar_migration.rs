mod common;

#[tokio::test]
async fn calendar_tables_exist_after_migration() {
    let env = common::setup().await;
    let pool = env.store.pool();

    // Each query must succeed (returns 0 rows but does not error out).
    sqlx::query("SELECT id, provider, display_name, identifier, caldav_url, enabled, created_at FROM calendar_accounts LIMIT 0")
        .execute(pool).await.unwrap();
    sqlx::query("SELECT id, account_id, name, color, included FROM calendars LIMIT 0")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("SELECT id, account_id, calendar_id, title, start_at, end_at, is_all_day, attendee_status, recurring_root, fetched_at FROM calendar_events LIMIT 0")
        .execute(pool).await.unwrap();
    sqlx::query("SELECT account_id, event_id, event_start, decision, linked_local_uuid, decided_at FROM event_decisions LIMIT 0")
        .execute(pool).await.unwrap();
}

#[tokio::test]
async fn calendar_events_pk_allows_same_event_id_at_different_starts() {
    let env = common::setup().await;
    let pool = env.store.pool();

    sqlx::query("INSERT INTO calendar_accounts (id, provider, display_name, identifier, enabled, created_at) VALUES (?, 'google', 'me', 'me@example.com', 1, '2026-05-19T00:00:00Z')")
        .bind("acc-1").execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO calendars (id, account_id, name, included) VALUES (?, ?, 'Primary', 1)",
    )
    .bind("cal-1")
    .bind("acc-1")
    .execute(pool)
    .await
    .unwrap();

    // Insert same event id with two different start_ats — both must succeed.
    for start in ["2026-05-19T09:00:00Z", "2026-05-26T09:00:00Z"] {
        sqlx::query("INSERT INTO calendar_events (id, account_id, calendar_id, title, start_at, end_at, is_all_day, fetched_at) VALUES (?, ?, ?, 'Standup', ?, ?, 0, ?)")
            .bind("evt-recurring")
            .bind("acc-1")
            .bind("cal-1")
            .bind(start)
            .bind(start)
            .bind("2026-05-19T00:00:00Z")
            .execute(pool).await.unwrap();
    }

    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM calendar_events WHERE id = 'evt-recurring'")
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn calendars_table_has_default_project_id_column() {
    let env = common::setup().await;
    let pool = env.store.pool();

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM pragma_table_info('calendars')
         WHERE name = 'default_project_id'",
    )
    .fetch_one(pool)
    .await
    .unwrap();

    assert_eq!(
        row.0, 1,
        "default_project_id column should exist after migration 0005"
    );
}
