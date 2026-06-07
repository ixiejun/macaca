## ADDED Requirements

### Requirement: Consumers use skill snapshot request facade
Upper consumers SHALL construct skill snapshots through a request/facade API instead of directly assembling `SkillRuntimeOptions`.

#### Scenario: Framework agent receives same skill catalog
- **GIVEN** an application agent has the same workspace, app directory, and skill policy as before
- **WHEN** `macaca-web` builds the traced framework agent
- **THEN** the resulting skill snapshot prompt, visible skill list, and filtered skill list match the previous direct `SkillRuntime` behavior

### Requirement: Executable skill tools use adapter facade
Upper consumers SHALL expose YAML executable skills through the adapter/facade path instead of deprecated registry instantiation APIs.

#### Scenario: Startup executable skill tools remain available
- **GIVEN** an application skills directory contains executable YAML skills
- **WHEN** `macaca-web` starts and builds the composite tool set
- **THEN** those skill tools are available with the same names, descriptions, schemas, and execution output shape as before

### Requirement: Skill source inventory uses canonical source primitives
Application-level skill source inventory SHALL use canonical skill source primitives where that can preserve existing behavior exactly.

#### Scenario: App skill loader keeps existing precedence
- **GIVEN** duplicate skill names exist in app-specific and global skill directories
- **WHEN** `macaca-app::SkillLoader` resolves a skill by name
- **THEN** the app-specific skill remains preferred over the global skill

### Requirement: Deprecated skill APIs are contained
Upper crates SHALL NOT call deprecated direct skill APIs after migration.

#### Scenario: Deprecated grep only finds compatibility sites
- **WHEN** the repository is scanned for deprecated skill APIs
- **THEN** matches are limited to `macaca-skill` compatibility wrappers or unrelated non-skill APIs
