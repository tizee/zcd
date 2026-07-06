---
updated: 2026-07-06
---

# Matching & Ranking: How `z <pattern>` Picks a Directory

When the shell hook has recorded visited directories, `zcd query -- <pattern>`
must turn a short, possibly mistyped pattern into exactly one directory path.
This doc explains the two cooperating algorithms — typo-tolerant fuzzy
matching (`fuzzy` crate) and zoxide-style frecency ranking (`zcd::db`) — and
the contract between them.

## Source Files

| File | Role |
|------|------|
| `fuzzy/src/fzy.rs` | Scoring DP, skip tolerance, LCS match gate |
| `fuzzy/src/score.rs` | Score constants and character-class bonuses |
| `zcd/src/db/dir.rs` | Frecency curve, aging, query/list ordering |
| `zcd/src/db/data.rs` | z-compatible datafile codec |
| `zcd/src/db/mod.rs` | `Database` facade: load/save/import/export/aging trigger |
| `scripts/zcd.plugin.zsh` | Shell contract: stdout of `query` is the jump target |

## Design Rationale

**Problem 1 — strict matching.** The original matcher required every needle
character to appear in the haystack *in order* (fzy semantics; fzf's
FuzzyMatchV2 has the same requirement). A single transposition in the query
(`labexample` for a directory actually named `lab/exmaple`) failed the gate
before scoring even ran, so `z` found nothing.

**Solution.** Extend the fzy dynamic program with a *skip-needle* transition:
an unmatched needle character costs a fixed penalty instead of killing the
match. A separate admission gate bounds how many characters may be skipped, so
tolerance does not degenerate into matching everything.

**Problem 2 — saturating rank curve.** The previous frecency used
`(ln(visits)+1) * recency * 100` capped at 1000. The log curve converges:
heavily visited directories became indistinguishable from each other.
Additionally, `visit_count` was never persisted, so the frequency factor
effectively reset on every CLI invocation.

**Solution.** Adopt the zoxide model: rank accumulates **linearly** (+1 per
visit) and never saturates; a periodic aging sweep keeps totals bounded;
recency is applied at query time as a bucket multiplier. Because rank itself
is the only persisted frequency signal, the on-disk format stays the
3-field z-compatible text format — no schema change, and the data remains
portable to other z-like tools.

## Architecture

The fuzzy crate knows nothing about frecency; the db layer knows nothing
about scoring internals. Their contract is a single function:
`fuzzy::match_score(needle, haystack) -> f64`, where `SCORE_MIN` (−∞) means
"not a match", `SCORE_MAX` (+∞) means "exact match", and any finite value is
a comparable quality signal. The db layer combines that signal with frecency
to produce the final order.

### Query Data Flow

```
z <pattern>                        (zsh plugin)
    |
    v
zcd query -- <pattern>             (cli/mod.rs)
    |
    v
Database::query                    (db/mod.rs, pass-through)
    |
    v
DirList::query                     (db/dir.rs)
    |  for each stored dir that still exists on disk:
    |
    +--> fuzzy::match_score(pattern, path)
    |        |
    |        +--(score == SCORE_MIN)--> drop candidate
    |        |
    |        +--(score > SCORE_MIN)---> keep (bucket, frecency)
    |
    v
sort by (score bucket desc, frecency desc)
    |
    v
first path -> stdout -> shell cd
```

### Matching Pipeline (inside `match_score`)

```
needle, haystack
    |
    v
Unicode lowercasing
    |
    v
Admission gate: LCS(needle, haystack) >= n - allowed_skips(n)
    |            allowed_skips: 0 for n <= 3, else n/4
    +--(fails)--> SCORE_MIN
    |
    v
Exact equality? --(yes)--> SCORE_MAX
    |
    v
Scoring DP (fzy + skip transition) --> finite score
```

## Behavioral Contracts

- **Bounded typo tolerance.** A needle matches iff at least
  `n - allowed_skips(n)` of its characters appear in the haystack in order
  (see `matches_within_tolerance` in `fuzzy/src/fzy.rs`). Needles of length
  ≤ 3 are strict; longer needles tolerate 25% misses. This is the only
  admission rule — scoring never resurrects a gated-out candidate.
- **Skipping is a last resort.** `SCORE_SKIP_NEEDLE` is strictly more
  expensive than any single-character match bonus, so the DP only skips when
  no alignment exists (see constants in `fuzzy/src/score.rs`). Consequence: a
  correctly spelled query always outscores the same query with a typo.
- **Sentinels are infinities.** `SCORE_MIN`/`SCORE_MAX` are ±∞, so gap
  arithmetic cannot underflow and comparisons are total via `f64::total_cmp`.
  The old `f64::MIN` sentinels combined with an `as u64` cast collapsed all
  negative scores to 0 and silently dropped valid matches — the regression
  test `ordinary_subsequence_scores_positive` pins this.
- **Match quality dominates, frecency disambiguates.** Query order sorts by
  the fuzzy score bucketed to 0.1 (`score_bucket` in `zcd/src/db/dir.rs`),
  with frecency breaking ties. A clearly better match cannot be buried by a
  frequently visited but poorly matching directory, and vice versa
  near-equal matches defer to habit.
- **Rank never saturates.** Stored rank grows +1 per visit with no cap
  (`insert_or_update` in `zcd/src/db/dir.rs`); relative frequency between two
  directories is preserved indefinitely (2× visits stays 2× score).
- **Aging keeps the database bounded.** After every insert, if the sum of all
  ranks exceeds the configured `max_age`, every rank is multiplied by 0.9 and
  entries decaying below 1.0 are dropped (`DirList::age`). Aging preserves
  relative order; it only compresses magnitude.
- **Recency is a query-time multiplier, not stored state.** `frecency()`
  scales rank by 4 (visited within the hour), 2 (within a day), 0.5 (within a
  week), 0.25 (older). The datafile stores raw rank; displayed rank
  (`query -r`, `list -r`) is the frecency value.
- **stdout is the jump target.** `zcd query` prints a path (or nothing plus a
  non-zero exit) — never diagnostics — because the zsh plugin consumes stdout
  as the `cd` argument (`__zcd_z` in `scripts/zcd.plugin.zsh`).
- **The datafile is the export format.** One codec (`zcd/src/db/data.rs`)
  reads and writes `path|rank|last_accessed` lines, identical to the original
  `z` tool. `import` merges (max rank, newest access); `export` writes the
  same format, so migration to/from other z-like tools is a file copy.

## Key Mechanisms

### The scoring DP with skip transition

`compute_score` in `fuzzy/src/fzy.rs` runs a dynamic program over
(needle prefix, haystack prefix) with two rolling rows:

- `matched[i][j]` — best score where needle char `i` matches haystack char
  `j`: either start a new match (`best[i-1][j-1] + positional bonus`) or
  extend a run (`matched[i-1][j-1] + SCORE_MATCH_CONSECUTIVE`).
- `best[i][j]` — the maximum of: a match at `(i, j)`; moving along the
  haystack (`best[i][j-1] + gap`, trailing gap once all needle chars are
  consumed); or **skipping the needle char** (`best[i-1][j] +
  SCORE_SKIP_NEEDLE`) — the extension over upstream fzy.

Positional bonuses are precomputed per haystack character from its
predecessor's class (start of path component > word boundary > camelCase >
after dot), inherited unchanged from fzy so path-component starts stay the
strongest anchors.

### Why LCS as the gate, not the DP itself

The DP always produces *some* score once skips are allowed, so admission must
be decided independently. The longest common subsequence length is exactly
"how many needle characters can appear in order", making
`LCS >= n - allowed_skips(n)` the natural gate. Both are O(n·m); patterns and
paths are short, so no windowing is needed.

### Why bucket the score before sorting

Raw fuzzy scores differ by hundredths for near-identical match quality (a gap
penalty here, a bonus there). Sorting on the raw value lets that noise
override habit. Rounding to 0.1 (`score_bucket`) makes "equally good" an
explicit equivalence class inside which frecency — the signal a habit tool
actually cares about — decides.

## BDD Scenarios

Each scenario maps to an existing test.

| # | Scenario | Given | When | Then | Test |
|---|----------|-------|------|------|------|
| 1 | Transposed query still jumps | dir `lab/exmaple` recorded | `z labexample` | path is returned | `test_query::query_finds_dir_despite_transposed_characters` |
| 2 | Exact spelling beats typo | one haystack | scoring both spellings | exact subsequence scores higher | `test_fzy::correct_spelling_beats_typo` |
| 3 | Short patterns stay strict | pattern of ≤ 3 chars | chars out of order | no match | `test_fzy::short_needles_stay_strict` |
| 4 | Excessive typos rejected | > 25% of chars unmatched | scoring | `SCORE_MIN` | `test_fzy::too_many_typos_do_not_match` |
| 5 | Frequency never saturates | ranks 100 vs 200 | computing frecency | exact 2× ratio preserved | `test_frecency::frequency_never_saturates` |
| 6 | Habit breaks ties | two equal-quality matches | one visited often/recently | frequent dir wins | `test_query::equal_match_quality_falls_back_to_frecency` |
| 7 | Quality beats habit | exact match vs high-rank sloppy match | query | exact match wins | `test_query::better_match_quality_beats_higher_frecency` |
| 8 | Aging bounds the database | total rank above `max_age` | insert | ranks ×0.9, entries < 1.0 dropped | `test_dir_list::aging_decays_ranks_and_drops_negligible_entries` |
| 9 | Dead paths never surface | recorded dir deleted from disk | query | candidate excluded | `test_query::query_skips_nonexistent_paths` |
| 10 | Data round-trips with other tools | entries exported | re-imported | ranks/epochs preserved, merge keeps max/newest | `test_db::import_merges_keeping_higher_rank_and_newer_access` |

## Open Questions

- **Skip threshold tuning.** `allowed_skips = n/4` and `SCORE_SKIP_NEEDLE
  = -1.0` are principled defaults, not empirically tuned. If false positives
  appear in daily use, the gate ratio is the first knob to tighten.
- **`exclude_dirs` is parsed but unused.** The config option exists and is
  validated, but no code path consults it during insert. Wiring it into the
  chpwd insert path is future work.
- **Aging trigger frequency.** Aging runs on every insert (a cheap sum over
  in-memory entries). If the database grows very large this could move to a
  save-time check, but at typical sizes (hundreds of entries) it is
  negligible.
