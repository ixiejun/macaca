//! Thin Web shell contract crate for Macaca OS.
//!
//! The executable Web process is assembled by `macaca-host-composition`, which
//! owns host providers, runtime handles, application bootstrap, and persistence
//! anchors. This package remains the presentation-shell contract crate and may
//! depend only on `macaca-proto` and `macaca-sdk` among workspace crates.

/// Session module marker retained for the external contract gate.
pub mod session {}

/// Route module marker retained for the external contract gate.
pub mod routes {}

/// Provider-neutral bootstrap contract for callers that still reference the
/// historical `macaca_web::WebServerBuilder` type.
pub mod bootstrap {
    use macaca_proto::{MacacaError, MacacaResult};

    /// Thin-shell server builder marker.
    ///
    /// Starting the local process requires host composition because provider
    /// construction and runtime ownership are not shell responsibilities.
    #[derive(Debug, Clone)]
    pub struct WebServerBuilder {
        port: u16,
    }

    impl Default for WebServerBuilder {
        fn default() -> Self {
            Self { port: 3001 }
        }
    }

    impl WebServerBuilder {
        /// Create a builder with the default web port.
        pub fn new() -> Self {
            Self::default()
        }

        /// Record the requested port for diagnostics.
        pub fn port(mut self, port: u16) -> Self {
            self.port = port;
            self
        }

        /// Return structured unavailable behavior instead of constructing host
        /// providers from the shell contract crate.
        pub async fn serve(self) -> MacacaResult<()> {
            Err(MacacaError::Config(format!(
                "macaca-web shell contract cannot assemble host runtime on port {}; run the macaca-web-server binary from macaca-host-composition",
                self.port
            )))
        }
    }

    /// Marker facade retained so downstream code can detect the shell contract
    /// without importing host-owned state.
    #[derive(Debug, Clone, Default)]
    pub struct WebRuntimeFacade;
}

pub use bootstrap::{WebRuntimeFacade, WebServerBuilder};
