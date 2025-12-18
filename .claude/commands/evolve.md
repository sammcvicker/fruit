# Feature Evolution Brainstorm

You are a product-minded engineer exploring new directions for `fruit`. Your goal is to generate innovative feature ideas that align with the project's philosophy, then create well-structured GitHub issues for the best ones.

## Arguments

Optional: `$ARGUMENTS` - theme to explore (e.g., `ai`, `scripting`, `performance`, `integration`)

## Guiding Principles

Features should align with these pillars:

1. **AI and Human Parity**: Tools designed for humans should work equally well for AI agents. Clear, parseable output. Scriptable interfaces. No interactive-only features.

2. **Codebase Table of Contents**: `fruit` should be the best way to understand a codebase at a glance. What files exist, what they do, what patterns they follow.

3. **Speed**: Stay fast. If a feature adds latency, it should be opt-in. Streaming output, lazy evaluation, parallel processing.

4. **Scriptability**: Every feature should compose well with Unix pipes and other tools. JSON output, exit codes, filtering.

## Workflow

### 1. Understand Current State

Review what `fruit` already does:

```bash
cargo run -- --help
cargo run -- .
cargo run -- --json . | head -50
cargo run -- --types .
```

Read the README and PLAN.md for existing roadmap items.

### 2. Explore Adjacent Possibilities

Consider these categories:

**Metadata Extraction** (building on comments and types)
- What other useful metadata could be extracted?
- Function signatures, imports, exports, dependencies?
- TODO/FIXME aggregation across the codebase?
- Test coverage indicators?

**Output Formats** (building on JSON)
- Could `fruit` output formats useful for other tools?
- Markdown for documentation?
- GraphViz for dependency visualization?
- LSP-compatible structures?

**Filtering and Querying**
- Beyond `.gitignore`, what filtering would be useful?
- By language? By modification time? By size?
- Glob patterns for includes/excludes?
- Content-based filtering (files containing X)?

**Integration Points**
- What tools would benefit from `fruit` output?
- Editor plugins? CI pipelines? Documentation generators?
- How could `fruit` be a building block for larger tools?

**AI/LLM Use Cases**
- What would an AI agent want from a tree command?
- Context window optimization (prioritize important files)?
- Semantic file grouping?
- Automatic relevance scoring?

### 3. Validate Ideas Against Principles

For each idea, ask:
- Does this serve both humans AND machines? (Parity)
- Does this help understand the codebase better? (ToC)
- Can this be fast or opt-in? (Speed)
- Does this compose with other tools? (Scriptability)

### 4. Check Existing Issues

```bash
gh issue list --state all
```

Don't duplicate. Build on existing ideas if relevant.

### 5. Create Feature Issues

For promising ideas:

```bash
gh issue create \
  --title "[Feature] <concise description>" \
  --label "enhancement" \
  --body "$(cat <<'EOF'
## Summary

<One paragraph describing the feature>

## Motivation

<Why would someone want this? What problem does it solve?>

## Design Sketch

<How might this work? CLI interface, output format, implementation approach>

## Examples

```bash
# Example usage
fruit --new-flag .
```

## Alignment

- [ ] AI/Human Parity
- [ ] Codebase Table of Contents
- [ ] Speed
- [ ] Scriptability

## Complexity

<Small/Medium/Large - rough implementation scope>

EOF
)"
```

### 6. Prioritize by Value

Consider:
- How many users would benefit?
- How well does it align with `fruit`'s core purpose?
- Implementation complexity vs. value delivered
- Does it open doors to other features?

### 7. Report Summary

Conclude with:

```markdown
## Evolution Session Summary

**Theme Explored**: <focus area or general>
**Issues Created**: <count>

### Featured Ideas
1. **#XX: <title>** - <one line summary>
2. **#XX: <title>** - <one line summary>

### Quick Wins (low effort, high value)
- #XX: <title>

### Ambitious Ideas (high effort, potentially transformative)
- #XX: <title>

### Deferred (interesting but not aligned)
- <idea and why it was skipped>

### Questions for Future Exploration
- <open questions that emerged>
```

## Idea Quality Guidelines

Good feature ideas:
- Solve a real problem someone has
- Fit naturally with existing functionality
- Have clear, concrete use cases
- Can be implemented incrementally

Avoid:
- Features that only sound cool but have no clear user
- Scope creep that dilutes `fruit`'s focus
- Features that require interactive input (breaks scriptability)
- Performance-heavy defaults (should be opt-in)

## Inspiration Sources

When stuck, consider:
- What do you wish `tree` could do?
- What's annoying about exploring unfamiliar codebases?
- What would make `fruit` output more useful in a prompt to an LLM?
- What would make CI pipelines smarter about what changed?
