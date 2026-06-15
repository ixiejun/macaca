//! Read-only accessors exposing composed Strategy clients from the facade.
//!
//! Shells that need capability-specific command surfaces can borrow the focused
//! client without reaching through the aggregate operation methods.

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
    super::types::SystemFacade<
        T,
        S,
        SV,
        TR,
        P,
        L,
        M,
        C,
        D,
        SK,
        MCP,
        A,
        ST,
        E,
        PMT,
        W3,
        EVM,
        SCH,
        HB,
    >
{
    /// Borrow the focused Web3 Service client.
    pub fn web3_client(&self) -> &W3 {
        &self.web3
    }

    /// Borrow the focused EVM Service client.
    pub fn evm_client(&self) -> &EVM {
        &self.evm
    }

    /// Borrow the focused Scheduler Service client.
    pub fn scheduler_client(&self) -> &SCH {
        &self.scheduler
    }

    /// Borrow the focused Heartbeat Service client.
    pub fn heartbeat_client(&self) -> &HB {
        &self.heartbeat
    }

    /// Borrow the focused Payment Service client.
    pub fn payment_client(&self) -> &PMT {
        &self.payment
    }

    /// Borrow the focused Store Service client.
    pub fn store_client(&self) -> &ST {
        &self.store
    }

    /// Borrow the focused Entitlement Service client.
    pub fn entitlement_client(&self) -> &E {
        &self.entitlement
    }

    /// Borrow the focused Application Service client.
    pub fn application_client(&self) -> &A {
        &self.application
    }

    /// Borrow the focused Driver Service client.
    pub fn driver_client(&self) -> &D {
        &self.driver
    }

    /// Borrow the focused Skill Service client.
    pub fn skill_client(&self) -> &SK {
        &self.skill
    }

    /// Borrow the focused MCP Service client.
    pub fn mcp_client(&self) -> &MCP {
        &self.mcp
    }
}
