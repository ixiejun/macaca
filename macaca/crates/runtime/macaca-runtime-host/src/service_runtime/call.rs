//! Call dispatch implementation for the host-owned ServiceRuntime facade.
//!
//! The logic lives in its own module because call dispatch is the highest-risk
//! runtime path: it combines Decorator admission, service-bus routing, timeout
//! and cancellation controls, bounded output checks, health observation, and
//! audit events without embedding provider-specific semantics.

use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceHealth, ServiceLifecycleState, ServiceReply,
};

use crate::{service_runtime::ServiceRuntime, service_runtime_error::ServiceRuntimeError};

impl ServiceRuntime {
    /// Dispatch a traced command through decorators and the service bus.
    pub async fn call(
        &self,
        service_id: &macaca_proto::KernelServiceId,
        source: ServiceBusSource,
        command: ServiceCommand,
    ) -> Result<ServiceReply, ServiceRuntimeError> {
        let (descriptor, trace) = self.descriptor_and_trace(service_id, &command)?;
        let dispatch_command = self.control.command_for_dispatch(command.clone());
        let context = crate::service_decorator::ServiceRuntimeCallContext {
            service_id,
            source: &source,
            command: &dispatch_command,
            descriptor: &descriptor,
        };
        let mut admission_decorators = Vec::with_capacity(self.decorators.len());
        for decorator in &self.decorators {
            if let Err(err) = decorator.before_dispatch(&context).await {
                self.emit_rejection(service_id, &command, &err)?;
                return Err(err);
            }
            admission_decorators.push(decorator.name());
        }

        let call_control = match self.control.prepare_call(&command.metadata) {
            Ok(control) => control,
            Err(err) => {
                self.emit_rejection(service_id, &command, &err)?;
                return Err(err);
            }
        };

        self.set_lifecycle(service_id, ServiceLifecycleState::Calling)?;
        self.emit(
            service_id,
            "service_runtime.call.dispatched",
            ServiceLifecycleState::Calling,
            None,
            trace.as_ref(),
            serde_json::json!({
                "command": dispatch_command.name.to_string(),
                "admission_decorators": admission_decorators,
            }),
        )?;
        let service = self.service_for(service_id)?;
        let mut envelope =
            macaca_proto::ServiceEnvelope::new(source, service_id.clone(), dispatch_command);
        self.control.apply_to_envelope(&mut envelope, &call_control);

        match self
            .control
            .dispatch(call_control, self.bus.call(envelope))
            .await
        {
            Ok(reply) => {
                let health = self
                    .observed_health_after_success(service_id, &service, ServiceHealth::Healthy)
                    .await;
                if let Err(err) = self.control.validate_reply(&reply) {
                    self.set_state(
                        service_id,
                        ServiceLifecycleState::Running,
                        ServiceHealth::Degraded {
                            reason: err.to_string(),
                        },
                        Some(err.to_string()),
                    )?;
                    self.emit(
                        service_id,
                        output_rejection_operation(&err),
                        ServiceLifecycleState::Running,
                        Some(ServiceHealth::Degraded {
                            reason: err.to_string(),
                        }),
                        reply.trace.as_ref(),
                        serde_json::json!({
                            "status": "rejected",
                            "error": err.to_string(),
                        }),
                    )?;
                    return Err(err);
                }
                self.set_state(
                    service_id,
                    ServiceLifecycleState::Running,
                    health.clone(),
                    None,
                )?;
                self.emit(
                    service_id,
                    "service_runtime.call.completed",
                    ServiceLifecycleState::Running,
                    Some(health),
                    reply.trace.as_ref(),
                    serde_json::json!({"status": reply.status}),
                )?;
                Ok(reply)
            }
            Err(err) => {
                if err.is_runtime_control_failure() {
                    return self.control_failure_with_result(
                        service_id,
                        control_failure_operation(&err),
                        trace.as_ref(),
                        err,
                    );
                }
                if let ServiceRuntimeError::InvalidArgument(reason) = err {
                    self.set_state(
                        service_id,
                        ServiceLifecycleState::Running,
                        ServiceHealth::Healthy,
                        None,
                    )?;
                    self.emit(
                        service_id,
                        "service_runtime.call.rejected",
                        ServiceLifecycleState::Running,
                        Some(ServiceHealth::Healthy),
                        trace.as_ref(),
                        serde_json::json!({"error": reason, "reason_code": "invalid_argument"}),
                    )?;
                    return Err(ServiceRuntimeError::InvalidArgument(reason));
                }
                self.fail_with_result(
                    service_id,
                    "service_runtime.call.failed",
                    trace.as_ref(),
                    err.to_string(),
                )
            }
        }
    }
}

fn control_failure_operation(error: &ServiceRuntimeError) -> &'static str {
    match error {
        ServiceRuntimeError::CallTimedOut { .. } => "service_runtime.call.timed_out",
        ServiceRuntimeError::CallCancelled { .. } => "service_runtime.call.cancelled",
        _ => "service_runtime.call.control_failed",
    }
}

fn output_rejection_operation(error: &ServiceRuntimeError) -> &'static str {
    match error {
        ServiceRuntimeError::StreamFrameLimitExceeded { .. } => {
            "service_runtime.call.stream_rejected"
        }
        _ => "service_runtime.call.output_rejected",
    }
}
