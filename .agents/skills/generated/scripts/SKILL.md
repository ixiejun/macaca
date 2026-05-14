---
name: scripts
description: "Skill for the Scripts area of agent. 15 symbols across 2 files."
---

# Scripts

15 symbols | 2 files | Cohesion: 86%

## When to Use

- Working with code in `macaca/`
- Understanding how is_complete, get_missing_events, generate_report work
- Modifying scripts-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `macaca/scripts/verify_logs.py` | is_complete, get_missing_events, generate_report, calculate_duration, parse_log_line (+7) |
| `macaca/scripts/benchmark.py` | parse_log_timestamps, analyze_log_structure, main |

## Entry Points

Start here when exploring this area:

- **`is_complete`** (Function) — `macaca/scripts/verify_logs.py:58`
- **`get_missing_events`** (Function) — `macaca/scripts/verify_logs.py:62`
- **`generate_report`** (Function) — `macaca/scripts/verify_logs.py:236`
- **`calculate_duration`** (Function) — `macaca/scripts/verify_logs.py:307`
- **`parse_log_line`** (Function) — `macaca/scripts/verify_logs.py:93`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `is_complete` | Function | `macaca/scripts/verify_logs.py` | 58 |
| `get_missing_events` | Function | `macaca/scripts/verify_logs.py` | 62 |
| `generate_report` | Function | `macaca/scripts/verify_logs.py` | 236 |
| `calculate_duration` | Function | `macaca/scripts/verify_logs.py` | 307 |
| `parse_log_line` | Function | `macaca/scripts/verify_logs.py` | 93 |
| `verify` | Function | `macaca/scripts/verify_logs.py` | 212 |
| `print_report` | Function | `macaca/scripts/verify_logs.py` | 327 |
| `main` | Function | `macaca/scripts/verify_logs.py` | 408 |
| `extract_task_id` | Function | `macaca/scripts/verify_logs.py` | 102 |
| `extract_fork_id` | Function | `macaca/scripts/verify_logs.py` | 120 |
| `extract_trace_id` | Function | `macaca/scripts/verify_logs.py` | 125 |
| `process_log` | Function | `macaca/scripts/verify_logs.py` | 130 |
| `parse_log_timestamps` | Function | `macaca/scripts/benchmark.py` | 21 |
| `analyze_log_structure` | Function | `macaca/scripts/benchmark.py` | 54 |
| `main` | Function | `macaca/scripts/benchmark.py` | 107 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Extract_task_id` | cross_community | 4 |
| `Main → Extract_fork_id` | cross_community | 4 |
| `Main → Extract_trace_id` | cross_community | 4 |
| `Main → Is_complete` | cross_community | 4 |
| `Main → Get_missing_events` | cross_community | 4 |
| `Main → Calculate_duration` | cross_community | 4 |

## How to Explore

1. `gitnexus_context({name: "is_complete"})` — see callers and callees
2. `gitnexus_query({query: "scripts"})` — find related execution flows
3. Read key files listed above for implementation details
