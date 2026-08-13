//! Stable root-level exports for the domain-pack contract module.
//!
//! The public `macaca_proto::*` surface historically re-exported selected
//! provider-neutral DTOs from private implementation modules. Keeping those
//! exports in this file preserves that API while allowing `mod.rs` to stay as
//! the small module registry required by the OS source-size constitution.

pub use super::catalog::{
    compose_installed_domain_pack_catalog, empty_domain_pack_catalog, snapshot_domain_pack_catalog,
    DomainPackCatalog, DomainPackCatalogSnapshot, InMemoryDomainPackCatalog,
    SharedDomainPackCatalog,
};
pub use super::commerce_cart::{
    cart_stable_hash, commerce_cart_descriptor_hashes, commerce_cart_pack_definition, Cart,
    CartAdjustment, CartArtifactHandle, CartAttribution, CartContext, CartCreateCartCommand,
    CartDescribeSchemaCommand, CartDescriptorHashes, CartDiscountApplication,
    CartDiscountRequestCommand, CartEstimate, CartEstimateCartCommand, CartExportCartCommand,
    CartFreshness, CartGetArtifactHandleCommand, CartHandoffIntent, CartHandoffRequestCommand,
    CartInspectAbandonmentCommand, CartInspectProviderCommand, CartLine, CartLineRequestCommand,
    CartPlanContextUpdateCommand, CartPlanDiscountCommand, CartPlanExportCommand,
    CartPlanHandoffCommand, CartPlanLineMutationCommand, CartProviderCapability,
    CartReadCartCommand, CartRedactionPolicy, CartResultEnvelope, CartResultStatus, CartScope,
    CartSearchCartsCommand, CartTotals, CartUpdateContextCommand, CartValidateCartCommand,
    CartValidationIssue, COMMERCE_CART_COMMANDS, COMMERCE_CART_PACK_ID, COMMERCE_CART_SERVICE_ID,
};
pub use super::commerce_catalog::{
    catalog_stable_hash, commerce_catalog_descriptor_hashes, commerce_catalog_pack_definition,
    AvailabilitySnapshot, CatalogArtifactHandle, CatalogAttribute, CatalogAttribution,
    CatalogChannel, CatalogCheckAvailabilityCommand, CatalogDescribeSchemaCommand,
    CatalogDescriptorHashes, CatalogExportCatalogCommand, CatalogFreshness,
    CatalogGetArtifactHandleCommand, CatalogGetPriceCommand, CatalogGetProductCommand,
    CatalogGetVariantCommand, CatalogInspectProviderCommand, CatalogListProductsCommand,
    CatalogListTaxonomyCommand, CatalogListVariantsCommand, CatalogMediaRequestCommand,
    CatalogModifier, CatalogMutationPlan, CatalogMutationResult, CatalogOption,
    CatalogPlanExportCommand, CatalogPlanMediaMutationCommand, CatalogPlanProductMutationCommand,
    CatalogPlanVariantMutationCommand, CatalogPrice, CatalogProduct, CatalogProductRequestCommand,
    CatalogProjection, CatalogProviderCapability, CatalogPublicationScope, CatalogRedactionPolicy,
    CatalogResultEnvelope, CatalogResultStatus, CatalogScope, CatalogSearchCatalogCommand,
    CatalogSearchRequest, CatalogSearchResult, CatalogTaxonomyNode, CatalogVariant,
    CatalogVariantRequestCommand, PriceBook, PriceContext, COMMERCE_CATALOG_COMMANDS,
    COMMERCE_CATALOG_PACK_ID, COMMERCE_CATALOG_SERVICE_ID,
};
pub use super::commerce_common::{
    CommercePackCommandEnvelope, CommercePackError, CommercePackPage,
};
pub use super::commerce_entitlement::{
    commerce_entitlement_descriptor_hashes, commerce_entitlement_pack_definition,
    entitlement_stable_hash, CommerceEntitlementAssignSeatCommand,
    CommerceEntitlementBatchCheckCommand, CommerceEntitlementCheckCommand,
    CommerceEntitlementDescribeSchemaCommand, CommerceEntitlementGetArtifactHandleCommand,
    CommerceEntitlementGetUsageBalanceCommand, CommerceEntitlementGrantCommand,
    CommerceEntitlementInspectProviderCommand, CommerceEntitlementPlanGrantCommand,
    CommerceEntitlementPlanProofExportCommand, CommerceEntitlementPlanResumeCommand,
    CommerceEntitlementPlanRevokeCommand, CommerceEntitlementPlanSuspendCommand,
    CommerceEntitlementPlanTransferCommand, CommerceEntitlementProofExportRequestCommand,
    CommerceEntitlementRecordEventReferenceCommand, CommerceEntitlementRecordUsageCommand,
    CommerceEntitlementReleaseSeatCommand, CommerceEntitlementResumeCommand,
    CommerceEntitlementRevokeCommand, CommerceEntitlementState, CommerceEntitlementSuspendCommand,
    CommerceEntitlementSyncSourceCommand, CommerceEntitlementTransferCommand,
    EntitlementArtifactHandle, EntitlementAttribution, EntitlementDescriptorHashes,
    EntitlementDimension, EntitlementEventReference, EntitlementFreshness, EntitlementGrant,
    EntitlementProofExportPlan, EntitlementProviderCapability, EntitlementRedactionPolicy,
    EntitlementResource, EntitlementResultEnvelope, EntitlementResultStatus, EntitlementScope,
    EntitlementSeatAssignment, EntitlementSourceEvidence, EntitlementSubject,
    EntitlementUsageBalance, EntitlementUsageRecord, COMMERCE_ENTITLEMENT_COMMANDS,
    COMMERCE_ENTITLEMENT_PACK_ID, COMMERCE_ENTITLEMENT_SERVICE_ID,
};
pub use super::commerce_order::{
    commerce_order_descriptor_hashes, commerce_order_pack_definition, order_stable_hash,
    FulfillmentIntent, FulfillmentStatusReference, OrderAdjustment, OrderArtifactHandle,
    OrderAttribution, OrderAuditExportPlan, OrderAuditExportRequestCommand,
    OrderCancelOrderCommand, OrderCancellationPlan, OrderCancellationResult,
    OrderCreateOrderCommand, OrderDescribeSchemaCommand, OrderDescriptorHashes, OrderFreshness,
    OrderFulfillmentIntentRequestCommand, OrderGetArtifactHandleCommand,
    OrderInspectProviderCommand, OrderLifecycleState, OrderLifecycleTransitionPlan, OrderLine,
    OrderListReturnReferencesCommand, OrderPlanAuditExportCommand, OrderPlanCancellationCommand,
    OrderPlanFulfillmentIntentCommand, OrderPlanOrderCommand, OrderPlanStateTransitionCommand,
    OrderProviderCapability, OrderReadOrderCommand, OrderRecord, OrderRedactionPolicy,
    OrderResultEnvelope, OrderResultStatus, OrderReturnReference, OrderScope,
    OrderSearchOrdersCommand, OrderStateTransitionRequestCommand, OrderSyncStatusCommand,
    OrderTotals, COMMERCE_ORDER_COMMANDS, COMMERCE_ORDER_PACK_ID, COMMERCE_ORDER_SERVICE_ID,
};
pub use super::commerce_payment_intent::{
    commerce_payment_intent_descriptor_hashes, commerce_payment_intent_pack_definition,
    payment_intent_stable_hash, PaymentActionRequirement, PaymentAuthorization,
    PaymentCancellation, PaymentCapture, PaymentIntentArtifactHandle, PaymentIntentAttribution,
    PaymentIntentAuditExportPlan, PaymentIntentAuditExportRequestCommand,
    PaymentIntentCancelCommand, PaymentIntentCaptureCommand, PaymentIntentConfirmCommand,
    PaymentIntentCreateIntentCommand, PaymentIntentDescribeSchemaCommand,
    PaymentIntentDescriptorHashes, PaymentIntentEventReference, PaymentIntentFreshness,
    PaymentIntentGetArtifactHandleCommand, PaymentIntentGetStatusCommand,
    PaymentIntentInspectActionCommand, PaymentIntentInspectIdempotencyCommand,
    PaymentIntentInspectProviderCommand, PaymentIntentPlan, PaymentIntentPlanAuditExportCommand,
    PaymentIntentPlanCancellationCommand, PaymentIntentPlanCaptureCommand,
    PaymentIntentPlanConfirmationCommand, PaymentIntentPlanIntentCommand,
    PaymentIntentProviderCapability, PaymentIntentRecord, PaymentIntentRecordEventReferenceCommand,
    PaymentIntentRedactionPolicy, PaymentIntentResultEnvelope, PaymentIntentResultStatus,
    PaymentIntentScope, PaymentMethodReference, COMMERCE_PAYMENT_INTENT_COMMANDS,
    COMMERCE_PAYMENT_INTENT_PACK_ID, COMMERCE_PAYMENT_INTENT_SERVICE_ID,
};
pub use super::commerce_receipt::{
    commerce_receipt_descriptor_hashes, commerce_receipt_pack_definition, receipt_stable_hash,
    ReceiptAdjustment, ReceiptArtifactHandle, ReceiptAttribution, ReceiptAudience,
    ReceiptAuditExportPlan, ReceiptCorrectionReference, ReceiptDeliveryRequest,
    ReceiptDeliveryState, ReceiptDescribeSchemaCommand, ReceiptDescriptorHashes,
    ReceiptEventReference, ReceiptFreshness, ReceiptGetArtifactHandleCommand,
    ReceiptGetDeliveryStatusCommand, ReceiptInspectProviderCommand, ReceiptIssueReceiptCommand,
    ReceiptLine, ReceiptLinkCorrectionReferenceCommand, ReceiptListCorrectionReferencesCommand,
    ReceiptPlanAuditExportCommand, ReceiptPlanDeliveryCommand, ReceiptPlanIssueCommand,
    ReceiptPlanReissueCommand, ReceiptProviderCapability, ReceiptReadReceiptCommand, ReceiptRecord,
    ReceiptRecordEventReferenceCommand, ReceiptRedactionPolicy, ReceiptReissueReceiptCommand,
    ReceiptResultEnvelope, ReceiptResultStatus, ReceiptScope, ReceiptSearchReceiptsCommand,
    ReceiptSourceReference, ReceiptSyncSourceCommand, ReceiptTotals, ReceiptVariant,
    ReceiptVerificationResult, ReceiptVerifyReceiptCommand, COMMERCE_RECEIPT_COMMANDS,
    COMMERCE_RECEIPT_PACK_ID, COMMERCE_RECEIPT_SERVICE_ID,
};
pub use super::communication_calendar::{
    calendar_stable_hash, communication_calendar_descriptor_hashes,
    communication_calendar_pack_definition, CalendarAttendee, CalendarAvailabilityQuery,
    CalendarCheckAvailabilityCommand, CalendarConference, CalendarConflict,
    CalendarCreateEventCommand, CalendarCursor, CalendarDeleteEventCommand,
    CalendarDescriptorHashes, CalendarError, CalendarEvent, CalendarExportIcalendarCommand,
    CalendarGetEventCommand, CalendarImportIcalendarCommand, CalendarInspectConflictsCommand,
    CalendarInstance, CalendarListCalendarsCommand, CalendarManageConferenceCommand,
    CalendarProposeTimesCommand, CalendarProviderCapability, CalendarProviderSnapshot,
    CalendarQueryEventsCommand, CalendarRecurrence, CalendarRegisterWatchCommand, CalendarReminder,
    CalendarRespondInviteCommand, CalendarResultEnvelope, CalendarResultStatus,
    CalendarSetReminderCommand, CalendarSource, CalendarSyncEventsCommand,
    CalendarUpdateEventCommand, CalendarWatch, COMMUNICATION_CALENDAR_COMMANDS,
    COMMUNICATION_CALENDAR_PACK_ID, COMMUNICATION_CALENDAR_SERVICE_ID,
};
pub use super::communication_email::{
    communication_email_descriptor_hashes, communication_email_pack_definition, email_stable_hash,
    EmailApplyLabelsCommand, EmailAttachmentRef, EmailBodyKind, EmailBodyPart,
    EmailCancelScheduledSendCommand, EmailComposeCommand, EmailConsentStatus, EmailDeliveryState,
    EmailDeliveryStatusCommand, EmailDescriptorHashes, EmailDraftRef, EmailError,
    EmailFetchAttachmentCommand, EmailFetchMessageCommand, EmailIngestEventCommand,
    EmailListThreadsCommand, EmailMarkReadCommand, EmailMessageRef, EmailProviderCapability,
    EmailProviderEventRef, EmailProviderSnapshot, EmailRateLimitStatus, EmailRecipient,
    EmailRecipientKind, EmailResultEnvelope, EmailResultStatus, EmailSaveDraftCommand,
    EmailScheduleSendCommand, EmailSendCommand, EmailSenderRef, EmailSyncCursor,
    EmailSyncMailboxCommand, EmailUpdateDraftCommand, EmailValidateRecipientsCommand,
    COMMUNICATION_EMAIL_COMMANDS, COMMUNICATION_EMAIL_PACK_ID, COMMUNICATION_EMAIL_SERVICE_ID,
};
pub use super::communication_email_preflight::{EmailPackDeclaration, EmailPackDeclarationSpec};
pub use super::communication_inbox::{
    communication_inbox_descriptor_hashes, communication_inbox_pack_definition, inbox_stable_hash,
    InboxArchiveItemCommand, InboxAttachmentHandle, InboxClaim, InboxClaimItemCommand, InboxCursor,
    InboxDescriptorHashes, InboxError, InboxEvent, InboxFetchAttachmentCommand,
    InboxFetchBodyCommand, InboxGetItemCommand, InboxIngestEventCommand, InboxItem, InboxLabel,
    InboxLabelItemCommand, InboxListItemsCommand, InboxListThreadsCommand, InboxMarkReadCommand,
    InboxMoveItemCommand, InboxProviderCapability, InboxProviderSnapshot,
    InboxRegisterSourceCommand, InboxReleaseItemCommand, InboxResultEnvelope, InboxResultStatus,
    InboxResumeSyncCommand, InboxRevokeSourceCommand, InboxSearchItemsCommand, InboxSource,
    InboxSummarizeItemCommand, InboxSyncCheckpoint, InboxSyncSourcesCommand, InboxThread,
    InboxUpdateSourceCommand, COMMUNICATION_INBOX_COMMANDS, COMMUNICATION_INBOX_PACK_ID,
    COMMUNICATION_INBOX_SERVICE_ID,
};
pub use super::communication_messaging::{
    communication_messaging_descriptor_hashes, communication_messaging_pack_definition,
    messaging_stable_hash, MessagingAttachHandleCommand, MessagingAttachmentRef, MessagingContent,
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
    COMMUNICATION_MESSAGING_COMMANDS, COMMUNICATION_MESSAGING_PACK_ID,
    COMMUNICATION_MESSAGING_SERVICE_ID,
};
pub use super::communication_messaging_preflight::{
    MessagingPackDeclaration, MessagingPackDeclarationSpec,
};
pub use super::communication_notification::{
    communication_notification_descriptor_hashes, communication_notification_pack_definition,
    notification_stable_hash, NotificationAcknowledgeCommand, NotificationActionDefinition,
    NotificationActionEvent, NotificationCancelCommand, NotificationDeliveryChannel,
    NotificationDeliveryHandle, NotificationDeliveryStatus, NotificationDescriptorHashes,
    NotificationDismissCommand, NotificationError, NotificationInspectDeliveryCommand,
    NotificationListNotificationsCommand, NotificationMessage, NotificationProviderCapability,
    NotificationProviderSnapshot, NotificationPublishCommand, NotificationRegisterActionCommand,
    NotificationRegisterSubscriptionCommand, NotificationResultEnvelope, NotificationResultStatus,
    NotificationRevokeSubscriptionCommand, NotificationSchedule, NotificationScheduleCommand,
    NotificationSubscriptionHandle, NotificationTarget, NotificationUnregisterActionCommand,
    NotificationUpdateCommand, COMMUNICATION_NOTIFICATION_COMMANDS,
    COMMUNICATION_NOTIFICATION_PACK_ID, COMMUNICATION_NOTIFICATION_SERVICE_ID,
};
pub use super::expansion::{
    expand_service_capabilities, DomainPackEffectiveCapabilityProjection,
    EffectiveServiceCapabilities,
};
pub use super::finance_common::{FinanceCommandEnvelope, FinanceError, FinancePage};
pub use super::foundation_config::{
    config_stable_hash, foundation_config_descriptor_hashes, foundation_config_pack_definition,
    ConfigDescribeSchemaCommand, ConfigDescriptorHashes, ConfigError,
    ConfigExplainProvenanceCommand, ConfigExportRedactedCommand, ConfigGetCommand,
    ConfigGetManyCommand, ConfigKeyReference, ConfigLayerReference, ConfigListKeysCommand,
    ConfigProvenance, ConfigProviderCapability, ConfigProviderSnapshot, ConfigRedactionSummary,
    ConfigReloadCommand, ConfigResolveEffectiveCommand, ConfigResultEnvelope, ConfigResultStatus,
    ConfigSchemaReference, ConfigSelector, ConfigSnapshotCommand, ConfigSourceReference,
    ConfigTypedValueRef, ConfigValidateCommand, ConfigValidationReport, ConfigValueKind,
    ConfigWatchCommand, ConfigWatchEvent, FOUNDATION_CONFIG_COMMANDS, FOUNDATION_CONFIG_PACK_ID,
    FOUNDATION_CONFIG_SERVICE_ID,
};
pub use super::foundation_filesystem::{
    filesystem_stable_hash, foundation_filesystem_descriptor_hashes,
    foundation_filesystem_pack_definition, FilesystemAccessMode, FilesystemAppendFileCommand,
    FilesystemCloseHandleCommand, FilesystemConflictMode, FilesystemContentRef,
    FilesystemCopyPathCommand, FilesystemCreateDirectoryCommand, FilesystemCreateTempCommand,
    FilesystemDeletePathCommand, FilesystemDescriptorHashes, FilesystemError, FilesystemHandleRef,
    FilesystemListDirectoryCommand, FilesystemMetadata, FilesystemMovePathCommand,
    FilesystemOpenHandleCommand, FilesystemPathRef, FilesystemProviderCapability,
    FilesystemProviderSnapshot, FilesystemReadFileCommand, FilesystemRestoreSnapshotCommand,
    FilesystemResultEnvelope, FilesystemResultStatus, FilesystemRootRef, FilesystemSnapshotRef,
    FilesystemSnapshotTreeCommand, FilesystemStatPathCommand, FilesystemWatchEvent,
    FilesystemWatchPathCommand, FilesystemWriteFileCommand, FOUNDATION_FILESYSTEM_COMMANDS,
    FOUNDATION_FILESYSTEM_PACK_ID, FOUNDATION_FILESYSTEM_SERVICE_ID,
};
pub use super::foundation_key_value_state::{
    foundation_key_value_state_descriptor_hashes, foundation_key_value_state_pack_definition,
    key_value_state_stable_hash, KeyValueBatchDeleteCommand, KeyValueBatchGetCommand,
    KeyValueBatchPutCommand, KeyValueCompactNamespaceCommand, KeyValueCompareAndSetCommand,
    KeyValueConflictMode, KeyValueConsistencyLevel, KeyValueDeleteCommand, KeyValueExistsCommand,
    KeyValueGetCommand, KeyValueGetTtlCommand, KeyValueIncrementCommand, KeyValueKeyRef,
    KeyValueListKeysCommand, KeyValueMigrateNamespaceCommand, KeyValueNamespaceRef,
    KeyValuePutCommand, KeyValueRestoreNamespaceCommand, KeyValueRevision, KeyValueSetTtlCommand,
    KeyValueSnapshotNamespaceCommand, KeyValueSnapshotRef, KeyValueStateDescriptorHashes,
    KeyValueStateError, KeyValueStateProviderCapability, KeyValueStateProviderSnapshot,
    KeyValueStateResultEnvelope, KeyValueStateResultStatus, KeyValueTtlPolicy,
    KeyValueTypedValueRef, KeyValueWatchEvent, KeyValueWatchNamespaceCommand,
    FOUNDATION_KEY_VALUE_STATE_COMMANDS, FOUNDATION_KEY_VALUE_STATE_PACK_ID,
    FOUNDATION_KEY_VALUE_STATE_SERVICE_ID,
};
pub use super::foundation_random::{
    foundation_random_descriptor_hashes, foundation_random_pack_definition, random_stable_hash,
    RandomAlphabetClass, RandomBytesCommand, RandomDescriptorHashes, RandomEntropyHealth,
    RandomEntropyHealthCommand, RandomError, RandomFillCommand, RandomIntegerCommand,
    RandomNonceCommand, RandomOutputEncoding, RandomProviderCapabilitiesCommand,
    RandomProviderCapability, RandomProviderSnapshot, RandomPurpose, RandomReplayPolicy,
    RandomResultEnvelope, RandomResultStatus, RandomSeedReference, RandomStreamReference,
    RandomStrengthClass, RandomTestStreamBytesCommand, RandomTestStreamCreateCommand,
    RandomTokenCommand, RandomUuidV4Command, FOUNDATION_RANDOM_COMMANDS, FOUNDATION_RANDOM_PACK_ID,
    FOUNDATION_RANDOM_SERVICE_ID,
};
pub use super::foundation_secrets_reference::{
    foundation_secrets_reference_descriptor_hashes, foundation_secrets_reference_pack_definition,
    secrets_reference_stable_hash, SecretAccessPolicy, SecretAuditRecord, SecretExternalLocator,
    SecretLeaseReference, SecretPurposeBinding, SecretReference, SecretResolutionHandle,
    SecretVersionState, SecretVersionStatus, SecretsAuditAccessCommand, SecretsBindPurposeCommand,
    SecretsCreateLeaseCommand, SecretsCreateReferenceCommand, SecretsImportReferenceCommand,
    SecretsInspectReferenceCommand, SecretsListReferencesCommand, SecretsReferenceDescriptorHashes,
    SecretsReferenceError, SecretsReferenceProviderCapability, SecretsReferenceProviderSnapshot,
    SecretsReferenceResultEnvelope, SecretsReferenceResultStatus, SecretsRenewLeaseCommand,
    SecretsResolveForProviderCommand, SecretsRevokeLeaseCommand, SecretsRotateReferenceCommand,
    SecretsVersionStatusCommand, FOUNDATION_SECRETS_REFERENCE_COMMANDS,
    FOUNDATION_SECRETS_REFERENCE_PACK_ID, FOUNDATION_SECRETS_REFERENCE_SERVICE_ID,
};
pub use super::foundation_session_state::{
    foundation_session_state_descriptor_hashes, foundation_session_state_pack_definition,
    session_state_stable_hash, SessionStateCheckpointRef, SessionStateClearSessionCommand,
    SessionStateCompactHistoryCommand, SessionStateCompareCheckpointCommand,
    SessionStateCreateCheckpointCommand, SessionStateDeleteCommand, SessionStateDescriptorHashes,
    SessionStateError, SessionStateExportRedactedCommand, SessionStateGetCommand,
    SessionStateInspectRecoveryCommand, SessionStateKeyRef, SessionStateListCheckpointsCommand,
    SessionStateListKeysCommand, SessionStateMergePatchCommand, SessionStateProviderCapability,
    SessionStateProviderSnapshot, SessionStatePutCommand, SessionStateRecoveryMetadata,
    SessionStateRedactionSummary, SessionStateRestoreCheckpointCommand, SessionStateRestorePlan,
    SessionStateResultEnvelope, SessionStateResultStatus, SessionStateRetentionPolicy,
    SessionStateRevision, SessionStateSessionRef, SessionStateValueRef,
    FOUNDATION_SESSION_STATE_COMMANDS, FOUNDATION_SESSION_STATE_PACK_ID,
    FOUNDATION_SESSION_STATE_SERVICE_ID,
};
pub use super::foundation_time::{
    foundation_time_descriptor_hashes, foundation_time_pack_definition, time_stable_hash,
    TimeAddDurationCommand, TimeCalendarConvertCommand, TimeCalendarReference,
    TimeCancelTimerCommand, TimeClockHealth, TimeClockHealthCommand, TimeClockSource,
    TimeConvertTimezoneCommand, TimeCreateTimerCommand, TimeDeadlineSpec, TimeDescriptorHashes,
    TimeDuration, TimeDurationBetweenCommand, TimeError, TimeEvaluateDeadlineCommand,
    TimeExactnessHint, TimeFormatCommand, TimeFormatSpec, TimeInspectTimerCommand, TimeInstant,
    TimeLocaleReference, TimeMonotonicInstant, TimeMonotonicNowCommand, TimeNowCommand,
    TimeParseCommand, TimeProviderCapability, TimeProviderSnapshot, TimeResolveTimezoneCommand,
    TimeResultEnvelope, TimeResultStatus, TimeTimerReference, TimeZoneReference,
    FOUNDATION_TIME_COMMANDS, FOUNDATION_TIME_PACK_ID, FOUNDATION_TIME_SERVICE_ID,
};
pub use super::industrial_reference_catalogs::industrial_reference_domain_pack_definitions;
pub use super::knowledge_citations::{
    citations_stable_hash, knowledge_citations_descriptor_hashes,
    knowledge_citations_pack_definition, BibliographyStyle, CitationContributor,
    CitationDescriptorHashes, CitationEvidence, CitationExportResult, CitationIdentifier,
    CitationImportResult, CitationItem, CitationProviderCapability, CitationResultEnvelope,
    CitationResultStatus, CitationSelector, CitationSourceAnchor, CitationVerificationResult,
    CitationsCreateCitationCommand, CitationsExportCitationsCommand,
    CitationsFormatBibliographyCommand, CitationsFormatCitationCommand,
    CitationsImportCitationsCommand, CitationsInspectProviderCommand,
    CitationsInspectSourceAnchorCommand, CitationsLinkSourceSpanCommand,
    CitationsListCitationsCommand, CitationsResolveIdentifierCommand,
    CitationsUpdateCitationCommand, CitationsVerifyCitationCommand, FormattedCitation,
    KNOWLEDGE_CITATIONS_COMMANDS, KNOWLEDGE_CITATIONS_PACK_ID, KNOWLEDGE_CITATIONS_SERVICE_ID,
};
pub use super::knowledge_citations_preflight::{
    CitationAdmissionEvidence, CitationDispatchPreflight,
};
pub use super::knowledge_common::{KnowledgeCommandEnvelope, KnowledgeError, KnowledgePage};
pub use super::knowledge_document_parsing::{
    document_parsing_stable_hash, knowledge_document_parsing_descriptor_hashes,
    knowledge_document_parsing_pack_definition, DocumentChunk, DocumentConfidence, DocumentElement,
    DocumentEmbeddedResource, DocumentEntity, DocumentFormField, DocumentGeometry,
    DocumentMetadata, DocumentOcrToken, DocumentPage, DocumentParseResult,
    DocumentParserCapability, DocumentParsingCancelParseJobCommand,
    DocumentParsingChunkDocumentCommand, DocumentParsingConvertToCanonicalCommand,
    DocumentParsingDescriptorHashes, DocumentParsingDetectFormatCommand,
    DocumentParsingExtractFormsCommand, DocumentParsingExtractLayoutCommand,
    DocumentParsingExtractMetadataCommand, DocumentParsingExtractTablesCommand,
    DocumentParsingExtractTextCommand, DocumentParsingGetParseJobCommand,
    DocumentParsingInspectParserCommand, DocumentParsingParseDocumentCommand,
    DocumentParsingResultEnvelope, DocumentParsingResultStatus,
    DocumentParsingStartParseJobCommand, DocumentParsingValidateDocumentCommand, DocumentSource,
    DocumentTable, DocumentTableCell, DocumentTextSpan, ParseJob, ParserProfile,
    KNOWLEDGE_DOCUMENT_PARSING_COMMANDS, KNOWLEDGE_DOCUMENT_PARSING_PACK_ID,
    KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID,
};
pub use super::knowledge_document_parsing_preflight::{
    DocumentParsingAdmissionEvidence, DocumentParsingDispatchPreflight,
};
pub use super::knowledge_graph::{
    graph_stable_hash, knowledge_graph_descriptor_hashes, knowledge_graph_pack_definition,
    GraphDeleteGraphItemsCommand, GraphDeleteTriplesCommand, GraphDescriptorHashes, GraphEdge,
    GraphExportPlan, GraphExportSubgraphCommand, GraphFindPathCommand, GraphImportPlan,
    GraphImportSubgraphCommand, GraphInspectProvenanceCommand, GraphInspectProviderCommand,
    GraphInspectStoreCommand, GraphMergeEntitiesCommand, GraphNode, GraphPath, GraphProperty,
    GraphProvenance, GraphProviderCapability, GraphQuery, GraphQueryCommand, GraphQueryResult,
    GraphRegisterStoreCommand, GraphResultEnvelope, GraphResultStatus, GraphSchema, GraphStore,
    GraphTraversal, GraphTraverseCommand, GraphUpsertEdgeCommand, GraphUpsertNodeCommand,
    GraphUpsertSchemaCommand, GraphUpsertTripleCommand, GraphValidateQueryCommand,
    GraphValidateSchemaCommand, RdfStatement, RdfTerm, KNOWLEDGE_GRAPH_COMMANDS,
    KNOWLEDGE_GRAPH_PACK_ID, KNOWLEDGE_GRAPH_SERVICE_ID,
};
pub use super::knowledge_retrieval::{
    knowledge_retrieval_descriptor_hashes, knowledge_retrieval_pack_definition,
    retrieval_stable_hash, RetrievalBulkRetrieveCommand, RetrievalCandidate, RetrievalChunk,
    RetrievalCollection, RetrievalCursor, RetrievalDeleteRecordsCommand, RetrievalDescriptorHashes,
    RetrievalEvidenceBundle, RetrievalExpandContextCommand, RetrievalFreshness,
    RetrievalFusionStrategy, RetrievalInspectCollectionCommand, RetrievalInspectRecordCommand,
    RetrievalMetadataFilter, RetrievalNamespace, RetrievalPackageEvidenceCommand,
    RetrievalProviderCapability, RetrievalQuery, RetrievalQueryDiagnosticsCommand,
    RetrievalRangeRetrieveCommand, RetrievalRecord, RetrievalRefreshCollectionCommand,
    RetrievalRegisterCollectionCommand, RetrievalRerankContextCommand, RetrievalResultEnvelope,
    RetrievalResultStatus, RetrievalRetrieveByIdCommand, RetrievalRetrieveCommand,
    RetrievalUpsertRecordsCommand, RetrievalVectorSpace, KNOWLEDGE_RETRIEVAL_COMMANDS,
    KNOWLEDGE_RETRIEVAL_PACK_ID, KNOWLEDGE_RETRIEVAL_SERVICE_ID,
};
pub use super::knowledge_retrieval_preflight::{
    RetrievalAdmissionEvidence, RetrievalDispatchPreflight,
};
pub use super::knowledge_search::{
    knowledge_search_descriptor_hashes, knowledge_search_pack_definition, search_stable_hash,
    SearchAnalyzerProfile, SearchAutocompleteCommand, SearchCorpus, SearchCursor,
    SearchDescriptorHashes, SearchExplainRankingCommand, SearchFacetRequest, SearchFacetsCommand,
    SearchField, SearchFilter, SearchHit, SearchIndexSchema, SearchIndexStatsCommand,
    SearchInspectIndexCommand, SearchProviderCapability, SearchQuery,
    SearchQueryDiagnosticsCommand, SearchRankingExplanation, SearchRankingProfile,
    SearchRefreshIndexCommand, SearchRegisterCorpusCommand, SearchResultEnvelope,
    SearchResultStatus, SearchSearchCommand, SearchSort, SearchSuggestCommand, SearchSynonymSet,
    KNOWLEDGE_SEARCH_COMMANDS, KNOWLEDGE_SEARCH_PACK_ID, KNOWLEDGE_SEARCH_SERVICE_ID,
};
pub use super::knowledge_search_preflight::{SearchAdmissionEvidence, SearchDispatchPreflight};
pub use super::knowledge_summarization::{
    knowledge_summarization_descriptor_hashes, knowledge_summarization_pack_definition,
    summarization_stable_hash, CompressionMap, SummarizationCompareSummariesCommand,
    SummarizationCompressContextCommand, SummarizationDescriptorHashes,
    SummarizationEvaluateSummaryCommand, SummarizationInspectProviderCommand,
    SummarizationInspectSummaryEvidenceCommand, SummarizationPlanCommand,
    SummarizationRefineSummaryCommand, SummarizationResultEnvelope, SummarizationResultStatus,
    SummarizationSummarizeCommand, SummarizationSummarizeConversationCommand,
    SummarizationSummarizeManyCommand, SummarizationSummarizeWithCitationsCommand,
    SummarizationValidateRequestCommand, SummaryClaim, SummaryComparisonReport,
    SummaryEvidenceLink, SummaryOutput, SummaryPlan, SummaryProviderCapability,
    SummaryQualityReport, SummaryRequest, SummarySource, KNOWLEDGE_SUMMARIZATION_COMMANDS,
    KNOWLEDGE_SUMMARIZATION_PACK_ID, KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
};
pub use super::knowledge_summarization_preflight::{
    SummarizationAdmissionEvidence, SummarizationDispatchPreflight,
};
pub use super::media_common::{MediaCommandEnvelope, MediaError, MediaPage};
pub use super::model::{
    AppPackPolicyOverride, AppServiceContractConfig, AppServicePolicyOverride,
    DomainPackAvailability, DomainPackCompatibility, DomainPackDataGovernance,
    DomainPackDefinition, DomainPackDiagnostics, DomainPackMetadata, DomainPackPolicyTemplate,
    DomainPackProviderCapabilityReport, DomainPackProviderCapabilityState,
    DomainPackProviderDescriptor, DomainPackProviderSnapshot, DomainPackSdkMetadata,
    DomainPackStability, DomainPackUnavailableDiagnostic,
};
pub use super::reference_catalogs::{
    developer_pack_definition, foundation_pack_definition, knowledge_pack_definition,
    reference_domain_pack_definitions,
};
pub use super::service_helpers::{
    domain_pack_command_trace, domain_pack_service_adapter_error, domain_pack_service_result,
};
pub use super::spec::{
    validate_domain_pack_family_id, validate_domain_pack_id, validate_domain_pack_parent,
    validate_domain_pack_version, AppServiceContractSpec, DomainPackCallableSpec,
    DomainPackDefinitionSpec, DomainPackHierarchySpec, DomainPackIdentitySpec,
};
pub use super::workflow_approval_semantics::{
    check_idempotency, filtered_pending_page, ApprovalAssignmentV1, ApprovalConsumptionMode,
    ApprovalDeadlineSpec, ApprovalDecisionGateV1, ApprovalDecisionV1, ApprovalEligibilityEvidence,
    ApprovalEligibilitySpec, ApprovalEvidenceBundleV1, ApprovalIdempotencyResult,
    ApprovalLifecycleSpec, ApprovalLifecycleState, ApprovalPendingProjection, ApprovalRequestV1,
};
