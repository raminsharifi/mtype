# Development and architecture

[Documentation index](README.md) · [Scoring](scoring-and-results.md) · [Data schemas](data-and-privacy.md)

## Toolchain and build

mtype is a Rust 2021 binary crate. Clone and run:

```sh
git clone https://github.com/raminsharifi/mtype
cd mtype
cargo run --release
```

Debug builds are faster to compile:

```sh
cargo run
```

Optimized release builds enable optimization level 3, link-time optimization,
and symbol stripping.

## Direct dependencies

| Crate | Responsibility |
| --- | --- |
| `ratatui` | Terminal layout, widgets, styling, charts, and test backend |
| `crossterm` | Raw terminal, alternate screen, keyboard/focus/paste events |
| `serde`, `serde_json`, `toml` | Config, history, export, content, preset, and theme serialization |
| `rand` | Word, quote, punctuation, numbers, and funbox randomness |
| `unicode-width` | Terminal width groundwork |
| `unicode-segmentation` | Unicode text support dependency |
| `unicode-normalization` | Lazy-mode accent normalization |
| `directories` | Platform config/data roots |
| `clap` | CLI parser and generated help |
| `anyhow`, `thiserror` | Application error handling |
| `ureq` | Explicit content sync HTTP client |
| `rusqlite` with bundled SQLite | Normalized local analytics |

## Runtime flow

1. `main.rs` parses CLI.
2. Sync, browser dashboard, and data commands take non-TUI early-return paths.
3. Normal/stat/practice TUI paths load config and apply temporary flags.
4. `tui.rs` enables raw mode and alternate screen.
5. `App` owns config, theme, engine, screen, result, profile, config workspace,
   editor, replay timing, and application lifecycle.
6. Crossterm events route by current screen. Config intercepts all input while
   open.
7. The engine processes explicit elapsed timestamps and exposes state/readouts.
8. On finish/failure, App builds a result, attempts persistence, and opens the
   results screen.
9. TUI restoration runs before `main` returns the application result.

## Source map

| Path | Responsibility |
| --- | --- |
| `src/main.rs` | CLI schema, temporary flag application, non-TUI dispatch, startup/shutdown |
| `src/tui.rs` | Terminal initialization and restoration |
| `src/app.rs` | State machine, event loop, screen controls, result synchronization |
| `src/config.rs` | Persistent config schema, enum strings/defaults, TOML I/O |
| `src/commandline.rs` | Six-tab config workspace, fuzzy search, action application, rendering |
| `src/engine.rs` | Input semantics, word commit/backspace, scoring, failures, outcomes/events |
| `src/numbers.rs` | WPM, rounding, mean, population standard deviation, consistency mapping |
| `src/wordgen.rs` | Mode sources, no-repeat selection, punctuation, numbers, British mappings, quote cleanup |
| `src/content.rs` | Embedded/local language and quote parsing/discovery/cache |
| `src/funbox.rs` | Supported generators, transforms, Zipf and no-quit behavior |
| `src/ui.rs` | Test/editor rendering, per-character styles, pace position, warnings |
| `src/results.rs` | Summary, graph, input history, replay |
| `src/stats.rs` | Terminal lifetime profile, history graph, heatmap, recent tests |
| `src/persistence.rs` | JSON history, validity, PBs, profile, streaks, export/import/reset |
| `src/analytics.rs` | SQLite schema/transactions, word view, adaptive ranking |
| `src/presets.rs` | Five local config slots |
| `src/theme.rs` | Bundled palettes, custom TOML loading, color parsing |
| `src/dashboard.rs` | Local HTTP server, API contract, aggregations, insights |
| `assets/dashboard.html` | Semantic dashboard structure and states |
| `assets/dashboard.css` | Responsive light/dark design, animation, reduced motion |
| `assets/dashboard.js` | API fetch, charts, filters, search, heatmap, interactions |
| `src/web.rs` | Explicit language/quote sync downloads |
| `assets/*.json` | Embedded English language and quote data |

## Engine design

Engine input methods accept `now_ms` rather than reading wall time. Tests can
therefore reproduce state exactly. The TUI passes milliseconds elapsed from an
App-owned `Instant`.

Important engine state:

- target and typed word arrays;
- active word index;
- before-start/running/finished/failed state;
- start/finish/word-start timestamps;
- keystroke records for accuracy/raw history;
- committed word records for net history;
- per-word bursts and incorrect-attempt counters;
- replayable input events;
- stateful word generator and RNG;
- optional selected quote metadata.

Character classification is a direct position-by-position port. Trailing spaces
are part of committed nonfinal targets. See [Scoring](scoring-and-results.md)
before changing this logic; small differences affect WPM, raw, accuracy,
history, PBs, dashboard analysis, and practice.

## Config workspace architecture

Every concrete `Action` maps to one `ConfigTab` and one setting group. Root tabs
render the intentional group order; multi-value groups drill into filtered
actions. Single-action groups execute immediately. Current renders the config
snapshot rather than action rows.

After an action applies, `CommandLine::refresh` rebuilds labels from the new
config while preserving context. Stats, custom editor, and quit are the only
actions classified as leaving the workspace. A reachability test verifies every
command appears in its tab's declared group list.

When adding a setting:

1. Add the config field/default/serialization type.
2. Add the action and `apply` outcome.
3. Add command choices and active markers.
4. Map action to a tab and group.
5. Add the group to intentional tab order.
6. Add Current value and summary row when persistent.
7. Document choices/default/restart effect.
8. Test application, navigation, rendering, and small terminals.

## Persistence architecture

`results.json` is primary readable history. Valid results are appended without
discarding old history. SQLite is a best-effort normalized derivative written
after JSON. Export serializes history detail, not the database file; import
merges history and rebuilds normalized rows idempotently.

Schema changes should:

- preserve older JSON through Serde defaults;
- update `SCHEMA_VERSION` and migration behavior deliberately;
- keep foreign keys and transaction atomicity;
- preserve corrected errors and event order;
- test duplicate session idempotency;
- update [Data and privacy](data-and-privacy.md).

## Dashboard architecture

The frontend is dependency-free and embedded with `include_str!`. The Rust
server is a small synchronous loopback HTTP listener. `/api/dashboard` loads
history/profile, opens/initializes SQLite, runs word queries, computes insights,
serializes camelCase JSON, and disables caching.

The browser app uses Canvas for the hero and progress charts, DOM rendering for
insights/tables/patterns/heatmap, ResizeObserver for chart redraw, and
IntersectionObserver for reveal motion. It respects reduced motion and system
light/dark preference.

When changing dashboard UI:

1. Keep assets self-contained and offline.
2. Update the Rust API struct/query and JS renderer together.
3. Preserve loading, empty, API-error, narrow, and reduced-motion states.
4. Run `node --check assets/dashboard.js`.
5. Start `mtype stats serve --no-open` and test the actual browser at desktop
   and mobile widths.
6. Check the browser console and API health route.

## Verification suite

Run all checks before committing:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
node --check assets/dashboard.js
git diff --check
./target/release/mtype --version
```

### Test layers

- Pure number/scoring tests verify reference formulas.
- Engine tests cover perfect/incorrect words, time completion, backspace,
  confidence, difficulties, and minimums.
- Word generation tests cover counts, no repeats, punctuation, numbers, quotes,
  Zen, and embedded content.
- Persistence tests cover validity, category PBs, profile/streak aggregation,
  and data behavior.
- Analytics tests use in-memory SQLite for normalized rows, idempotency, and
  adaptive ranking.
- Ratatui TestBackend tests render test, result, stats, config, and small
  terminals without requiring an interactive terminal.
- Dashboard tests cover API naming/static routes; live browser QA covers the
  visual and interaction layer.

Tests that simulate typing do not increment the developer's real started-test
metadata. Avoid adding unit paths that write platform user data.

## Documentation verification

When CLI/config/schema behavior changes:

- compare generated `--help` for every affected command;
- check every relative Markdown link;
- keep README concise and move detailed explanation here;
- update exact defaults, choices, formulas, paths, and destructive behavior;
- avoid documenting an intended behavior that differs from the implementation.

## Release process

The repository release workflow triggers when a GitHub release is published.
Its matrix builds:

- `x86_64-unknown-linux-gnu` as `mtype-linux-x86_64`;
- `aarch64-apple-darwin` as `mtype-macos-arm64`.

It does not build Intel macOS. Each job checks out the release tag, installs the
stable Rust target, runs `cargo build --release --target`, stages the binary, and
uploads it to the existing release with `--clobber`.

A release checklist should include:

1. Bump `Cargo.toml` and the mtype package entry in `Cargo.lock`.
2. Run the complete verification suite.
3. Ensure the worktree contains only intended changes.
4. Commit without unintended trailers/metadata.
5. Push the release commit.
6. Publish a tag/release pointing at that exact commit.
7. Monitor both workflow jobs to completion.
8. Verify release asset names, digests, tag SHA, and clean local worktree.

## Attribution and license

mtype is derived from Monkeytype. Maintain upstream notices when porting or
updating behavior/data. See [NOTICE.md](../NOTICE.md) for details and
[LICENSE](../LICENSE) for GPL-3.0-or-later terms.
