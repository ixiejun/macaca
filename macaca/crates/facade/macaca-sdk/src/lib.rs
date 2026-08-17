//! `aos-sdk` — declarative agent SDK for Agent OS.
//!
//! Provides YAML/TOML configuration parsing, a fluent builder API,
//! and facade clients that register declarative agents through stable
//! service-oriented boundaries.

pub mod ability_kit;
pub mod alert_client;
pub mod app_protocol_client;
pub mod application;
pub mod application_client;
pub mod application_execution_client;
pub mod application_execution_facade;
pub mod application_kit;
pub mod application_testkit;
pub mod autonomy_evolution_client;
pub mod builder;
pub mod config;
pub mod domain_pack_accounting_client;
pub mod domain_pack_bridge;
pub mod domain_pack_client;
pub mod domain_pack_command_builder;
pub mod driver_client;
pub mod entitlement_client;
pub mod evm_client;
pub mod facade;
pub mod foundation_config_client;
pub mod foundation_filesystem_client;
pub mod foundation_random_client;
pub mod foundation_time_client;
pub mod heartbeat_client;
pub mod interaction_client;
pub mod llm_client;
pub mod mcp_client;
pub mod memory_client;
pub mod package_client;
pub mod package_fixtures;
pub mod payment_client;
pub mod persona;
pub mod persona_prototype;
pub mod plugin_capability_client;
pub mod plugin_client;
pub mod plugin_hook_client;
pub mod plugin_sdk;
pub mod scheduled_agent_task_client;
pub mod scheduler_client;
pub mod service_client;
pub mod spec;
pub mod status_client;
pub mod store_client;
pub mod system_facade;
pub mod task_client;
pub mod tool_client;
pub mod trace_client;
pub mod validation;
pub mod web3_client;
pub mod workbench_client;

#[cfg(test)]
mod application_execution_client_tests;

pub use ability_kit::{AbilityDescriptorBuilder, AbilityKit};
pub use alert_client::{
    ServiceBackedAlertClient, SystemAlertClient, UnavailableSystemAlertClient,
    ALERT_HEALTH_COMMAND, ALERT_RAISE_COMMAND, ALERT_RESOLVE_COMMAND, ALERT_SERVICE_ID,
    ALERT_SNAPSHOT_COMMAND,
};
pub use app_protocol_client::{
    ServiceBackedAppProtocolClient, SystemAppProtocolClient, UnavailableSystemAppProtocolClient,
};
pub use application::{
    service_call_command, trace_emit_command, ApplicationAbiBuilder, ApplicationHostCommandBuilder,
};
pub use application_client::{
    ServiceBackedApplicationClient, SystemApplicationClient, UnavailableSystemApplicationClient,
};
pub use application_execution_client::{
    ServiceBackedApplicationExecutionClient, SystemApplicationExecutionClient,
    UnavailableSystemApplicationExecutionClient,
};
pub use application_execution_facade::SystemApplicationExecutionFacadeExt;
pub use application_kit::{
    generate_wasm_guest_bindings, ApplicationKit, ApplicationManifestBuilder,
    RustWasmBindgenBackend, WasmBindgenBackend, WasmBindgenDiagnostic, WasmBindgenInput,
    WasmBindgenOutput, WasmComponentApplicationDescriptor, WasmComponentApplicationScaffold,
    WasmGuestBindingPlan, WasmMockHostImportBinding,
};
pub use application_testkit::{
    ApplicationContractDiagnostic, ApplicationContractReport, ApplicationContractTestKit,
};
pub use autonomy_evolution_client::{
    ServiceBackedAutonomyEvolutionClient, SystemAutonomyEvolutionClient,
    UnavailableSystemAutonomyEvolutionClient,
};
pub use builder::AgentBuilder;
pub use config::AgentConfig;
pub use domain_pack_accounting_client::{
    AccountingDomainPackCommandBuildOutcome, AccountingDomainPackCommandBuilder,
};
pub use domain_pack_bridge::{
    calendar_stable_hash, communication_calendar_descriptor_hashes,
    communication_calendar_pack_definition, communication_email_descriptor_hashes,
    communication_email_pack_definition, communication_inbox_descriptor_hashes,
    communication_inbox_pack_definition, communication_messaging_descriptor_hashes,
    communication_messaging_pack_definition, communication_notification_descriptor_hashes,
    communication_notification_pack_definition, compose_installed_domain_pack_catalog,
    config_stable_hash, developer_pack_definition, email_stable_hash, empty_domain_pack_catalog,
    expand_service_capabilities, filesystem_stable_hash, foundation_config_descriptor_hashes,
    foundation_config_pack_definition, foundation_filesystem_descriptor_hashes,
    foundation_filesystem_pack_definition, foundation_key_value_state_descriptor_hashes,
    foundation_key_value_state_pack_definition, foundation_pack_definition,
    foundation_random_descriptor_hashes, foundation_random_pack_definition,
    foundation_secrets_reference_descriptor_hashes, foundation_secrets_reference_pack_definition,
    foundation_session_state_descriptor_hashes, foundation_session_state_pack_definition,
    foundation_time_descriptor_hashes, foundation_time_pack_definition, inbox_stable_hash,
    industrial_reference_domain_pack_definitions, key_value_state_stable_hash,
    knowledge_pack_definition, messaging_stable_hash, notification_stable_hash, random_stable_hash,
    reference_domain_pack_definitions, secrets_reference_stable_hash, session_state_stable_hash,
    snapshot_domain_pack_catalog, time_stable_hash, AppPackPolicyOverride,
    AppServiceContractConfig, AppServiceContractSpec, AppServicePolicyOverride, CalendarAttendee,
    CalendarAvailabilityQuery, CalendarCheckAvailabilityCommand, CalendarConference,
    CalendarConflict, CalendarCreateEventCommand, CalendarCursor, CalendarDeleteEventCommand,
    CalendarDescriptorHashes, CalendarError, CalendarEvent, CalendarExportIcalendarCommand,
    CalendarGetEventCommand, CalendarImportIcalendarCommand, CalendarInspectConflictsCommand,
    CalendarInstance, CalendarListCalendarsCommand, CalendarManageConferenceCommand,
    CalendarProposeTimesCommand, CalendarProviderCapability, CalendarProviderSnapshot,
    CalendarQueryEventsCommand, CalendarRecurrence, CalendarRegisterWatchCommand, CalendarReminder,
    CalendarRespondInviteCommand, CalendarResultEnvelope, CalendarResultStatus,
    CalendarSetReminderCommand, CalendarSource, CalendarSyncEventsCommand,
    CalendarUpdateEventCommand, CalendarWatch, ConfigDescribeSchemaCommand, ConfigDescriptorHashes,
    ConfigError, ConfigExplainProvenanceCommand, ConfigExportRedactedCommand, ConfigGetCommand,
    ConfigGetManyCommand, ConfigKeyReference, ConfigLayerReference, ConfigListKeysCommand,
    ConfigProvenance, ConfigProviderCapability, ConfigProviderSnapshot, ConfigRedactionSummary,
    ConfigReloadCommand, ConfigResolveEffectiveCommand, ConfigResultEnvelope, ConfigResultStatus,
    ConfigSchemaReference, ConfigSelector, ConfigSnapshotCommand, ConfigSourceReference,
    ConfigTypedValueRef, ConfigValidateCommand, ConfigValidationReport, ConfigValueKind,
    ConfigWatchCommand, ConfigWatchEvent, DomainPackAvailability, DomainPackCallableSpec,
    DomainPackCatalog, DomainPackCatalogSnapshot, DomainPackCompatibility,
    DomainPackDataGovernance, DomainPackDefinition, DomainPackDefinitionSpec,
    DomainPackDiagnostics, DomainPackEffectiveCapabilityProjection, DomainPackHierarchySpec,
    DomainPackIdentitySpec, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityReport, DomainPackProviderCapabilityState,
    DomainPackProviderDescriptor, DomainPackProviderSnapshot, DomainPackSdkMetadata,
    DomainPackStability, DomainPackUnavailableDiagnostic, EffectiveServiceCapabilities,
    EmailApplyLabelsCommand, EmailAttachmentRef, EmailBodyKind, EmailBodyPart,
    EmailCancelScheduledSendCommand, EmailComposeCommand, EmailConsentStatus, EmailDeliveryState,
    EmailDeliveryStatusCommand, EmailDescriptorHashes, EmailDraftRef, EmailError,
    EmailFetchAttachmentCommand, EmailFetchMessageCommand, EmailIngestEventCommand,
    EmailListThreadsCommand, EmailMarkReadCommand, EmailMessageRef, EmailProviderCapability,
    EmailProviderEventRef, EmailProviderSnapshot, EmailRateLimitStatus, EmailRecipient,
    EmailRecipientKind, EmailResultEnvelope, EmailResultStatus, EmailSaveDraftCommand,
    EmailScheduleSendCommand, EmailSendCommand, EmailSenderRef, EmailSyncCursor,
    EmailSyncMailboxCommand, EmailUpdateDraftCommand, EmailValidateRecipientsCommand,
    FilesystemAccessMode, FilesystemAppendFileCommand, FilesystemCloseHandleCommand,
    FilesystemConflictMode, FilesystemContentRef, FilesystemCopyPathCommand,
    FilesystemCreateDirectoryCommand, FilesystemCreateTempCommand, FilesystemDeletePathCommand,
    FilesystemDescriptorHashes, FilesystemError, FilesystemHandleRef,
    FilesystemListDirectoryCommand, FilesystemMetadata, FilesystemMovePathCommand,
    FilesystemOpenHandleCommand, FilesystemPathRef, FilesystemProviderCapability,
    FilesystemProviderSnapshot, FilesystemReadFileCommand, FilesystemRestoreSnapshotCommand,
    FilesystemResultEnvelope, FilesystemResultStatus, FilesystemRootRef, FilesystemSnapshotRef,
    FilesystemSnapshotTreeCommand, FilesystemStatPathCommand, FilesystemWatchEvent,
    FilesystemWatchPathCommand, FilesystemWriteFileCommand, InMemoryDomainPackCatalog,
    InboxArchiveItemCommand, InboxAttachmentHandle, InboxClaim, InboxClaimItemCommand, InboxCursor,
    InboxDescriptorHashes, InboxError, InboxEvent, InboxFetchAttachmentCommand,
    InboxFetchBodyCommand, InboxGetItemCommand, InboxIngestEventCommand, InboxItem, InboxLabel,
    InboxLabelItemCommand, InboxListItemsCommand, InboxListThreadsCommand, InboxMarkReadCommand,
    InboxMoveItemCommand, InboxProviderCapability, InboxProviderSnapshot,
    InboxRegisterSourceCommand, InboxReleaseItemCommand, InboxResultEnvelope, InboxResultStatus,
    InboxResumeSyncCommand, InboxRevokeSourceCommand, InboxSearchItemsCommand, InboxSource,
    InboxSummarizeItemCommand, InboxSyncCheckpoint, InboxSyncSourcesCommand, InboxThread,
    InboxUpdateSourceCommand, KeyValueBatchDeleteCommand, KeyValueBatchGetCommand,
    KeyValueBatchPutCommand, KeyValueCompactNamespaceCommand, KeyValueCompareAndSetCommand,
    KeyValueConflictMode, KeyValueConsistencyLevel, KeyValueDeleteCommand, KeyValueExistsCommand,
    KeyValueGetCommand, KeyValueGetTtlCommand, KeyValueIncrementCommand, KeyValueKeyRef,
    KeyValueListKeysCommand, KeyValueMigrateNamespaceCommand, KeyValueNamespaceRef,
    KeyValuePutCommand, KeyValueRestoreNamespaceCommand, KeyValueRevision, KeyValueSetTtlCommand,
    KeyValueSnapshotNamespaceCommand, KeyValueSnapshotRef, KeyValueStateDescriptorHashes,
    KeyValueStateError, KeyValueStateProviderCapability, KeyValueStateProviderSnapshot,
    KeyValueStateResultEnvelope, KeyValueStateResultStatus, KeyValueTtlPolicy,
    KeyValueTypedValueRef, KeyValueWatchEvent, KeyValueWatchNamespaceCommand,
    MessagingAttachHandleCommand, MessagingAttachmentRef, MessagingContent,
    MessagingConversationKind, MessagingConversationRef, MessagingCreateConversationCommand,
    MessagingCursor, MessagingDeleteMessageCommand, MessagingDeliveryState,
    MessagingDeliveryStatusCommand, MessagingDescriptorHashes, MessagingEditMessageCommand,
    MessagingError, MessagingFetchMessageCommand, MessagingFindConversationCommand,
    MessagingIngestEventCommand, MessagingInspectParticipantsCommand, MessagingListMessagesCommand,
    MessagingMarkReadCommand, MessagingMessageRef, MessagingParticipantRef,
    MessagingProviderCapability, MessagingProviderEventRef, MessagingProviderSnapshot,
    MessagingRateLimitStatus, MessagingReaction, MessagingReactionCommand,
    MessagingReplyMessageCommand, MessagingResultEnvelope, MessagingResultStatus,
    MessagingSendMessageCommand, MessagingSendTypingCommand, MessagingSenderRef,
    NotificationAcknowledgeCommand, NotificationActionDefinition, NotificationActionEvent,
    NotificationCancelCommand, NotificationDeliveryChannel, NotificationDeliveryHandle,
    NotificationDeliveryStatus, NotificationDescriptorHashes, NotificationDismissCommand,
    NotificationError, NotificationInspectDeliveryCommand, NotificationListNotificationsCommand,
    NotificationMessage, NotificationProviderCapability, NotificationProviderSnapshot,
    NotificationPublishCommand, NotificationRegisterActionCommand,
    NotificationRegisterSubscriptionCommand, NotificationResultEnvelope, NotificationResultStatus,
    NotificationRevokeSubscriptionCommand, NotificationSchedule, NotificationScheduleCommand,
    NotificationSubscriptionHandle, NotificationTarget, NotificationUnregisterActionCommand,
    NotificationUpdateCommand, RandomAlphabetClass, RandomBytesCommand, RandomDescriptorHashes,
    RandomEntropyHealth, RandomEntropyHealthCommand, RandomError, RandomFillCommand,
    RandomIntegerCommand, RandomNonceCommand, RandomOutputEncoding,
    RandomProviderCapabilitiesCommand, RandomProviderCapability, RandomProviderSnapshot,
    RandomPurpose, RandomReplayPolicy, RandomResultEnvelope, RandomResultStatus,
    RandomSeedReference, RandomStreamReference, RandomStrengthClass, RandomTestStreamBytesCommand,
    RandomTestStreamCreateCommand, RandomTokenCommand, RandomUuidV4Command, SecretAccessPolicy,
    SecretAuditRecord, SecretExternalLocator, SecretLeaseReference, SecretPurposeBinding,
    SecretReference, SecretResolutionHandle, SecretVersionState, SecretVersionStatus,
    SecretsAuditAccessCommand, SecretsBindPurposeCommand, SecretsCreateLeaseCommand,
    SecretsCreateReferenceCommand, SecretsImportReferenceCommand, SecretsInspectReferenceCommand,
    SecretsListReferencesCommand, SecretsReferenceDescriptorHashes, SecretsReferenceError,
    SecretsReferenceProviderCapability, SecretsReferenceProviderSnapshot,
    SecretsReferenceResultEnvelope, SecretsReferenceResultStatus, SecretsRenewLeaseCommand,
    SecretsResolveForProviderCommand, SecretsRevokeLeaseCommand, SecretsRotateReferenceCommand,
    SecretsVersionStatusCommand, SessionStateCheckpointRef, SessionStateClearSessionCommand,
    SessionStateCompactHistoryCommand, SessionStateCompareCheckpointCommand,
    SessionStateCreateCheckpointCommand, SessionStateDeleteCommand, SessionStateDescriptorHashes,
    SessionStateError, SessionStateExportRedactedCommand, SessionStateGetCommand,
    SessionStateInspectRecoveryCommand, SessionStateKeyRef, SessionStateListCheckpointsCommand,
    SessionStateListKeysCommand, SessionStateMergePatchCommand, SessionStateProviderCapability,
    SessionStateProviderSnapshot, SessionStatePutCommand, SessionStateRecoveryMetadata,
    SessionStateRedactionSummary, SessionStateRestoreCheckpointCommand, SessionStateRestorePlan,
    SessionStateResultEnvelope, SessionStateResultStatus, SessionStateRetentionPolicy,
    SessionStateRevision, SessionStateSessionRef, SessionStateValueRef, SharedDomainPackCatalog,
    TimeAddDurationCommand, TimeCalendarConvertCommand, TimeCalendarReference,
    TimeCancelTimerCommand, TimeClockHealth, TimeClockHealthCommand, TimeClockSource,
    TimeConvertTimezoneCommand, TimeCreateTimerCommand, TimeDeadlineSpec, TimeDescriptorHashes,
    TimeDuration, TimeDurationBetweenCommand, TimeError, TimeEvaluateDeadlineCommand,
    TimeExactnessHint, TimeFormatCommand, TimeFormatSpec, TimeInspectTimerCommand, TimeInstant,
    TimeLocaleReference, TimeMonotonicInstant, TimeMonotonicNowCommand, TimeNowCommand,
    TimeParseCommand, TimeProviderCapability, TimeProviderSnapshot, TimeResolveTimezoneCommand,
    TimeResultEnvelope, TimeResultStatus, TimeTimerReference, TimeZoneReference,
    COMMUNICATION_CALENDAR_COMMANDS, COMMUNICATION_CALENDAR_PACK_ID,
    COMMUNICATION_CALENDAR_SERVICE_ID, COMMUNICATION_EMAIL_COMMANDS, COMMUNICATION_EMAIL_PACK_ID,
    COMMUNICATION_EMAIL_SERVICE_ID, COMMUNICATION_INBOX_COMMANDS, COMMUNICATION_INBOX_PACK_ID,
    COMMUNICATION_INBOX_SERVICE_ID, COMMUNICATION_MESSAGING_COMMANDS,
    COMMUNICATION_MESSAGING_PACK_ID, COMMUNICATION_MESSAGING_SERVICE_ID,
    COMMUNICATION_NOTIFICATION_COMMANDS, COMMUNICATION_NOTIFICATION_PACK_ID,
    COMMUNICATION_NOTIFICATION_SERVICE_ID, FOUNDATION_CONFIG_COMMANDS, FOUNDATION_CONFIG_PACK_ID,
    FOUNDATION_CONFIG_SERVICE_ID, FOUNDATION_FILESYSTEM_COMMANDS, FOUNDATION_FILESYSTEM_PACK_ID,
    FOUNDATION_FILESYSTEM_SERVICE_ID, FOUNDATION_KEY_VALUE_STATE_COMMANDS,
    FOUNDATION_KEY_VALUE_STATE_PACK_ID, FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
    FOUNDATION_RANDOM_COMMANDS, FOUNDATION_RANDOM_PACK_ID, FOUNDATION_RANDOM_SERVICE_ID,
    FOUNDATION_SECRETS_REFERENCE_COMMANDS, FOUNDATION_SECRETS_REFERENCE_PACK_ID,
    FOUNDATION_SECRETS_REFERENCE_SERVICE_ID, FOUNDATION_SESSION_STATE_COMMANDS,
    FOUNDATION_SESSION_STATE_PACK_ID, FOUNDATION_SESSION_STATE_SERVICE_ID,
    FOUNDATION_TIME_COMMANDS, FOUNDATION_TIME_PACK_ID, FOUNDATION_TIME_SERVICE_ID,
};
pub use domain_pack_bridge::{
    citations_stable_hash, document_parsing_stable_hash, graph_stable_hash,
    knowledge_citations_descriptor_hashes, knowledge_citations_pack_definition,
    knowledge_document_parsing_descriptor_hashes, knowledge_document_parsing_pack_definition,
    knowledge_graph_descriptor_hashes, knowledge_graph_pack_definition,
    knowledge_retrieval_descriptor_hashes, knowledge_retrieval_pack_definition,
    knowledge_search_descriptor_hashes, knowledge_search_pack_definition,
    knowledge_summarization_descriptor_hashes, knowledge_summarization_pack_definition,
    retrieval_stable_hash, search_stable_hash, summarization_stable_hash, BibliographyStyle,
    CitationContributor, CitationDescriptorHashes, CitationEvidence, CitationExportResult,
    CitationIdentifier, CitationImportResult, CitationItem, CitationProviderCapability,
    CitationResultEnvelope, CitationResultStatus, CitationSelector, CitationSourceAnchor,
    CitationVerificationResult, CitationsCreateCitationCommand, CitationsExportCitationsCommand,
    CitationsFormatBibliographyCommand, CitationsFormatCitationCommand,
    CitationsImportCitationsCommand, CitationsInspectProviderCommand,
    CitationsInspectSourceAnchorCommand, CitationsLinkSourceSpanCommand,
    CitationsListCitationsCommand, CitationsResolveIdentifierCommand,
    CitationsUpdateCitationCommand, CitationsVerifyCitationCommand, CompressionMap, DocumentChunk,
    DocumentConfidence, DocumentElement, DocumentEmbeddedResource, DocumentEntity,
    DocumentFormField, DocumentGeometry, DocumentMetadata, DocumentOcrToken, DocumentPage,
    DocumentParseResult, DocumentParserCapability, DocumentParsingCancelParseJobCommand,
    DocumentParsingChunkDocumentCommand, DocumentParsingConvertToCanonicalCommand,
    DocumentParsingDescriptorHashes, DocumentParsingDetectFormatCommand,
    DocumentParsingExtractFormsCommand, DocumentParsingExtractLayoutCommand,
    DocumentParsingExtractMetadataCommand, DocumentParsingExtractTablesCommand,
    DocumentParsingExtractTextCommand, DocumentParsingGetParseJobCommand,
    DocumentParsingInspectParserCommand, DocumentParsingParseDocumentCommand,
    DocumentParsingResultEnvelope, DocumentParsingResultStatus,
    DocumentParsingStartParseJobCommand, DocumentParsingValidateDocumentCommand, DocumentSource,
    DocumentTable, DocumentTableCell, DocumentTextSpan, FormattedCitation,
    GraphDeleteGraphItemsCommand, GraphDeleteTriplesCommand, GraphDescriptorHashes, GraphEdge,
    GraphExportPlan, GraphExportSubgraphCommand, GraphFindPathCommand, GraphImportPlan,
    GraphImportSubgraphCommand, GraphInspectProvenanceCommand, GraphInspectProviderCommand,
    GraphInspectStoreCommand, GraphMergeEntitiesCommand, GraphNode, GraphPath, GraphProperty,
    GraphProvenance, GraphProviderCapability, GraphQuery, GraphQueryCommand, GraphQueryResult,
    GraphRegisterStoreCommand, GraphResultEnvelope, GraphResultStatus, GraphSchema, GraphStore,
    GraphTraversal, GraphTraverseCommand, GraphUpsertEdgeCommand, GraphUpsertNodeCommand,
    GraphUpsertSchemaCommand, GraphUpsertTripleCommand, GraphValidateQueryCommand,
    GraphValidateSchemaCommand, KnowledgeCommandEnvelope, KnowledgeError, KnowledgePage, ParseJob,
    ParserProfile, RdfStatement, RdfTerm, RetrievalBulkRetrieveCommand, RetrievalCandidate,
    RetrievalChunk, RetrievalCollection, RetrievalCursor, RetrievalDeleteRecordsCommand,
    RetrievalDescriptorHashes, RetrievalEvidenceBundle, RetrievalExpandContextCommand,
    RetrievalFreshness, RetrievalFusionStrategy, RetrievalInspectCollectionCommand,
    RetrievalInspectRecordCommand, RetrievalMetadataFilter, RetrievalNamespace,
    RetrievalPackageEvidenceCommand, RetrievalProviderCapability, RetrievalQuery,
    RetrievalQueryDiagnosticsCommand, RetrievalRangeRetrieveCommand, RetrievalRecord,
    RetrievalRefreshCollectionCommand, RetrievalRegisterCollectionCommand,
    RetrievalRerankContextCommand, RetrievalResultEnvelope, RetrievalResultStatus,
    RetrievalRetrieveByIdCommand, RetrievalRetrieveCommand, RetrievalUpsertRecordsCommand,
    RetrievalVectorSpace, SearchAnalyzerProfile, SearchAutocompleteCommand, SearchCorpus,
    SearchCursor, SearchDescriptorHashes, SearchExplainRankingCommand, SearchFacetRequest,
    SearchFacetsCommand, SearchField, SearchFilter, SearchHit, SearchIndexSchema,
    SearchIndexStatsCommand, SearchInspectIndexCommand, SearchProviderCapability, SearchQuery,
    SearchQueryDiagnosticsCommand, SearchRankingExplanation, SearchRankingProfile,
    SearchRefreshIndexCommand, SearchRegisterCorpusCommand, SearchResultEnvelope,
    SearchResultStatus, SearchSearchCommand, SearchSort, SearchSuggestCommand, SearchSynonymSet,
    SummarizationCompareSummariesCommand, SummarizationCompressContextCommand,
    SummarizationDescriptorHashes, SummarizationEvaluateSummaryCommand,
    SummarizationInspectProviderCommand, SummarizationInspectSummaryEvidenceCommand,
    SummarizationPlanCommand, SummarizationRefineSummaryCommand, SummarizationResultEnvelope,
    SummarizationResultStatus, SummarizationSummarizeCommand,
    SummarizationSummarizeConversationCommand, SummarizationSummarizeManyCommand,
    SummarizationSummarizeWithCitationsCommand, SummarizationValidateRequestCommand, SummaryClaim,
    SummaryComparisonReport, SummaryEvidenceLink, SummaryOutput, SummaryPlan,
    SummaryProviderCapability, SummaryQualityReport, SummaryRequest, SummarySource,
    KNOWLEDGE_CITATIONS_COMMANDS, KNOWLEDGE_CITATIONS_PACK_ID, KNOWLEDGE_CITATIONS_SERVICE_ID,
    KNOWLEDGE_DOCUMENT_PARSING_COMMANDS, KNOWLEDGE_DOCUMENT_PARSING_PACK_ID,
    KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID, KNOWLEDGE_GRAPH_COMMANDS, KNOWLEDGE_GRAPH_PACK_ID,
    KNOWLEDGE_GRAPH_SERVICE_ID, KNOWLEDGE_RETRIEVAL_COMMANDS, KNOWLEDGE_RETRIEVAL_PACK_ID,
    KNOWLEDGE_RETRIEVAL_SERVICE_ID, KNOWLEDGE_SEARCH_COMMANDS, KNOWLEDGE_SEARCH_PACK_ID,
    KNOWLEDGE_SEARCH_SERVICE_ID, KNOWLEDGE_SUMMARIZATION_COMMANDS, KNOWLEDGE_SUMMARIZATION_PACK_ID,
    KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
};
pub use domain_pack_client::{
    CatalogBackedDomainPackClient, DomainPackInspectCommand, DomainPackInspectResult,
    DomainPackListCommand, DomainPackListResult, DomainPackResolveCommand, DomainPackResolveResult,
    DomainPackServiceCallBuilder, EmptySystemDomainPackClient, SystemDomainPackClient,
};
pub use domain_pack_command_builder::{
    DomainPackCommandCatalogBuilder, DomainPackCommandSpec, DomainPackDeclaredCommandBuilder,
};
pub use driver_client::{
    ServiceBackedDriverClient, SystemDriverClient, UnavailableSystemDriverClient,
};
pub use entitlement_client::{
    ServiceBackedEntitlementClient, SystemEntitlementClient, UnavailableSystemEntitlementClient,
};
pub use evm_client::{ServiceBackedEvmClient, SystemEvmClient, UnavailableSystemEvmClient};
pub use facade::{AgentRegistryApi, MacacaSdk};
pub use foundation_config_client::{
    config_describe_schema_command, config_effective_command, config_export_redacted_command,
    config_get_command, config_get_many_command, config_list_keys_command,
    config_provenance_command, config_reload_command, config_snapshot_command,
    config_unavailable_diagnostics_command, config_validate_command, config_watch_command,
    ConfigDomainPackCommandBuildOutcome, ConfigDomainPackCommandBuilder,
};
pub use foundation_filesystem_client::{
    filesystem_append_file_command, filesystem_close_handle_command, filesystem_copy_path_command,
    filesystem_create_directory_command, filesystem_create_temp_command,
    filesystem_delete_path_command, filesystem_list_directory_command,
    filesystem_move_path_command, filesystem_open_handle_command, filesystem_read_file_command,
    filesystem_restore_snapshot_command, filesystem_snapshot_tree_command,
    filesystem_stat_path_command, filesystem_watch_path_command, filesystem_write_file_command,
    FilesystemDomainPackCommandBuildOutcome, FilesystemDomainPackCommandBuilder,
};
pub use foundation_random_client::{
    random_bytes_command, random_entropy_health_command, random_integer_command,
    random_nonce_command, random_provider_capabilities_command, random_test_stream_command,
    random_token_command, random_unavailable_diagnostics_command, random_uuid_v4_command,
    RandomDomainPackCommandBuildOutcome, RandomDomainPackCommandBuilder,
};
pub use foundation_time_client::{
    clock_health_command, deadline_evaluation_command, localized_format_command,
    mock_clock_setup_command, monotonic_now_command, monotonic_timeout_command, now_command,
    strict_parse_command, timer_cancel_command, timer_create_command, timezone_conversion_command,
    TimeDomainPackCommandBuildOutcome, TimeDomainPackCommandBuilder,
};
pub use heartbeat_client::{
    ServiceBackedHeartbeatClient, SystemHeartbeatClient, UnavailableSystemHeartbeatClient,
};
pub use interaction_client::{
    ServiceBackedInteractionClient, SystemInteractionClient, UnavailableSystemInteractionClient,
};
pub use llm_client::{
    llm_service_chat_client_from_system, ServiceBackedLlmClient, SystemLlmClient,
    UnavailableSystemLlmClient,
};
pub use macaca_proto::{
    driver_service_descriptor, DriverInventoryCommand, DriverLoadServiceCommand, DriverLoadStatus,
    DriverServiceScope, DriverToolCatalogCommand, DRIVER_SERVICE_ID,
};
pub use macaca_proto::{
    Alert, AlertSeverity, SkillCatalogEntryView, TaskServiceSnapshot, TaskServiceSnapshotCommand,
};
pub use macaca_proto::{
    MemoryCapabilitySet, MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryPolicyHints,
    MemoryPrefetchCommand, MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand,
    MemoryRememberResult, MemoryScope, MemoryServiceSnapshot, MemoryServiceSnapshotCommand,
    MemoryStatusCommand, MemoryStatusReport, MemoryVisibility,
};
pub use mcp_client::{ServiceBackedMcpClient, SystemMcpClient, UnavailableSystemMcpClient};
pub use memory_client::{
    ServiceBackedMemoryClient, SystemMemoryClient, UnavailableSystemMemoryClient,
};
pub use package_fixtures::{
    application_platform_agent_fixture, application_platform_genui_fixture,
    application_platform_headless_fixture, application_platform_plugin_enhanced_fixture,
    application_platform_store_entitled_fixture, application_platform_wasm_skeleton_fixture,
    driver_plugin_fixture, evm_optional_fixture, free_skill_fixture, gateway_plugin_fixture,
    genui_app_fixture, invalid_missing_required_service_fixture, invalid_missing_runtime_fixture,
    paid_skill_fixture, wasm_stub_app_fixture, web3_optional_fixture, yaml_app_fixture,
    ApplicationPlatformFixture, EcosystemPackageFixtureBuilder,
};
pub use payment_client::{
    ServiceBackedPaymentClient, SystemPaymentClient, UnavailableSystemPaymentClient,
};
pub use persona::AgentPersona;
pub use persona_prototype::{PersonaOverrides, PersonaPrototype};
pub use plugin_capability_client::{
    ServiceBackedPluginCapabilityClient, SystemPluginCapabilityClient,
    UnavailableSystemPluginCapabilityClient,
};
pub use plugin_client::{
    ServiceBackedPluginControlClient, SystemPluginControlClient,
    UnavailableSystemPluginControlClient,
};
pub use plugin_hook_client::{
    ServiceBackedPluginHookClient, SystemPluginHookClient, UnavailableSystemPluginHookClient,
};
pub use plugin_sdk::{
    PluginCapabilityBuilder, PluginConfigBuilder, PluginContext, PluginContractDiagnostic,
    PluginContractReport, PluginContractTestKit, PluginHookBuilder, PluginManifestBuilder,
    PluginRegistration, PluginRegistrationBuilder, PluginSdk, PluginSecretRequirementBuilder,
};
pub use scheduled_agent_task_client::{
    ServiceBackedScheduledAgentTaskClient, SystemScheduledAgentTaskClient,
    UnavailableSystemScheduledAgentTaskClient,
};
pub use scheduler_client::{
    ServiceBackedSchedulerClient, SystemSchedulerClient, UnavailableSystemSchedulerClient,
};
pub use service_client::{ServiceCallCommand, ServiceCallResult, ServiceInspectionResult};
pub use spec::{AgentSpec, AgentSpecBuilder, TracePolicy};
pub use store_client::{ServiceBackedStoreClient, SystemStoreClient, UnavailableSystemStoreClient};
pub use system_facade::{
    ApprovalDecisionCommand, EmptySystemPackageClient, EmptySystemTraceClient,
    PackageInspectionCommand, PackageInspectionResult, ServiceBackedTaskBoardDataSource,
    ServiceInspectionCommand, SessionEventQueryCommand, StaticSystemStatusClient, SystemFacade,
    SystemPackageClient, SystemServiceClient, SystemStatusClient, SystemStatusSnapshot,
    SystemTaskClient, SystemTraceClient, TaskBoardDataSource, TaskBoardQueryCommand,
    TaskBoardQueryResult, TraceQueryResult, TraceTailCommand, UnavailableSystemServiceClient,
};
pub use task_client::{TaskServiceClient, UnavailableTaskServiceClient};
pub use tool_client::{ServiceBackedToolClient, SystemToolClient, UnavailableSystemToolClient};
pub use validation::{SdkValidationChain, SdkValidator};
pub use web3_client::{ServiceBackedWeb3Client, SystemWeb3Client, UnavailableSystemWeb3Client};
pub use workbench_client::{
    is_structured_unavailable, ServiceBackedWorkbenchClient, SystemWorkbenchClient,
    SystemWorkbenchFacadeExt, UnavailableWorkbenchServiceClient, WorkbenchClientCatalog,
};
