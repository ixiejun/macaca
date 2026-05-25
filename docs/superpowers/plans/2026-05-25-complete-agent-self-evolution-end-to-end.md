# Complete Agent Self-Evolution End-To-End Plan

## Decision

Use a runtime-host Bridge over service-owned Strategies. The autonomy evolution service owns gate semantics and OS-code proposal evaluation; runtime-host owns only command sequencing across services.

## One Task

Implement the complete unattended closure path in one change:

`observer evidence -> live tick -> admission -> benchmark -> release safety -> target adapter dispatch -> governance live audit replay`

## Implementation Steps

1. Extend the autonomy evolution service command surface with OS-code proposal evaluation.
2. Add a runtime-host live executor that calls autonomy evolution, target adapters, and audit replay in order.
3. Support Skill package materialization through the existing Skill service operator.
4. Support OS-code evolution through the existing proposal adapter, with direct mutation blocked by default.
5. Add tests for Skill dispatch, OS-code dispatch, unsupported-target fail-closed behavior, and replayed audit evidence.
