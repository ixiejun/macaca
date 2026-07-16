use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ai_common::{
    ai_bounded_token, ai_pack_definition, ai_stable_hash, define_ai_command_wrappers,
    AiPackCommandEnvelope, AiPackDescriptor, AiPackError, AiPackPage, AiProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const AI_LLM_PACK_ID: &str = "pack.ai.llm.v1";
pub const AI_LLM_SERVICE_ID: &str = "service.ai.llm";

/// Canonical command names described by `pack.ai.llm.v1`.
pub const AI_LLM_COMMANDS: &[&str] = &[
    "llm.chat",
    "llm.complete",
    "llm.route_model",
    "llm.estimate_tokens",
    "llm.inspect_budget",
    "llm.cancel_generation",
];

const LLM_PERMISSION_SCOPES: &[&str] = &["ai.llm.invoke", "ai.llm.route", "ai.llm.budget"];

const HOSTED_MODEL_METADATA: &[(&str, &str)] = &[
    ("streaming", "true"),
    ("tool_calls", "true"),
    ("structured_output", "true"),
    ("raw_prompts_in_trace", "false"),
];
const LOCAL_RUNTIME_METADATA: &[(&str, &str)] = &[
    ("network_required", "false"),
    ("structured_output", "limited"),
    ("model_names_exposed", "false"),
];
const PLUGIN_METADATA: &[(&str, &str)] = &[
    ("registration", "service_runtime"),
    ("policy_decorated", "true"),
];
const MOCK_METADATA: &[(&str, &str)] = &[
    ("deterministic", "true"),
    ("provider_payloads", "synthetic"),
];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const LLM_PROVIDER_CLASSES: &[AiProviderClass<'_>] = &[
    AiProviderClass {
        provider_class: "hosted-model",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: HOSTED_MODEL_METADATA,
    },
    AiProviderClass {
        provider_class: "local-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_RUNTIME_METADATA,
    },
    AiProviderClass {
        provider_class: "plugin",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLUGIN_METADATA,
    },
    AiProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    AiProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the LLM pack descriptor without binding a concrete model provider.
pub fn ai_llm_pack_definition() -> DomainPackDefinition {
    ai_pack_definition(AiPackDescriptor {
        pack_id: AI_LLM_PACK_ID,
        child_change_id: "openspec:add-pack-ai-llm",
        docs_slug: "llm",
        sdk_slug: "llm",
        service_id: AI_LLM_SERVICE_ID,
        commands: AI_LLM_COMMANDS,
        permission_scopes: LLM_PERMISSION_SCOPES,
        provider_classes: LLM_PROVIDER_CLASSES,
        health_probe: "llm.inspect_budget",
        unavailable_reason: "ai_llm_provider_not_installed",
        replay_schema: "ai.llm.replay.v1",
        data_classification: "ai_llm_reference_metadata",
        retention_policy: "messages_generation_options_tool_calls_budget_and_usage_by_reference",
        redaction_policy: "raw_prompts_outputs_tool_payloads_model_names_credentials_and_provider_payloads_redacted",
        timeout_ms: 120_000,
        budget_units: 12,
        examples: &[
            "Declare `pack.ai.llm.v1` as optional until an LLM provider is installed.",
            "Use message refs, content hashes, tool-call metadata, and budget envelopes instead of raw prompts.",
        ],
        migration_notes: &[
            "LLM commands become callable only after an approved LLM service provider registers matching schemas.",
            "Concrete providers, model names, prompts, and native payloads stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmInvocation {
    pub invocation_ref: String,
    pub messages: Vec<LlmMessage>,
    pub options: BTreeMap<String, String>,
    pub tool_calls: Vec<LlmToolCall>,
    pub budget: LlmBudgetEnvelope,
    pub redaction_profile: String,
}

impl LlmInvocation {
    /// Validate the request shape without reading or logging private prompt text.
    pub fn is_bounded(&self, max_messages: usize, max_tool_calls: usize) -> bool {
        !self.messages.is_empty()
            && self.messages.len() <= max_messages
            && self.tool_calls.len() <= max_tool_calls
            && self.budget.is_bounded()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmMessage {
    pub message_ref: String,
    pub role: String,
    pub content: Vec<LlmContentBlock>,
    pub redaction_class: String,
}

impl LlmMessage {
    /// Validate message metadata without carrying raw prompt or completion text.
    pub fn is_reference_only(&self) -> bool {
        ai_bounded_token(&self.message_ref, 128)
            && matches!(self.role.as_str(), "system" | "user" | "assistant" | "tool")
            && !self.content.is_empty()
            && self.content.iter().all(LlmContentBlock::is_reference_only)
            && ai_bounded_token(&self.redaction_class, 64)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmContentBlock {
    pub block_ref: String,
    pub content_kind: String,
    pub payload_ref: String,
    pub payload_hash: String,
}

impl LlmContentBlock {
    /// Validate content block references and hashes instead of raw content.
    pub fn is_reference_only(&self) -> bool {
        ai_bounded_token(&self.block_ref, 128)
            && matches!(
                self.content_kind.as_str(),
                "text_ref" | "image_ref" | "audio_ref" | "structured_output_ref"
            )
            && ai_bounded_token(&self.payload_ref, 256)
            && ai_bounded_token(&self.payload_hash, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub call_ref: String,
    pub tool_ref: String,
    pub capability_scope: String,
    pub argument_schema_ref: String,
    pub argument_hash: String,
}

impl LlmToolCall {
    /// Validate that generated tool calls are metadata only and require service policy.
    pub fn requires_policy_gate(&self) -> bool {
        ai_bounded_token(&self.call_ref, 128)
            && ai_bounded_token(&self.tool_ref, 128)
            && ai_bounded_token(&self.capability_scope, 128)
            && ai_bounded_token(&self.argument_schema_ref, 256)
            && ai_bounded_token(&self.argument_hash, 256)
            && self.capability_scope.starts_with("service.")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmStreamFrame {
    pub stream_ref: String,
    pub sequence: u64,
    pub frame_kind: String,
    pub delta_ref: String,
    pub terminal: bool,
}

impl LlmStreamFrame {
    /// Validate stream ordering so late frames cannot follow final or cancelled states.
    pub fn sequence_is_finalized(frames: &[LlmStreamFrame]) -> bool {
        !frames.is_empty()
            && frames.windows(2).all(|window| {
                let left = &window[0];
                let right = &window[1];
                left.stream_ref == right.stream_ref
                    && !left.terminal
                    && right.sequence == left.sequence + 1
            })
            && frames.iter().all(|frame| {
                ai_bounded_token(&frame.stream_ref, 128)
                    && matches!(frame.frame_kind.as_str(), "delta" | "final" | "cancelled")
                    && ai_bounded_token(&frame.delta_ref, 256)
            })
            && frames.last().is_some_and(|frame| frame.terminal)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmGeneration {
    pub generation_ref: String,
    pub content: Vec<LlmContentBlock>,
    pub finish_reason: String,
    pub usage: LlmBudgetEnvelope,
    pub safety_summary_ref: String,
}

impl LlmGeneration {
    /// Validate structured-output metadata without exposing raw model output.
    pub fn matches_structured_schema(&self, schema_ref: &str) -> bool {
        ai_bounded_token(&self.generation_ref, 128)
            && ai_bounded_token(schema_ref, 256)
            && matches!(
                self.finish_reason.as_str(),
                "stop" | "length" | "tool_call" | "cancelled"
            )
            && self.content.iter().any(|block| {
                block.content_kind == "structured_output_ref" && block.payload_ref == schema_ref
            })
            && self.content.iter().all(LlmContentBlock::is_reference_only)
            && self.usage.is_bounded()
            && ai_bounded_token(&self.safety_summary_ref, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmBudgetEnvelope {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub max_cost_micros: u64,
    pub retained_output_bytes: u64,
}

impl LlmBudgetEnvelope {
    /// Ensure budget metadata is explicit before a service provider may reserve resources.
    pub fn is_bounded(&self) -> bool {
        self.max_input_tokens > 0
            && self.max_output_tokens > 0
            && self.max_cost_micros > 0
            && self.retained_output_bytes > 0
    }

    /// Validate postflight usage against a reserved budget envelope.
    pub fn fits_within(&self, reserved: &LlmBudgetEnvelope) -> bool {
        self.max_input_tokens <= reserved.max_input_tokens
            && self.max_output_tokens <= reserved.max_output_tokens
            && self.max_cost_micros <= reserved.max_cost_micros
            && self.retained_output_bytes <= reserved.retained_output_bytes
    }
}

define_ai_command_wrappers!(
    LlmChatCommand,
    LlmCompleteCommand,
    LlmRouteModelCommand,
    LlmEstimateTokensCommand,
    LlmInspectBudgetCommand,
    LlmCancelGenerationCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    BudgetExceeded,
    Cancelled,
    SchemaMismatch,
    ToolPolicyRequired,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmResultEnvelope<T> {
    pub status: LlmResultStatus,
    pub data: Option<T>,
    pub page: Option<AiPackPage<T>>,
    pub error: Option<AiPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub invocation_hash: String,
    pub stream_frame_hash: String,
    pub budget_hash: String,
    pub redaction_profile_hash: String,
}

pub fn ai_llm_descriptor_hashes() -> LlmDescriptorHashes {
    LlmDescriptorHashes {
        command_schema_hash: llm_stable_hash(&AI_LLM_COMMANDS),
        result_schema_hash: llm_stable_hash(&LlmResultStatus::Success),
        descriptor_hash: llm_stable_hash(&ai_llm_pack_definition()),
        provider_capability_hash: llm_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        invocation_hash: llm_stable_hash(&LlmInvocation {
            invocation_ref: "invocation".into(),
            messages: vec![LlmMessage {
                message_ref: "message".into(),
                role: "user".into(),
                content: vec![LlmContentBlock {
                    block_ref: "block".into(),
                    content_kind: "text_ref".into(),
                    payload_ref: "prompt-ref".into(),
                    payload_hash: "prompt-hash".into(),
                }],
                redaction_class: "private".into(),
            }],
            budget: LlmBudgetEnvelope {
                max_input_tokens: 100,
                max_output_tokens: 50,
                max_cost_micros: 10,
                retained_output_bytes: 1024,
            },
            ..Default::default()
        }),
        stream_frame_hash: llm_stable_hash(&LlmStreamFrame {
            stream_ref: "stream".into(),
            sequence: 1,
            frame_kind: "delta".into(),
            delta_ref: "delta-ref".into(),
            terminal: false,
        }),
        budget_hash: llm_stable_hash(&LlmBudgetEnvelope {
            max_input_tokens: 100,
            max_output_tokens: 50,
            max_cost_micros: 10,
            retained_output_bytes: 1024,
        }),
        redaction_profile_hash: llm_stable_hash("llm-redaction-v1"),
    }
}

pub fn llm_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    ai_stable_hash(value)
}
