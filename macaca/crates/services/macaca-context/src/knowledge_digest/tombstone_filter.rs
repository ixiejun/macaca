//! Tombstone **Decorator** semantics for [`crate::memory::KnowledgeDigestItem`] rows produced upstream.
//!
//! Claims whose evidence rows reference deleted memory ids must lose those references; when no evidence
//! remains, the entire claim is dropped so deleted knowledge cannot re-enter the model context via compiled
//! digest (OpenSpec `knowledge-digest-context`).

use std::collections::HashSet;

use crate::memory::KnowledgeDigestItem;

/// Removes tombstoned opaque memory ids from each item; drops items with no remaining evidence.
///
/// `tombstoned` uses the same string form as recall snippet source ids and governance
/// [`macaca_proto::MemoryId`] wire encoding.
#[must_use]
pub fn filter_digest_items_by_tombstones(
    mut items: Vec<KnowledgeDigestItem>,
    tombstoned: &HashSet<String>,
) -> Vec<KnowledgeDigestItem> {
    if tombstoned.is_empty() {
        return items;
    }
    items.retain_mut(|item| {
        item.evidence_memory_ids
            .retain(|id| !tombstoned.contains(id));
        !item.evidence_memory_ids.is_empty()
    });
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{ContextSourceProvenance, PrivacyTier};

    fn sample(id: &str, evidence: &[&str]) -> KnowledgeDigestItem {
        KnowledgeDigestItem {
            claim_id: id.into(),
            label: "t".into(),
            statement: "s".into(),
            provenance: ContextSourceProvenance {
                provider_id: "workspace-knowledge-digest".into(),
                source_id: id.into(),
                evidence: evidence.iter().map(|s| (*s).to_string()).collect(),
            },
            evidence_memory_ids: evidence.iter().map(|s| (*s).to_string()).collect(),
            confidence: 1.0,
            freshness: 1.0,
            privacy_tier: PrivacyTier::Workspace,
            request_only: true,
            redacted: true,
        }
    }

    #[test]
    fn drops_fully_tombstoned_claim() {
        let items = vec![sample("c1", &["m1"])];
        let ts: HashSet<String> = ["m1".into()].into_iter().collect();
        let out = filter_digest_items_by_tombstones(items, &ts);
        assert!(out.is_empty());
    }

    #[test]
    fn trims_partial_tombstone_overlap() {
        let items = vec![sample("c1", &["m1", "m2"])];
        let ts: HashSet<String> = ["m1".into()].into_iter().collect();
        let out = filter_digest_items_by_tombstones(items, &ts);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].evidence_memory_ids, vec!["m2".to_string()]);
    }
}
