# Work on GitHub Issue

You are a developer working on the `fruit` project. Your job is to pick up an issue, implement it, and close it out.

## Arguments

Optional: `$ARGUMENTS` - either an issue number (e.g., `15`) or a focus area (e.g., `bug`, `performance`, `testing`)

## Workflow

### 1. Select an Issue

Run `gh issue list` to see open issues.

**If an issue number was provided in arguments:**
- Work on that specific issue

**If a focus area was provided:**
- Filter issues by that label or keyword and pick the most impactful one

**If no arguments:**
- Review the issue list
- Prioritize by: bugs first, then issues blocking other work, then lowest issue number
- Read issue details with `gh issue view <number>` if needed to understand scope

### 2. Set Up Feature Branch

```bash
git checkout develop  # or main if no develop branch
git pull
git checkout -b issue-<number>-<short-description>
```

Use a descriptive branch name like `issue-15-fix-edition-2024` or `issue-1-file-size-limit`.

### 3. Implement the Fix

- Read relevant source files to understand the codebase
- Make minimal, focused changes that address the issue
- Follow existing code style and patterns
- Add tests if appropriate
- Run `cargo build` and `cargo test` to verify

### 4. Update the Changelog

Before committing, update `CHANGELOG.md` if the change is user-facing:
- **Added**: New features or capabilities
- **Changed**: Changes to existing functionality
- **Fixed**: Bug fixes
- **Performance**: Speed or memory improvements (include metrics if available)
- **Deprecated**: Features that will be removed
- **Removed**: Features that were removed

Skip changelog updates for:
- Internal refactoring with no user-visible changes
- Documentation-only changes
- Test-only changes

### 5. Complete the Work

**If work is COMPLETE:**
```bash
git add -A
git commit -m "Fix #<number>: <description>"
git checkout develop
git merge issue-<number>-<short-description>
git branch -d issue-<number>-<short-description>
gh issue close <number> --comment "Fixed in $(git rev-parse --short HEAD)"
```

**If work is NOT COMPLETE (blocked, needs discussion, too large):**
```bash
git add -A
git commit -m "WIP #<number>: <description of progress>"
gh issue comment <number> --body "Progress update: <what was done, what remains, any blockers>"
```
Do NOT merge incomplete work. Leave the branch for future continuation.

### 6. Report Back

Summarize what was done:
- Which issue was worked on
- What changes were made
- Whether it was merged or left as WIP
- Any follow-up needed

## Guidelines

- Keep changes minimal and focused on the issue
- Don't scope-creep into other improvements
- If an issue is too large, break it into smaller commits or suggest splitting the issue
- Test your changes before merging
- Write clear commit messages referencing the issue number
