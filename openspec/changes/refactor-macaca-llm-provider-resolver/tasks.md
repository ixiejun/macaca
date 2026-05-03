## 1. OpenSpec

- [x] 1.1 Read current macaca-llm design plan and existing `refactor-llm-provider-model-routing` change.
- [x] 1.2 Create proposal, design, task list, and delta spec for the smaller resolver change.
- [x] 1.3 Validate with `openspec validate refactor-macaca-llm-provider-resolver --strict`.

## 2. Impact and Baseline

- [x] 2.1 Refresh GitNexus index.
- [x] 2.2 Run GitNexus impact for `LlmRouter`.
- [x] 2.3 Run GitNexus impact for `resolve_target`.
- [x] 2.4 Run GitNexus impact for `resolve_provider_name`.
- [x] 2.5 Run baseline `cargo test -p macaca-llm router -- --nocapture`.
- [x] 2.6 Run baseline `cargo check -p macaca-llm`.

## 3. Resolver Primitives

- [x] 3.1 Create `macaca/crates/macaca-llm/src/resolver.rs`.
- [x] 3.2 Add `ProviderResolver`.
- [x] 3.3 Add prefix/static resolver for current built-in rules.
- [x] 3.4 Add `ResolverChain` with first-match semantics and unknown-model fallback.
- [x] 3.5 Add resolver unit tests covering current routing rules.

## 4. Router Migration

- [x] 4.1 Add default resolver chain to `LlmRouter`.
- [x] 4.2 Make `resolve_target` call the resolver chain.
- [x] 4.3 Mark `LlmRouter::resolve_provider_name` deprecated but callable.
- [x] 4.4 Move provider-name inference tests to resolver tests.
- [x] 4.5 Add compatibility test proving deprecated helper remains callable.
- [x] 4.6 Move router tests to a sibling test module so `router.rs` stays below 500 lines.

## 5. Verification

- [x] 5.1 Run `cargo fmt`.
- [x] 5.2 Run `cargo test -p macaca-llm resolver -- --nocapture`.
- [x] 5.3 Run `cargo test -p macaca-llm router -- --nocapture`.
- [x] 5.4 Run `cargo check -p macaca-llm`.
- [x] 5.5 Run `openspec validate refactor-macaca-llm-provider-resolver --strict`.
- [x] 5.6 Run `npx gitnexus detect-changes --repo agent --scope all`.
