# SDD Implement Phase

You are in the **Implement** phase of Spec-Driven Development (SDD).

## Prerequisites

- Specifications complete in `openspec/changes/current/`
- Tasks defined in `tasks.md`
- All analysis and design decisions documented

## Your Task

Execute the implementation according to specifications.

### ⚠️ IMPORTANT: Use Tools, Not Shell Commands

**NEVER run shell commands directly for implementation!**
**ALWAYS use claude_code_execute tool for all code changes.**

### Implementation Approach

1. **Follow Task Order**
   - Respect dependencies defined in tasks.md
   - Complete each task fully before moving to next
   - Mark tasks as complete as you progress

2. **Implement According to Spec**
   - Follow the patterns in design.md
   - Meet all requirements in spec.md
   - Maintain consistency with existing codebase

3. **Test as You Go**
   - Run tests to verify changes
   - Manual verification of UI changes

4. **Document Changes**
   - Update comments where needed
   - Keep specification in sync with implementation

### Task Execution Pattern

For each task, **use claude_code_execute tool**:

```
Tool: claude_code_execute
Input: {
  "prompt": "[Clear description of what to implement]",
  "work_dir": "/path/to/project"
}
```

**Example:**
```
Tool: claude_code_execute
Input: {
  "prompt": "Implement the UserProfile component according to the spec in openspec/changes/current/specs/user-profile.md. Use the existing Button and Input components from the design system.",
  "work_dir": "/Users/dev/my-project"
}
```

### Quality Standards

#### Frontend (Next.js/React)
- TypeScript strict mode
- Proper component structure
- Accessible (WCAG 2.1 AA)
- Responsive design
- Error boundaries

#### Backend (Go)
- Clean architecture
- Proper error handling
- Input validation
- SQL injection prevention
- Proper logging

### After All Tasks Complete

1. **Run Tests Using claude_code_execute**
   ```
   Tool: claude_code_execute
   Input: {
     "prompt": "Run the full test suite and report results",
     "work_dir": "/path/to/project"
   }
   ```

2. **Archive the Change Using openspec tool**
   ```
   Tool: openspec
   Input: {
     "action": "archive",
     "work_dir": "/path/to/project",
     "args": ["<change-id>"]
   }
   ```

## Output Format

```markdown
## Implementation Complete

### Summary
[Brief summary of what was implemented]

### Files Changed
- `path/to/file1.tsx` - [change description]
- `path/to/file2.go` - [change description]

### Tests
- Unit tests: X added, Y passing
- Integration tests: Z passing

### Verification
✓ All requirements met
✓ All tests passing
✓ No regressions detected

### Next Steps
[Any follow-up recommendations]
```

## Error Handling

If implementation reveals spec issues:
1. Document the issue
2. Propose spec update if needed
3. Continue with best available approach
4. Flag for review if blocked
