//! Static industrial pack taxonomy used by the descriptor catalog.
//!
//! This module is pure data. Keeping the taxonomy separate from catalog
//! construction prevents the registry implementation from growing into a giant
//! file as more provider-neutral sub-pack descriptors become specialized.

/// Descriptor seed for a child pack in the industrial capability catalog.
#[derive(Debug, Clone, Copy)]
pub(crate) struct IndustrialSubPackEntry {
    pub(crate) family: &'static str,
    pub(crate) slug: &'static str,
    pub(crate) label: &'static str,
}

/// Canonical child-pack taxonomy from the industrial capability catalog.
pub(crate) const INDUSTRIAL_SUB_PACKS: &[IndustrialSubPackEntry] = &[
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "filesystem",
        label: "Foundation filesystem",
    },
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "key-value-state",
        label: "Foundation key-value state",
    },
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "time",
        label: "Foundation time",
    },
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "random",
        label: "Foundation random",
    },
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "config",
        label: "Foundation config",
    },
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "secrets-reference",
        label: "Foundation secrets reference",
    },
    IndustrialSubPackEntry {
        family: "foundation",
        slug: "session-state",
        label: "Foundation session state",
    },
    IndustrialSubPackEntry {
        family: "communication",
        slug: "email",
        label: "Communication email",
    },
    IndustrialSubPackEntry {
        family: "communication",
        slug: "messaging",
        label: "Communication messaging",
    },
    IndustrialSubPackEntry {
        family: "communication",
        slug: "notification",
        label: "Communication notification",
    },
    IndustrialSubPackEntry {
        family: "communication",
        slug: "inbox",
        label: "Communication inbox",
    },
    IndustrialSubPackEntry {
        family: "communication",
        slug: "calendar",
        label: "Communication calendar",
    },
    IndustrialSubPackEntry {
        family: "knowledge",
        slug: "search",
        label: "Knowledge search",
    },
    IndustrialSubPackEntry {
        family: "knowledge",
        slug: "retrieval",
        label: "Knowledge retrieval",
    },
    IndustrialSubPackEntry {
        family: "knowledge",
        slug: "document-parsing",
        label: "Knowledge document parsing",
    },
    IndustrialSubPackEntry {
        family: "knowledge",
        slug: "citations",
        label: "Knowledge citations",
    },
    IndustrialSubPackEntry {
        family: "knowledge",
        slug: "graph",
        label: "Knowledge graph",
    },
    IndustrialSubPackEntry {
        family: "knowledge",
        slug: "summarization",
        label: "Knowledge summarization",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "code",
        label: "Developer code",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "repository",
        label: "Developer repository",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "ci",
        label: "Developer CI",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "issue-tracker",
        label: "Developer issue tracker",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "terminal",
        label: "Developer terminal",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "browser-automation",
        label: "Developer browser automation",
    },
    IndustrialSubPackEntry {
        family: "developer",
        slug: "design-tools",
        label: "Developer design tools",
    },
    IndustrialSubPackEntry {
        family: "office",
        slug: "document",
        label: "Office document",
    },
    IndustrialSubPackEntry {
        family: "office",
        slug: "spreadsheet",
        label: "Office spreadsheet",
    },
    IndustrialSubPackEntry {
        family: "office",
        slug: "presentation",
        label: "Office presentation",
    },
    IndustrialSubPackEntry {
        family: "office",
        slug: "pdf",
        label: "Office PDF",
    },
    IndustrialSubPackEntry {
        family: "office",
        slug: "forms",
        label: "Office forms",
    },
    IndustrialSubPackEntry {
        family: "media",
        slug: "image",
        label: "Media image",
    },
    IndustrialSubPackEntry {
        family: "media",
        slug: "audio",
        label: "Media audio",
    },
    IndustrialSubPackEntry {
        family: "media",
        slug: "video",
        label: "Media video",
    },
    IndustrialSubPackEntry {
        family: "media",
        slug: "transcription",
        label: "Media transcription",
    },
    IndustrialSubPackEntry {
        family: "media",
        slug: "rendering",
        label: "Media rendering",
    },
    IndustrialSubPackEntry {
        family: "finance",
        slug: "market-data",
        label: "Finance market data",
    },
    IndustrialSubPackEntry {
        family: "finance",
        slug: "stock",
        label: "Finance stock",
    },
    IndustrialSubPackEntry {
        family: "finance",
        slug: "crypto",
        label: "Finance crypto",
    },
    IndustrialSubPackEntry {
        family: "finance",
        slug: "accounting",
        label: "Finance accounting",
    },
    IndustrialSubPackEntry {
        family: "finance",
        slug: "portfolio",
        label: "Finance portfolio",
    },
    IndustrialSubPackEntry {
        family: "finance",
        slug: "invoice",
        label: "Finance invoice",
    },
    IndustrialSubPackEntry {
        family: "commerce",
        slug: "catalog",
        label: "Commerce catalog",
    },
    IndustrialSubPackEntry {
        family: "commerce",
        slug: "cart",
        label: "Commerce cart",
    },
    IndustrialSubPackEntry {
        family: "commerce",
        slug: "order",
        label: "Commerce order",
    },
    IndustrialSubPackEntry {
        family: "commerce",
        slug: "payment-intent",
        label: "Commerce payment intent",
    },
    IndustrialSubPackEntry {
        family: "commerce",
        slug: "receipt",
        label: "Commerce receipt",
    },
    IndustrialSubPackEntry {
        family: "commerce",
        slug: "entitlement",
        label: "Commerce entitlement",
    },
    IndustrialSubPackEntry {
        family: "identity",
        slug: "account",
        label: "Identity account",
    },
    IndustrialSubPackEntry {
        family: "identity",
        slug: "profile",
        label: "Identity profile",
    },
    IndustrialSubPackEntry {
        family: "identity",
        slug: "auth-handoff",
        label: "Identity auth handoff",
    },
    IndustrialSubPackEntry {
        family: "identity",
        slug: "organization",
        label: "Identity organization",
    },
    IndustrialSubPackEntry {
        family: "identity",
        slug: "tenant",
        label: "Identity tenant",
    },
    IndustrialSubPackEntry {
        family: "location",
        slug: "maps",
        label: "Location maps",
    },
    IndustrialSubPackEntry {
        family: "location",
        slug: "geocode",
        label: "Location geocode",
    },
    IndustrialSubPackEntry {
        family: "location",
        slug: "route",
        label: "Location route",
    },
    IndustrialSubPackEntry {
        family: "location",
        slug: "place-search",
        label: "Location place search",
    },
    IndustrialSubPackEntry {
        family: "location",
        slug: "timezone",
        label: "Location timezone",
    },
    IndustrialSubPackEntry {
        family: "device",
        slug: "sensors",
        label: "Device sensors",
    },
    IndustrialSubPackEntry {
        family: "device",
        slug: "camera",
        label: "Device camera",
    },
    IndustrialSubPackEntry {
        family: "device",
        slug: "local-files",
        label: "Device local files",
    },
    IndustrialSubPackEntry {
        family: "device",
        slug: "notifications",
        label: "Device notifications",
    },
    IndustrialSubPackEntry {
        family: "device",
        slug: "foreground-background-host",
        label: "Device foreground/background host",
    },
    IndustrialSubPackEntry {
        family: "ai",
        slug: "llm",
        label: "AI LLM",
    },
    IndustrialSubPackEntry {
        family: "ai",
        slug: "embedding",
        label: "AI embedding",
    },
    IndustrialSubPackEntry {
        family: "ai",
        slug: "rerank",
        label: "AI rerank",
    },
    IndustrialSubPackEntry {
        family: "ai",
        slug: "vision",
        label: "AI vision",
    },
    IndustrialSubPackEntry {
        family: "ai",
        slug: "speech",
        label: "AI speech",
    },
    IndustrialSubPackEntry {
        family: "ai",
        slug: "model-evaluation",
        label: "AI model evaluation",
    },
    IndustrialSubPackEntry {
        family: "workflow",
        slug: "task",
        label: "Workflow task",
    },
    IndustrialSubPackEntry {
        family: "workflow",
        slug: "schedule",
        label: "Workflow schedule",
    },
    IndustrialSubPackEntry {
        family: "workflow",
        slug: "approval",
        label: "Workflow approval",
    },
    IndustrialSubPackEntry {
        family: "workflow",
        slug: "delegation",
        label: "Workflow delegation",
    },
    IndustrialSubPackEntry {
        family: "workflow",
        slug: "review",
        label: "Workflow review",
    },
    IndustrialSubPackEntry {
        family: "workflow",
        slug: "recovery",
        label: "Workflow recovery",
    },
];
