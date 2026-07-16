use super::communication_common::{bounded_communication_token, optional_secret_reference_is_safe};
use super::communication_notification::{
    NotificationActionDefinition, NotificationActionEvent, NotificationDeliveryHandle,
    NotificationListNotificationsCommand, NotificationMessage, NotificationPublishCommand,
    NotificationRegisterActionCommand, NotificationRegisterSubscriptionCommand,
    NotificationSchedule, NotificationScheduleCommand, NotificationSubscriptionHandle,
    NotificationTarget,
};

impl NotificationMessage {
    /// Validate notification content as bounded title/body references.
    pub fn is_reference_only(&self) -> bool {
        bounded_communication_token(&self.title_ref, 256)
            && bounded_communication_token(&self.body_ref, 256)
            && self
                .locale
                .as_deref()
                .is_none_or(|locale| bounded_communication_token(locale, 32))
            && matches!(
                self.sensitivity.as_str(),
                "public" | "private" | "sensitive"
            )
            && self
                .category_id
                .as_deref()
                .is_none_or(|category| bounded_communication_token(category, 160))
            && self
                .collapse_key
                .as_deref()
                .is_none_or(|key| bounded_communication_token(key, 160))
    }
}

impl NotificationTarget {
    /// Validate targets as redacted handles without raw push endpoints.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.target_id, 160)
            && matches!(
                self.target_kind.as_str(),
                "user" | "device" | "agent" | "topic"
            )
            && self
                .subscription
                .as_ref()
                .is_none_or(NotificationSubscriptionHandle::is_safe_reference)
            && bounded_communication_token(&self.redaction_label, 96)
    }
}

impl NotificationSchedule {
    /// Validate notification schedule and expiry metadata.
    pub fn is_valid_window(&self) -> bool {
        (self.deliver_at_epoch_ms.is_some() ^ self.relative_delay_ms.is_some())
            && self.deliver_at_epoch_ms.is_none_or(|value| value > 0)
            && self.relative_delay_ms.is_none_or(|delay| delay > 0)
            && self
                .timezone_id
                .as_deref()
                .is_none_or(|timezone| bounded_communication_token(timezone, 96))
            && self.expiry_epoch_ms.is_none_or(|expiry| expiry > 0)
    }
}

impl NotificationActionDefinition {
    /// Validate action metadata as host callback semantics, not app-specific workflows.
    pub fn is_safe_definition(&self) -> bool {
        bounded_communication_token(&self.action_id, 160)
            && bounded_communication_token(&self.title_ref, 256)
            && matches!(
                self.semantic_role.as_str(),
                "open" | "dismiss" | "reply" | "custom"
            )
    }
}

impl NotificationActionEvent {
    /// Validate action events with bounded input references and replay evidence.
    pub fn is_safe_reference(&self) -> bool {
        self.delivery.is_safe_reference()
            && bounded_communication_token(&self.action_id, 160)
            && self
                .bounded_input_ref
                .as_deref()
                .is_none_or(|input| bounded_communication_token(input, 256))
            && bounded_communication_token(&self.replay_ref, 256)
    }
}

impl NotificationSubscriptionHandle {
    /// Validate subscription handles and require provider secrets through secret references only.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.subscription_id, 160)
            && bounded_communication_token(&self.target_class, 96)
            && optional_secret_reference_is_safe(self.secret_ref.as_deref())
            && bounded_communication_token(&self.provider_class, 96)
    }
}

impl NotificationDeliveryHandle {
    /// Validate delivery handles without provider-native receipt payloads.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.delivery_id, 160)
            && bounded_communication_token(&self.provider_class, 96)
    }
}

impl NotificationPublishCommand {
    /// Validate publish requests before host or remote notification providers run.
    pub fn has_admission_preconditions(&self) -> bool {
        self.message.is_reference_only()
            && self.target.is_safe_reference()
            && bounded_communication_token(&self.client_request_id, 128)
    }
}

impl NotificationScheduleCommand {
    /// Validate scheduled notification requests with bounded schedule metadata.
    pub fn has_schedule_preconditions(&self) -> bool {
        self.publish.has_admission_preconditions() && self.schedule.is_valid_window()
    }
}

impl NotificationListNotificationsCommand {
    /// Validate list pagination and target scope.
    pub fn has_bounded_pagination(&self, max_page_size: u32) -> bool {
        self.target.is_safe_reference() && self.page_size > 0 && self.page_size <= max_page_size
    }
}

impl NotificationRegisterActionCommand {
    /// Validate action registration uses bounded callback route handles.
    pub fn has_registration_preconditions(&self) -> bool {
        bounded_communication_token(&self.category_id, 160)
            && self.action.is_safe_definition()
            && bounded_communication_token(&self.callback_route, 256)
    }
}

impl NotificationRegisterSubscriptionCommand {
    /// Validate subscription registration never carries raw push tokens or endpoints.
    pub fn has_subscription_preconditions(&self) -> bool {
        self.target.is_safe_reference() && optional_secret_reference_is_safe(Some(&self.secret_ref))
    }
}
