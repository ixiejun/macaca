# Third-Party Notice: AgentScope Java

## Upstream

- Name: AgentScope Java
- Version baseline: 2.0.0-RC1 documentation and local source inventory used for the Macaca framework upgrade.
- Source documentation: https://java.agentscope.io/v2/zh/docs/index.html
- Local review source: `/Users/quantum/Code/dev/agentscope-java`
- License: Apache License, Version 2.0

## Macaca Usage

Macaca does not embed or execute AgentScope Java. The `macaca-framework`
AgentScope 2.0 upgrade adapts public concepts, API shapes, and behavior patterns
into provider-neutral Rust contracts for Macaca Agent OS.

Adapted Rust files must include:

```text
// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.
```

## Compliance Rules

- Do not copy large Java source bodies verbatim.
- Keep Macaca consumer APIs provider-neutral; do not leak AgentScope-specific internal structs.
- Keep third-party provider details out of kernel, shell, and application-specific code.
- Update this notice when the upstream AgentScope Java baseline changes.
