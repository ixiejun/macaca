# SDD Analyze Phase

You are in the **Analyze** phase of Spec-Driven Development (SDD).

## Context

A complex feature has been identified that requires proper specification before implementation. Your role is to thoroughly analyze the requirements and gather all necessary context.

## Your Task

Perform comprehensive analysis of the feature request.

### 1. Understand Requirements
- Parse the feature request completely
- Identify functional requirements
- Identify non-functional requirements (performance, security, accessibility)
- Note any ambiguities that need clarification

### 2. Gather Technical Context
- Check existing codebase structure
- Identify affected files and components
- Review related APIs and data models
- Check for existing patterns to follow

### 3. Design Analysis (if Figma provided)
- Fetch design context using Figma MCP
- Document component hierarchy
- Extract colors, typography, spacing
- Identify responsive breakpoints

### 4. Architecture Considerations
- Frontend component structure
- Backend API requirements
- Database changes needed
- Third-party integrations

## Output Format

```markdown
## Feature Analysis

### Summary
[One-sentence description of what we're building]

### Requirements
**Functional:**
1. [Requirement 1]
2. [Requirement 2]

**Non-Functional:**
- Performance: [requirements]
- Security: [requirements]
- Accessibility: [requirements]

### Technical Context
- **Affected Areas**: [list files/modules]
- **Database Changes**: [yes/no, details]
- **API Changes**: [yes/no, details]
- **Third-party Services**: [if any]

### Component Structure
[Proposed component hierarchy if applicable]

### Open Questions
1. [Question 1]
2. [Question 2]

### Recommendation
[Proceed to SPEC phase / Need clarification / Simplified approach possible]
```

## Next Step

After completing analysis, the Plan phase will create detailed specifications using OpenSpec.
