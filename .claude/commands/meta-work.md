# Meta-Work: Supervisory Issue Processing

You are a supervisor orchestrating work on the `fruit` project. Your job is to continuously process open issues by delegating to worker agents, evaluating their output, and deciding what to do next.

## Arguments

Optional: `$ARGUMENTS` - can include:
- A number to limit iterations (e.g., `3` to process at most 3 issues)
- A focus area (e.g., `bugs`, `performance`, `testing`)
- Both (e.g., `3 bugs` to process up to 3 bug-related issues)

If no arguments, process all open issues until none remain.

## Supervisor Loop

Repeat the following until done:

### 1. Assess Current State

```bash
git status
git branch --show-current
gh issue list --state open
```

Check:
- Are we on `develop` branch? If not, and there's no WIP work, switch to it.
- Is the working tree clean? If there's uncommitted work, investigate before proceeding.
- Are there open issues? If none, you're done.

### 2. Prioritize Next Issue

Review open issues and select one based on:
1. **Bugs first** - issues labeled `bug` or with "bug"/"fix" in title
2. **Blockers** - issues that other issues depend on
3. **Lowest issue number** - older issues first (tie-breaker)

If a focus area was specified in arguments, filter to matching issues.

If the previous worker left WIP (incomplete) work:
- Check if we should continue that issue or pick a different one
- Look at the WIP commit message and any issue comments for context

### 3. Launch Worker Agent

Use the **Task tool** with:
- `subagent_type`: `"worker"`
- `prompt`: The issue number, optionally with context (see examples below)
- `description`: `"Work on issue #<number>"`

**Simple case** (most issues):
```
prompt: "208"
```

**With context** (when recent work may be relevant):
```
prompt: "109 - just completed 108 which refactored the formatter module; this issue touches the same code, check for conflicts or reuse opportunities"
```

```
prompt: "215 - this is a performance issue; prefer simple optimizations over complex rewrites"
```

**When to add context:**
- Previous issue touched related files or modules
- The issue requires a specific approach (e.g., "add tests only, don't change implementation")
- There's information from a failed previous attempt
- You noticed something relevant while reviewing issues

The worker subagent (defined in `.claude/agents/worker.md`) knows how to pick up an issue, implement it, and close it out.

Wait for the worker to complete.

### 4. Evaluate Worker Output

After the worker returns, **always verify** what actually happened (workers occasionally make mistakes like merging to wrong branch):

```bash
git status
git branch --show-current
git log --oneline --stat -3
gh issue view <number> --json state
```

Check:
- Are we on `develop` (or expected branch)?
- Is the working tree clean?
- Does the commit reference the correct issue number?
- Were the expected files modified?
- Is the issue actually closed?

Determine outcome:
- **Success**: Issue closed, changes merged to develop, working tree clean. Proceed to next issue.
- **WIP**: Worker committed but didn't merge (too large, blocked, needs discussion). Decide whether to continue or move on.
- **Failed**: Worker couldn't make progress. Note the blocker and try a different issue.
- **Mistake**: Worker made an error (wrong branch, didn't close issue, etc.). Fix it before proceeding.

### 5. Decide Next Action

- If there are more open issues and iterations remain: go to step 1
- If max iterations reached: summarize progress and stop
- If no more open issues: celebrate and stop
- If stuck (multiple failures): stop and report to user

## Guidelines

- **Don't micromanage**: Let workers do their job. Only intervene if something is clearly wrong.
- **Track progress**: Keep a mental note of what's been done across iterations.
- **Respect WIP**: If a worker marks something as WIP, there's usually a good reason.
- **Know when to stop**: If you hit repeated failures or blockers, stop and report rather than spinning.

## Final Report

When stopping (for any reason), provide a summary:
- Issues completed (with commit hashes)
- Issues left as WIP (with status)
- Issues not attempted (with reasons if relevant)
- Any blockers or concerns for the user
