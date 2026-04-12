#!/usr/bin/env python3
"""
Poll Macaca run-trace + raw events for a session. Detect stalls, deadlines, and todo failures.

Usage:
  export MACACA_API=http://127.0.0.1:3001
  python3 macaca/scripts/trace_watch.py <session_id> [--interval 3] [--once]

  # Stop waiting blindly:
  python3 macaca/scripts/trace_watch.py <session_id> --deadline-seconds 600 --stall-seconds 120 \\
      --app-id <uuid> --exit-on-todo-failed

  # With --app-id, also prints GET .../todos/claim-diagnostics (why Pending tasks are not claimed).
  # Use --no-claim-diagnostics to skip.

Requires: urllib (stdlib)
"""
from __future__ import annotations

import argparse
from typing import Optional
import json
import sys
import time
import urllib.error
import urllib.request


def fetch(url: str) -> dict:
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.loads(r.read().decode())


def main() -> int:
    p = argparse.ArgumentParser(description="Watch Macaca session run_trace + errors")
    p.add_argument("session_id")
    p.add_argument("--interval", type=float, default=3.0)
    p.add_argument("--once", action="store_true")
    p.add_argument("--base", default=None, help="API base, default MACACA_API or http://127.0.0.1:3001")
    p.add_argument(
        "--deadline-seconds",
        type=float,
        default=0.0,
        help="Wall-clock max runtime (exit 2 when exceeded). 0 = no limit.",
    )
    p.add_argument(
        "--stall-seconds",
        type=float,
        default=0.0,
        help="If run_trace latest_seq unchanged for this long, exit 3 (stalled). 0 = off.",
    )
    p.add_argument(
        "--app-id",
        default=None,
        help="If set, poll /api/apps/{app_id}/todos/progress?session_id=... each tick",
    )
    p.add_argument(
        "--exit-on-todo-failed",
        action="store_true",
        help="With --app-id: exit 4 when progress.failed > 0",
    )
    p.add_argument(
        "--no-claim-diagnostics",
        action="store_true",
        help="With --app-id: do not fetch /todos/claim-diagnostics",
    )
    args = p.parse_args()
    base = (args.base or __import__("os").environ.get("MACACA_API") or "http://127.0.0.1:3001").rstrip("/")
    sid = args.session_id

    last_seq = 0
    started = time.monotonic()
    stall_tracker_seq: Optional[int] = None
    stall_since: Optional[float] = None

    while True:
        elapsed = time.monotonic() - started
        if args.deadline_seconds > 0 and elapsed >= args.deadline_seconds:
            print(
                f"[trace_watch] DEADLINE exceeded ({args.deadline_seconds}s elapsed) — "
                "coordinator may be paused on create_goal (waiting for goal completion) or SSE still open.",
                file=sys.stderr,
            )
            return 2

        try:
            rt = fetch(f"{base}/api/sessions/{sid}/run-trace?since=0&limit=200")
            ev = fetch(f"{base}/api/sessions/{sid}/events?since={last_seq}&limit=500")
        except urllib.error.URLError as e:
            print(f"[trace_watch] request failed: {e}", file=sys.stderr)
            if args.once:
                return 1
            time.sleep(args.interval)
            continue

        latest = int(rt.get("latest_seq") or 0)
        if args.stall_seconds > 0:
            if stall_tracker_seq is None or latest != stall_tracker_seq:
                stall_tracker_seq = latest
                stall_since = time.monotonic()
            elif stall_since is not None and (time.monotonic() - stall_since) >= args.stall_seconds:
                print(
                    f"[trace_watch] STALL: latest_seq={latest} unchanged for {args.stall_seconds}s "
                    "(no new run_trace checkpoints — check planner review_todo / goal completion / backend log).",
                    file=sys.stderr,
                )
                return 3

        traces = rt.get("events") or []
        print(f"--- run_trace (latest_seq={latest}, showing {len(traces)} checkpoints) ---")
        for e in traces[-30:]:
            pl = e.get("payload") or {}
            print(
                f"  seq={e.get('seq')} phase={pl.get('phase')} status={pl.get('status')} "
                f"component={pl.get('component')} msg={pl.get('message')!r}"
            )

        if args.app_id:
            try:
                prog = fetch(f"{base}/api/apps/{args.app_id}/todos/progress?session_id={sid}")
                print(
                    f"--- todos/progress session={sid[:8]}… "
                    f"total={prog.get('total')} completed={prog.get('completed')} failed={prog.get('failed')} "
                    f"pending_review={prog.get('pending_review')} in_progress={prog.get('in_progress')} "
                    f"pending={prog.get('pending')} ---"
                )
                if args.exit_on_todo_failed and int(prog.get("failed") or 0) > 0:
                    print("[trace_watch] EXIT: progress.failed > 0", file=sys.stderr)
                    return 4
            except urllib.error.URLError as e:
                print(f"[trace_watch] progress fetch failed: {e}", file=sys.stderr)

            if not args.no_claim_diagnostics:
                try:
                    cd = fetch(
                        f"{base}/api/apps/{args.app_id}/todos/claim-diagnostics?session_id={sid}"
                    )
                    stuck = cd.get("agents_pending_but_not_claiming") or []
                    print(
                        f"--- claim-diagnostics: agents with Pending but not claiming now: {stuck!r} ---"
                    )
                    for a in cd.get("agents") or []:
                        if not a.get("worker_would_claim_now") and int(a.get("pending_count") or 0) > 0:
                            print(f"  [{a.get('agent')}] pending={a.get('pending_count')} — {a.get('summary')}")
                            fg = a.get("first_gate") or {}
                            if fg:
                                print(
                                    f"    gate: seq={fg.get('sequence_number')} "
                                    f"status={fg.get('status')} kind={fg.get('gate_kind')} "
                                    f"id={fg.get('task_id')} title={fg.get('title')!r}"
                                )
                                for dep in fg.get("incomplete_dependencies") or []:
                                    print(
                                        f"      dep {dep.get('task_id')} "
                                        f"[{dep.get('status')}] {dep.get('title')!r} ({dep.get('assigned_agent')})"
                                    )
                except urllib.error.HTTPError as e:
                    if e.code == 400:
                        print(
                            "[trace_watch] claim-diagnostics: session_id required (unexpected)",
                            file=sys.stderr,
                        )
                    else:
                        print(f"[trace_watch] claim-diagnostics HTTP {e.code}", file=sys.stderr)
                except urllib.error.URLError as e:
                    print(f"[trace_watch] claim-diagnostics fetch failed: {e}", file=sys.stderr)

        events = ev.get("events") or []
        errs = []
        for e in events:
            et = e.get("event_type", "")
            pl = e.get("payload") or {}
            if et == "tool_result" and isinstance(pl, dict):
                out = str(pl.get("output", ""))
                if "error" in out.lower() or pl.get("is_error"):
                    errs.append((e.get("seq"), pl.get("tool_name"), out[:200]))
            if et == "delegated_tool_result":
                ev2 = pl.get("event") or {}
                if ev2.get("is_error"):
                    errs.append((e.get("seq"), ev2.get("tool_name"), str(ev2.get("output", ""))[:200]))
            if et in ("delegated_task_error", "error"):
                errs.append((e.get("seq"), et, str(pl)[:200]))
        if events:
            last_seq = max(last_seq, max(e.get("seq", 0) for e in events))

        if errs:
            print("--- recent errors (from new events slice) ---")
            for row in errs[-15:]:
                print(f"  seq={row[0]} {row[1]}: {row[2]}")

        if args.once:
            return 0
        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
