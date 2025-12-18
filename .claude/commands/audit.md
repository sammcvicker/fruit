# Codebase Quality Audit

You are a senior engineer performing a quality audit of the `fruit` codebase. Your goal is to identify technical debt, architectural inconsistencies, and areas for improvement, then create well-structured GitHub issues for each finding.

## Arguments

Optional: `$ARGUMENTS` - focus area (e.g., `tests`, `error-handling`, `types`, `comments`, `tree`, `output`)

## Audit Pillars

Every finding should relate to one or more of these core principles:

1. **Maintainability**: Can future developers easily understand and modify this code?
2. **Testability**: Is the code structured for easy unit and integration testing?
3. **Extensibility**: Can new features be added without major refactoring?

## Workflow

### 1. Scope the Audit

**If a focus area was provided:**
- Concentrate on that specific module or concern
- Read the relevant source files deeply

**If no arguments:**
- Perform a broad audit across all modules
- Prioritize by code complexity and change frequency

### 2. Systematic Review

For each source file, evaluate:

**Code Structure**
- [ ] Functions are small and single-purpose
- [ ] Clear separation of concerns between modules
- [ ] No circular dependencies
- [ ] Consistent naming conventions

**Error Handling**
- [ ] Errors are propagated appropriately (not swallowed)
- [ ] Error messages are helpful and actionable
- [ ] Edge cases are handled gracefully

**Testing**
- [ ] Critical paths have test coverage
- [ ] Tests are isolated and deterministic
- [ ] Test helpers reduce duplication
- [ ] Edge cases in `tests/edge_cases.rs` are comprehensive

**Documentation**
- [ ] Public APIs have doc comments
- [ ] Complex logic has inline explanations
- [ ] Module-level docs explain purpose and usage

**Performance**
- [ ] No unnecessary allocations in hot paths
- [ ] Appropriate use of iterators vs collecting
- [ ] Benchmarks cover critical operations

**Type Safety**
- [ ] Minimal use of `unwrap()` in library code
- [ ] Strong types instead of primitive obsession
- [ ] Enums for state machines and variants

### 3. Check for Common Issues

Look specifically for:

```bash
# Find unwraps that could panic
rg "\.unwrap\(\)" src/

# Find TODOs and FIXMEs
rg -i "(TODO|FIXME|HACK|XXX)" src/

# Find large functions (potential refactoring targets)
rg -c "^    fn |^fn " src/*.rs

# Check for duplicated patterns
# Look for similar code blocks that could be abstracted
```

### 4. Cross-Reference with Existing Issues

```bash
gh issue list --label "tech-debt" --state all
gh issue list --label "refactor" --state all
```

Don't duplicate existing issues. Reference them if relevant.

### 5. Create Issues

For each finding, create a GitHub issue:

```bash
gh issue create \
  --title "[Audit] <concise description>" \
  --label "tech-debt" \
  --body "$(cat <<'EOF'
## Problem

<What is wrong or suboptimal>

## Impact

<How this affects maintainability/testability/extensibility>

## Location

<Specific files and line numbers>

## Suggested Fix

<Concrete steps to address this>

## Pillar

- [ ] Maintainability
- [ ] Testability
- [ ] Extensibility

EOF
)"
```

### 6. Prioritize Issues

After creating issues, add priority labels:

- `priority: high` - Actively causing problems or blocking improvements
- `priority: medium` - Should be addressed soon
- `priority: low` - Nice to have, address opportunistically

### 7. Report Summary

Conclude with a summary:

```markdown
## Audit Summary

**Files Reviewed**: <list>
**Issues Created**: <count>

### High Priority
- #XX: <title>

### Medium Priority
- #XX: <title>

### Low Priority
- #XX: <title>

### Observations
<General patterns noticed, positive highlights, architectural concerns>
```

## Issue Quality Guidelines

Good audit issues:
- Are specific and actionable
- Include file paths and line numbers
- Explain the "why" not just the "what"
- Suggest a concrete solution
- Are appropriately scoped (not too broad)

Avoid:
- Vague issues like "improve code quality"
- Combining unrelated concerns in one issue
- Nitpicks that don't meaningfully improve the codebase
- Issues that would require major rewrites without clear benefit
