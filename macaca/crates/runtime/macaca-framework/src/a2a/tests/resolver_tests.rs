//! Contract tests for [`FileCardResolver`] (Strategy: filesystem-backed AgentCard discovery).

use std::io::Write;

use crate::a2a::{AgentCardResolver, FileCardResolver};

use super::fixtures::sample_card;

#[tokio::test]
async fn test_file_card_resolver() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    let card_json = serde_json::to_string(&sample_card()).unwrap();
    tmp.write_all(card_json.as_bytes()).unwrap();

    let resolver = FileCardResolver::new(tmp.path());
    let card = resolver.resolve().await.unwrap();
    assert_eq!(card.name, "test-agent");
    assert_eq!(card.url, "http://localhost:9000");
    assert_eq!(card.skills.len(), 1);
}
