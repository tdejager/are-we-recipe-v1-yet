# Attribution Pipeline

The pipeline collects attribution data for Recipe v1 feedstocks - determining who converted each feedstock to the new format.

## Phase 1: Identify Feedstocks Needing Attribution

```
collect_attributions() starts
    ↓
Filter feedstock_states for:
  - recipe_type == RecipeV1
  - attribution == None
    ↓
Check for cached commit data (from previous interrupted runs)
```

## Phase 2: Batch Query Recipe Commit History

```
GitHubClient::batch_query_recipe_history()
    ↓
GraphQL batches of 50 repos each
    ↓
For each repo, queries: history(path: "recipe.yaml") or "recipe/recipe.yaml"
    ↓
Returns: first commit that added recipe.yaml (sha, message, date, author)
    ↓
Cache results in feedstock_states.recipe_commit_cache
    ↓
💾 Save checkpoint (resume point if interrupted)
```

## Phase 3: Classify New Feedstocks vs Conversions

```
Check commit message for each result:
    ↓
is_initial_feedstock_commit(message)?
  - Contains "initial feedstock commit"
  - Starts with "initial commit"
    ↓
YES → New Feedstock (recipe.yaml existed from day 1)
NO  → Conversion (recipe.yaml added later)
```

## Phase 4: Batch Fetch Maintainers (New Feedstocks Only)

```
GitHubClient::batch_fetch_maintainers()
    ↓
GraphQL batches of 50 repos
    ↓
Fetches content of recipe.yaml or recipe/recipe.yaml
    ↓
Parses YAML to extract extra.recipe-maintainers list
    ↓
Returns: HashMap<feedstock, Vec<maintainer>>
```

## Phase 5: Batch Fetch PR Info (Conversions Only)

```
GitHubClient::batch_query_prs_for_commits()
    ↓
GraphQL batches of 50 repos
    ↓
For each commit SHA, queries: associatedPullRequests
    ↓
Returns: HashMap<feedstock, PullRequestInfo{number, author}>
```

## Phase 6: Batch Fetch Human Contributors (Bot PRs Only)

```
Filter PRs where author is a bot (matches BOT_PATTERNS)
    ↓
GitHubClient::batch_fetch_pr_human_contributors()
    ↓
GraphQL batches of 50 PRs
    ↓
For each PR, fetches commits and finds first non-bot author
    ↓
Returns: HashMap<feedstock, human_username>
```

## Phase 7: Process Attributions (Fast - All Data Pre-fetched)

```
For each feedstock result:
    ↓
If NEW FEEDSTOCK:
  → contributors = maintainers from recipe.yaml (or "unknown")
  → contribution_type = NewFeedstock
    ↓
If CONVERSION:
  → If PR found with human author: credit PR author
  → If PR found with bot author: credit pre-fetched human contributor
  → If no PR: credit commit author
  → contribution_type = Conversion
    ↓
Store Attribution { contribution_type, contributors, date, commit_sha }
```

## Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    BATCH QUERIES (GraphQL)                       │
├─────────────────────────────────────────────────────────────────┤
│  1. Recipe commit history    → commit SHA, message, author      │
│  2. Maintainers (new only)   → Vec<maintainer>                  │
│  3. PR info (conversions)    → PR number, author                │
│  4. Human contributors (bots)→ human username                   │
└─────────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────────┐
│                    ATTRIBUTION DECISION                          │
├─────────────────────────────────────────────────────────────────┤
│  New Feedstock:                                                  │
│    → Credit: recipe.yaml maintainers                            │
│                                                                  │
│  Conversion (human PR):                                          │
│    → Credit: PR author                                          │
│                                                                  │
│  Conversion (bot PR):                                            │
│    → Credit: first human commit author in PR                    │
│                                                                  │
│  Conversion (no PR):                                             │
│    → Credit: commit author                                      │
└─────────────────────────────────────────────────────────────────┘
```

## CLI Flags

- `--reattribute` - Clear existing attributions and recalculate all
- `--reattribute-only` - Skip analysis, just run attribution on existing data
- `--refetch-recipe-commits` - Clear cached commit data, force re-fetch from API

## Key Files

- `attribution.rs` - Main `collect_attributions()` function and processing logic
- `github.rs` - GraphQL batch query functions
- `models/feedstock.rs` - `Attribution`, `RecipeCommitCache` structs
- `models/cli.rs` - CLI flag definitions
