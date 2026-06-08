//! Contract tests for [`WorkingMemory`] and [`InMemoryWorkingMemory`].

use crate::message::{Msg, Role};
use crate::state::StateModule;

use super::super::working::{InMemoryWorkingMemory, WorkingMemory};
use super::helpers::user_msg;

// 1. add and get (no filter)
    // -----------------------------------------------------------------------
#[tokio::test]
async fn test_add_and_get() {
    let mut mem = InMemoryWorkingMemory::new();
    let m = user_msg("hello");
    mem.add(m.clone(), vec![]).await;

    let got = mem.get_memory(None, None).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, m.id);
}

// -----------------------------------------------------------------------
// 2. mark filtering — include
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_mark_filtering() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("pinned");
    let b = user_msg("normal");
    mem.add(a.clone(), vec!["pinned".into()]).await;
    mem.add(b.clone(), vec![]).await;

    let pinned = mem.get_memory(Some("pinned"), None).await;
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].id, a.id);
}

// -----------------------------------------------------------------------
// 3. exclude_mark filtering
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_exclude_mark() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("compressed");
    let b = user_msg("fresh");
    mem.add(a.clone(), vec!["compressed".into()]).await;
    mem.add(b.clone(), vec![]).await;

    let fresh = mem.get_memory(None, Some("compressed")).await;
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].id, b.id);
}

// -----------------------------------------------------------------------
// 4. delete by ID
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_delete_by_id() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("a");
    let b = user_msg("b");
    let a_id = a.id.clone();
    mem.add(a, vec![]).await;
    mem.add(b.clone(), vec![]).await;

    mem.delete(&a_id).await;

    let got = mem.get_memory(None, None).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].id, b.id);
}

// -----------------------------------------------------------------------
// 5. delete_by_mark
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_delete_by_mark() {
    let mut mem = InMemoryWorkingMemory::new();
    mem.add(user_msg("draft-1"), vec!["draft".into()]).await;
    mem.add(user_msg("draft-2"), vec!["draft".into()]).await;
    mem.add(user_msg("final"), vec![]).await;

    mem.delete_by_mark("draft").await;

    let got = mem.get_memory(None, None).await;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].get_text(), "final");
}

// -----------------------------------------------------------------------
// 6. update_mark
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_update_mark() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("msg-a");
    let a_id = a.id.clone();
    mem.add(a, vec!["draft".into()]).await;
    mem.add(user_msg("msg-b"), vec!["draft".into()]).await;

    // Only update "a"
    mem.update_mark(&[a_id.clone()], "draft", "published").await;

    let published = mem.get_memory(Some("published"), None).await;
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].id, a_id);

    // "b" should still be "draft"
    let draft = mem.get_memory(Some("draft"), None).await;
    assert_eq!(draft.len(), 1);
    assert_eq!(draft[0].get_text(), "msg-b");
}

// -----------------------------------------------------------------------
// 7. summary prepended
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_summary() {
    let mut mem = InMemoryWorkingMemory::new();
    mem.add(user_msg("first"), vec![]).await;
    mem.add(user_msg("second"), vec![]).await;

    let summary = Msg::assistant("summary-bot", "Summary of earlier conversation.");
    mem.update_summary(summary.clone()).await;

    let with_summary = mem.get_with_summary().await;
    assert_eq!(with_summary.len(), 3);
    assert_eq!(with_summary[0].id, summary.id); // summary is first
    assert_eq!(with_summary[1].get_text(), "first");
    assert_eq!(with_summary[2].get_text(), "second");
}

// -----------------------------------------------------------------------
// 8. clear
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_clear() {
    let mut mem = InMemoryWorkingMemory::new();
    mem.add(user_msg("x"), vec!["tag".into()]).await;
    mem.update_summary(Msg::assistant("bot", "summary")).await;

    mem.clear().await;

    assert_eq!(mem.size().await, 0);
    // get_with_summary should return nothing after clear (summary also cleared)
    assert!(mem.get_with_summary().await.is_empty());
}

// -----------------------------------------------------------------------
// 9. state_dict / load_state_dict round-trip
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_state_roundtrip() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("state-a");
    let b = Msg::assistant("bot", "state-b");
    mem.add(a.clone(), vec!["important".into()]).await;
    mem.add(b.clone(), vec![]).await;
    mem.update_summary(Msg::system("compact summary")).await;

    let state = mem.state_dict();

    let mut restored = InMemoryWorkingMemory::new();
    restored.load_state_dict(state).unwrap();

    assert_eq!(restored.size().await, 2);

    let msgs = restored.get_memory(None, None).await;
    assert_eq!(msgs[0].id, a.id);
    assert_eq!(msgs[1].id, b.id);

    // marks preserved
    let important = restored.get_memory(Some("important"), None).await;
    assert_eq!(important.len(), 1);

    // summary preserved
    let with_summary = restored.get_with_summary().await;
    assert_eq!(with_summary.len(), 3); // summary + 2 msgs
    assert_eq!(with_summary[0].role, Role::System);
}

// -----------------------------------------------------------------------
// 10. size
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_size() {
    let mut mem = InMemoryWorkingMemory::new();
    assert_eq!(mem.size().await, 0);
    mem.add(user_msg("one"), vec![]).await;
    assert_eq!(mem.size().await, 1);
    mem.add(user_msg("two"), vec![]).await;
    assert_eq!(mem.size().await, 2);
    mem.delete_by_mark("nonexistent").await;
    assert_eq!(mem.size().await, 2);
}

// -----------------------------------------------------------------------
// 11. test_cross_mark_query
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_cross_mark_query() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("msg-a");
    let b = user_msg("msg-b");
    let c = user_msg("msg-c");
    mem.add(a.clone(), vec!["a".into()]).await;
    mem.add(b.clone(), vec!["b".into()]).await;
    mem.add(c.clone(), vec!["a".into(), "b".into()]).await;

    // Query mark="a" → should return a and c
    let with_a = mem.get_memory(Some("a"), None).await;
    assert_eq!(with_a.len(), 2);
    assert_eq!(with_a[0].id, a.id);
    assert_eq!(with_a[1].id, c.id);

    // Query exclude_mark="b" → should return a only
    let without_b = mem.get_memory(None, Some("b")).await;
    assert_eq!(without_b.len(), 1);
    assert_eq!(without_b[0].id, a.id);

    // When mark is provided, exclude_mark is ignored (mark wins per doc)
    let mark_wins = mem.get_memory(Some("a"), Some("b")).await;
    assert_eq!(mark_wins.len(), 2); // returns all with mark "a", ignoring exclude
}

// -----------------------------------------------------------------------
// 12. test_update_mark_nonexistent_id
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_update_mark_nonexistent_id() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("msg-a");
    mem.add(a.clone(), vec!["draft".into()]).await;

    // Update mark for a nonexistent ID — should not error
    mem.update_mark(&["nonexistent-id".to_string()], "draft", "published")
        .await;

    // Original message should still have "draft" mark
    let drafts = mem.get_memory(Some("draft"), None).await;
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].id, a.id);

    // No "published" marks
    let published = mem.get_memory(Some("published"), None).await;
    assert_eq!(published.len(), 0);
}

// -----------------------------------------------------------------------
// 13. test_delete_by_mark_count
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_delete_by_mark_count() {
    let mut mem = InMemoryWorkingMemory::new();
    for i in 0..5 {
        mem.add(user_msg(&format!("msg-{}", i)), vec!["x".into()])
            .await;
    }
    assert_eq!(mem.size().await, 5);

    mem.delete_by_mark("x").await;
    assert_eq!(mem.size().await, 0);
    let all = mem.get_memory(None, None).await;
    assert!(all.is_empty());
}

// -----------------------------------------------------------------------
// 14. test_empty_memory_get_with_summary
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_empty_memory_get_with_summary() {
    let mem = InMemoryWorkingMemory::new();
    let result = mem.get_with_summary().await;
    assert!(result.is_empty());
}

// -----------------------------------------------------------------------
// 15. test_large_message_count
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_large_message_count() {
    let mut mem = InMemoryWorkingMemory::new();
    for i in 0..500 {
        let mark = if i % 2 == 0 { "even" } else { "odd" };
        mem.add(user_msg(&format!("msg-{}", i)), vec![mark.into()])
            .await;
    }
    assert_eq!(mem.size().await, 500);

    let evens = mem.get_memory(Some("even"), None).await;
    assert_eq!(evens.len(), 250);

    let odds = mem.get_memory(Some("odd"), None).await;
    assert_eq!(odds.len(), 250);

    let no_evens = mem.get_memory(None, Some("even")).await;
    assert_eq!(no_evens.len(), 250);
}

// -----------------------------------------------------------------------
// 16. test_state_dict_roundtrip
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_state_dict_roundtrip() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("alpha");
    let b = Msg::assistant("bot", "beta");
    let c = user_msg("gamma");
    mem.add(a.clone(), vec!["tag1".into()]).await;
    mem.add(b.clone(), vec!["tag2".into()]).await;
    mem.add(c.clone(), vec!["tag1".into(), "tag2".into()]).await;

    let state = mem.state_dict();

    let mut restored = InMemoryWorkingMemory::new();
    restored.load_state_dict(state).unwrap();

    assert_eq!(restored.size().await, 3);

    let all = restored.get_memory(None, None).await;
    assert_eq!(all[0].id, a.id);
    assert_eq!(all[1].id, b.id);
    assert_eq!(all[2].id, c.id);

    // Marks preserved
    let tag1 = restored.get_memory(Some("tag1"), None).await;
    assert_eq!(tag1.len(), 2);
    let tag2 = restored.get_memory(Some("tag2"), None).await;
    assert_eq!(tag2.len(), 2);
}

// -----------------------------------------------------------------------
// 17. test_state_dict_with_summary
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_state_dict_with_summary() {
    let mut mem = InMemoryWorkingMemory::new();
    mem.add(user_msg("msg1"), vec![]).await;

    let summary = Msg::assistant("bot", "This is a summary.");
    mem.update_summary(summary.clone()).await;

    let state = mem.state_dict();

    let mut restored = InMemoryWorkingMemory::new();
    restored.load_state_dict(state).unwrap();

    let with_summary = restored.get_with_summary().await;
    assert_eq!(with_summary.len(), 2); // summary + 1 message
    assert_eq!(with_summary[0].id, summary.id);
    assert_eq!(with_summary[0].get_text(), "This is a summary.");
    assert_eq!(with_summary[0].role, Role::Assistant);
}

// -----------------------------------------------------------------------
// 18. test_clear_also_clears_summary
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_clear_also_clears_summary() {
    let mut mem = InMemoryWorkingMemory::new();
    mem.add(user_msg("hello"), vec![]).await;
    mem.update_summary(Msg::assistant("bot", "summary text"))
        .await;

    // Verify summary is present
    let before = mem.get_with_summary().await;
    assert_eq!(before.len(), 2);

    mem.clear().await;

    let after = mem.get_with_summary().await;
    assert!(after.is_empty());
    assert_eq!(mem.size().await, 0);
}

// -----------------------------------------------------------------------
// 19. test_delete_nonexistent_id
// -----------------------------------------------------------------------
#[tokio::test]
async fn test_delete_nonexistent_id() {
    let mut mem = InMemoryWorkingMemory::new();
    let a = user_msg("keep me");
    mem.add(a.clone(), vec![]).await;

    // Delete a nonexistent ID — should not error or affect existing data
    mem.delete("totally-fake-id").await;

    assert_eq!(mem.size().await, 1);
    let all = mem.get_memory(None, None).await;
    assert_eq!(all[0].id, a.id);
}

// =======================================================================
