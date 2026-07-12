# Local data, schemas, privacy, backup, import, and reset

[Documentation index](README.md) · [Stats and practice](stats-and-practice.md) · [Troubleshooting](troubleshooting.md)

## Privacy model

mtype has no account, authentication, cloud profile, remote analytics,
telemetry, advertising, or background sync. Typing input, custom text, results,
mistakes, replays, settings, presets, and dashboard analysis remain on the local
machine.

The only intentional network command is `mtype sync`, which downloads a named
language or quote JSON file from the Monkeytype repository. The browser
dashboard is local networking only: it binds IPv4 loopback and loads embedded
assets from the same origin.

## Platform directories

mtype uses `directories::ProjectDirs::from("com", "monkeytype", "mtype")`.

### macOS

Config and application data resolve under:

```text
~/Library/Application Support/com.monkeytype.mtype/
```

### Linux

```text
~/.config/mtype/
~/.local/share/mtype/
```

`config.toml` uses the config root. Results, metadata, SQLite, presets, themes,
languages, and quotes use the data root. `XDG_CONFIG_HOME` and `XDG_DATA_HOME`
override the default Linux roots.

## File inventory

| File/folder | Format | Purpose | Included by data export |
| --- | --- | --- | --- |
| `config.toml` | TOML | Complete persistent user configuration | No |
| `results.json` | Compact JSON array | Primary readable test history, word outcomes, and replay input events | Yes, transformed into the export bundle |
| `meta.json` | JSON object | Started-test counter | Yes |
| `analytics.sqlite3` | SQLite 3, WAL mode | Normalized sessions, words, and input events | No; rebuilt from exported result detail on import |
| `analytics.sqlite3-wal` / `-shm` | SQLite sidecars | WAL transaction state while database is active | No |
| `presets.json` | Pretty JSON map | Named config slots `slot 1` through `slot 5` | No |
| `themes/*.toml` | TOML | User-defined terminal themes | No |
| `languages/*.json` | Monkeytype JSON | Synced language word lists | No |
| `quotes/*.json` | Monkeytype JSON | Synced quote collections | No |

## `results.json` result fields

Each stored result contains:

| Field | Meaning |
| --- | --- |
| `wpm` | Final net WPM |
| `raw_wpm` | Final raw WPM |
| `acc` | Keystroke accuracy percentage |
| `consistency` | Burst consistency score |
| `mode` | Mode string |
| `mode2` | Duration, count, quote ID, `zen`, `custom`, or practice source |
| `punctuation`, `numbers` | Category modifiers |
| `language` | Active language name |
| `difficulty` | Difficulty string |
| `duration_sec` | Final elapsed seconds |
| `timestamp_ms` | Unix epoch milliseconds assigned when saving |
| `char_correct`, `char_incorrect`, `char_extra`, `char_missed` | Final character classes |
| `word_outcomes` | Detailed reached words |
| `input_events` | Replayable input log |

Newer detail fields use Serde defaults so older results can still load. Older
records without word outcomes remain useful for lifetime metrics but cannot
fully reconstruct word analytics.

## Word outcome schema

| Field | Meaning |
| --- | --- |
| `word_index` | Zero-based test position |
| `target` | Expected word |
| `typed` | Final typed form |
| `preceding_word` | Previous target when one exists |
| `duration_ms` | Time from word start to outcome capture |
| `burst_wpm` | Word-level physical speed |
| `correct` | Final exact correctness |
| `had_error` | Final error or any corrected error |
| `incorrect_keystrokes` | Count of incorrect character attempts |
| `char_correct`, `char_incorrect`, `char_extra`, `char_missed` | Word character classes |

## Input event schema

| Field | Meaning |
| --- | --- |
| `elapsed_ms` | Milliseconds since the test's first character |
| `word_index` | Active word when the event occurred |
| `kind` | `character`, `commit`, `backspace`, or `word_backspace` |
| `value` | Character string for character events; null otherwise |
| `correct` | Attempt correctness for character/commit events where applicable |

Because incorrect attempts and backspaces are retained, saved data can contain
text that was corrected before final submission.

## Started-test metadata

`meta.json` contains `started_tests`. The application increments it once when a
test first enters the running state. The profile uses the greater of this value
and result-history length, preventing completed from exceeding started after an
older or partial metadata import.

## SQLite initialization

Opening analytics enables foreign keys and WAL journal mode. Schema version 1
creates one metadata table, three normalized data tables, four indexes, and one
aggregate view.

### `schema_meta`

| Column | Type |
| --- | --- |
| `version` | INTEGER NOT NULL |

### `test_sessions`

| Column | Type/constraint | Meaning |
| --- | --- | --- |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT | Internal session key |
| `external_id` | TEXT NOT NULL UNIQUE | Save timestamp string, used for idempotency |
| `occurred_at_ms` | INTEGER NOT NULL | Unix epoch milliseconds |
| `mode`, `mode2`, `language`, `difficulty` | TEXT NOT NULL | Test category/context |
| `punctuation`, `numbers` | INTEGER NOT NULL | Boolean modifiers |
| `duration_sec` | REAL NOT NULL | Duration |
| `wpm`, `raw_wpm`, `accuracy`, `consistency` | REAL NOT NULL | Result metrics |
| `failed` | INTEGER NOT NULL | Failure flag; normal save flow records only valid nonfailed results |

### `word_events`

| Column | Type/constraint |
| --- | --- |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `test_id` | INTEGER NOT NULL, foreign key with cascade delete |
| `word_index` | INTEGER NOT NULL |
| `target_word`, `typed_word` | TEXT NOT NULL |
| `preceding_word` | TEXT nullable |
| `duration_ms` | INTEGER NOT NULL |
| `burst_wpm` | REAL NOT NULL |
| `correct`, `had_error` | INTEGER NOT NULL |
| `incorrect_keystrokes` | INTEGER NOT NULL |
| `char_correct`, `char_incorrect`, `char_extra`, `char_missed` | INTEGER NOT NULL |

### `input_events`

| Column | Type/constraint |
| --- | --- |
| `id` | INTEGER PRIMARY KEY AUTOINCREMENT |
| `test_id` | INTEGER NOT NULL, foreign key with cascade delete |
| `sequence` | INTEGER NOT NULL |
| `elapsed_ms`, `word_index` | INTEGER NOT NULL |
| `kind` | TEXT NOT NULL |
| `value` | TEXT nullable |
| `correct` | INTEGER nullable |

### Indexes

- `idx_tests_occurred` on session timestamp descending;
- `idx_words_target` on target word;
- `idx_words_errors` on error/final correctness;
- `idx_inputs_test_sequence` on session and sequence.

### `word_practice_stats` view

The view groups `word_events` by session language and target word. It exposes
attempt count, error attempts, final misses, average positive burst, and newest
timestamp. Adaptive practice queries this view.

## Analytics transaction and idempotency

One database transaction inserts a session, then all word events, then all input
events. `external_id` is the result timestamp. `INSERT OR IGNORE` makes a repeated
record attempt idempotent: if the session timestamp already exists, no duplicate
children are inserted.

Integer conversions saturate at SQLite's signed 64-bit maximum rather than
overflowing.

JSON history is saved before SQLite recording. Analytics failure is intentionally
best-effort so a database issue cannot lose the human-readable result or break
the result screen.

## Export format

```sh
mtype data export mtype-backup.json
```

The exported top-level JSON object contains:

```json
{
  "version": 1,
  "results": [],
  "started_tests": 0
}
```

`results` contains the full stored-result objects described above. The export
is pretty-printed and overwrites the destination file. Its parent directory is
not created automatically.

Not exported:

- config;
- presets;
- raw SQLite database and WAL sidecars;
- custom themes;
- synced languages or quotes.

For a complete machine-state backup, copy the config and data directories after
stopping all mtype/dashboard processes in addition to creating a portable data
export.

## Import semantics

```sh
mtype data import mtype-backup.json
```

Import:

1. Reads and parses JSON.
2. Requires `version == 1`.
3. Appends imported results to local history.
4. Sorts by `timestamp_ms`.
5. Deduplicates equal timestamps.
6. Saves merged `results.json`.
7. Sets started count to the maximum of local count, imported count, and merged
   history length.
8. Replays every merged stored result that can be converted back to a test result
   into normalized analytics; SQLite idempotency prevents duplicate sessions.
9. Prints the number of newly added result timestamps.

Import is a merge, not a replacement. Config and non-result local assets remain
unchanged.

## Reset semantics

```sh
mtype data reset --yes
```

The required acknowledgement prevents accidental deletion. Reset removes:

- `results.json`;
- `meta.json`;
- `analytics.sqlite3` and SQLite sidecars as handled by the reset path.

It does not remove:

- `config.toml`;
- `presets.json`;
- custom themes;
- synced languages;
- synced quotes;
- exports stored elsewhere.

Reset cannot be undone without a backup/import.

## Manual inspection

`results.json` and exports can be inspected with `jq`. Stop mtype before editing;
manual mutation is unsupported.

```sh
jq 'length' results.json
jq '.results | length' mtype-backup.json
```

SQLite can be inspected read-only with the `sqlite3` CLI after stopping active
writers:

```sh
sqlite3 analytics.sqlite3 '.tables'
sqlite3 analytics.sqlite3 'SELECT COUNT(*) FROM test_sessions;'
sqlite3 analytics.sqlite3 'SELECT target_word, attempts, error_attempts, avg_burst_wpm FROM word_practice_stats ORDER BY error_attempts DESC LIMIT 20;'
```

Avoid deleting individual normalized rows unless you understand foreign keys,
the readable JSON source, and future import rebuilding. Use the supported reset
command for a consistent full history reset.

## Sensitive-data guidance

Typing history can include custom text, quote content, final typed mistakes,
corrected characters, and timing. Treat these files as personal data:

- review exports before sharing;
- do not attach SQLite, results, or config to public bug reports without
  redaction;
- protect backups with the same permissions as other personal documents;
- remember that localhost prevents remote binding but other local software under
  the same user/session may still access an actively served dashboard URL.
