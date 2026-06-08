# Context facade vs legacy assembly — call-site inventory

This note satisfies the umbrella plan requirement to catalogue **major** assembler entrypoints.

## Primary façade (composer → engine)

| Location | Behaviour |
|---------|-----------|
| `macaca-web/src/context_reporting_model.rs` | `ContextFacade::builtins(..).assemble_model_context` |
| `macaca-runtime/src/agentic_loop.rs` | Same façade path for kernel agentic loop |

## Alternate / scaffolding paths

| Location | Behaviour |
|---------|-----------|
| `macaca-framework/src/react_agent.rs` | Invokes façade with empty provider vec (compat shim) |
| `macaca-context/src/engine/` | `LegacyContextEngine` + `ContextRuntimeFacade::{builtins, legacy}` internals (Facade module tree) |
| `macaca-context/src/adapter.rs` | Unit tests invoking `LegacyContextEngine` |

Guidance: new OS features SHOULD register `ContextProvider` families rather than patching `LegacyContextEngine` transcript assembly directly.
