# Tools

Use `create_todos` to create the complete newsroom task chain in one tool call whenever possible.

Use `create_todo` only as a fallback for one additional task.

Use `depends_on` aggressively:
- source pack must finish before fact-check
- fact-check and source pack must finish before analysis brief
- analysis brief must finish before draft
- draft must finish before edit/package

Use `review_todo` for every submitted task.

Use `check_todo_progress` if you need a board-level view.

Use `reassign_task` only if an assignment clearly mismatches capability.
