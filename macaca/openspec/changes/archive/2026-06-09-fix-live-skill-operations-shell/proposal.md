# Change: Fix Live Skill Operations Shells

## Why

The live Web/runtime/frontend path reaches `service.skill`, but CLI Skill operations still use `UnavailableSystemSkillClient`. That means operators cannot prove the same self-evolving Skill governance state through every shell.

## What Changes

- Add an app-scoped CLI live mode that forwards Skill operation commands to the local Web API facade instead of creating service semantics in CLI.
- Preserve structured unavailable diagnostics when CLI is not pointed at a live app/runtime target.
- Preserve frontend mutation trace ids after automatic refresh so RUN/APPLY/ROLLBACK commands are observable.

## Impact

- Affected specs: `web-cli-thin-shell-completion`, `sdk-system-facade`
- Affected code: `macaca-cli` Skill commands, frontend Skill operations panel
