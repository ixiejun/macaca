# Newsroom AutoWriter

Newsroom AutoWriter is a declarative test application for Macaca Agent OS. It exercises non-coding autonomous work: live web research, source validation, analysis, writing, editing, task dependencies, review, trace visibility, and final coordinator resume.

## Suggested Demo Prompt

```text
Write a researched analysis article about DeepSeek V4 for technical founders and AI product leaders. Include market implications, technical uncertainty, competitive positioning, and a publication package with headline options.
```

## Agent Roles

- `news_coordinator`: user-facing editor-in-chief; creates goals and reports final deliverables.
- `news_planner`: managing editor; decomposes goals, assigns tasks, sets dependencies, reviews work.
- `news_researcher`: searches the web, inspects pages, and writes source packs.
- `news_fact_checker`: verifies claims and flags unsupported or speculative statements.
- `news_analyst`: builds thesis, argument map, implications, counterarguments, and caveats.
- `news_writer`: drafts the long-form article in Markdown.
- `news_editor`: produces final article, headlines, SEO/social copy, and publication checklist.

## Expected Task Flow

1. `news_coordinator` calls `create_goal`.
2. `news_planner` creates ordered TaskBoard todos.
3. `news_researcher` writes `shared/research/source-pack.md`.
4. `news_fact_checker` writes `shared/research/fact-check.md`.
5. `news_analyst` writes `shared/research/analysis-brief.md`.
6. `news_writer` writes `shared/articles/<topic>-draft.md`.
7. `news_editor` writes `shared/articles/<topic>-final.md` and `shared/articles/publication-package.md`.
8. `news_planner` reviews every task.
9. `news_coordinator` resumes and summarizes final outputs.

## Capability Coverage

This app is intended to test:

- multi-agent planning and review
- dependency-aware task claiming
- worker trace streaming for tool calls and assistant messages
- browser/search style tool execution through `playwright_search`
- source-backed writing workflows
- final coordinator resume after goal completion
- historical event restoration after refresh

## Web Research Skill

`playwright_search` exposes two actions:

```json
{"action":"search","args":["DeepSeek V4 latest analysis","--max-results","8"]}
```

```json
{"action":"fetch_url","args":["https://example.com/source"]}
```

It uses Playwright when available and falls back to HTTP search/extraction when Playwright is not installed.
