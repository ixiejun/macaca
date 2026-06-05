## 1. Specification

- [x] 1.1 Add an OpenSpec delta describing model-decided four-agent Workbench collaboration.
- [x] 1.2 Validate the new OpenSpec change in strict mode.

## 2. Application Package

- [x] 2.1 Update Workbench workflow metadata and prompts so coordinator, planner, coder, and reviewer describe explicit handoff responsibilities.
- [x] 2.2 Update the WASM build metadata to emit sequential `agent_delegate` host commands for all four agents.
- [x] 2.3 Pass prior command outputs with `${host.results.N.output}` placeholders so downstream agents receive coordinator and planner context.
- [x] 2.4 Keep all task complexity decisions in coordinator model instructions, with no keyword or language-specific hardcoding.

## 3. Generated Artifacts And Installation

- [x] 3.1 Rebuild the Workbench WASM component and confirm the generated metadata contains the four-agent delegation chain.
- [x] 3.2 Sync the rebuilt application package into local installed app workspaces used by the running backend.

## 4. Verification

- [x] 4.1 Run Workbench package validation.
- [x] 4.2 Run targeted Component Model host-command placeholder tests.
- [x] 4.3 Run `openspec validate update-codex-workbench-agent-collaboration --strict`.
- [x] 4.4 Run GitNexus change detection or record indexing/tool limitations before commit.
- [x] 4.5 Commit the OpenSpec, implementation, generated artifacts, and installation sync proof.
