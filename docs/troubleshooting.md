# Troubleshooting

[Documentation index](README.md) · [Installation](installation.md) · [Data and privacy](data-and-privacy.md)

## `mtype: command not found`

Find the executable and confirm its directory is on PATH:

```sh
command -v mtype
echo "$PATH"
```

Common install locations are `/usr/local/bin/mtype`, `~/.local/bin/mtype`, and
`~/.cargo/bin/mtype`. For a Cargo installation, load the environment:

```sh
source "$HOME/.cargo/env"
```

For a user-local binary, add this to the shell startup file:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

## macOS says the application cannot be opened

The release binary is unsigned. Remove the quarantine attribute:

```sh
xattr -d com.apple.quarantine /path/to/mtype
chmod +x /path/to/mtype
```

Confirm that an Apple Silicon binary is being used on an Apple Silicon Mac:

```sh
uname -m
file /path/to/mtype
```

The expected machine value is `arm64`; the release asset is
`mtype-macos-arm64`. There is no Intel macOS release asset.

## Linux reports permission denied

```sh
chmod +x /path/to/mtype
```

If the file is on a filesystem mounted with `noexec`, move it to
`~/.local/bin` or `/usr/local/bin`.

## The terminal display is clipped or says it is too small

Increase the terminal window size. The test screen needs at least 20 columns by
4 rows for its normal layout. Stats needs at least 30 columns by 10 rows. Config
has a compact mode but still needs 28 columns by 9 rows to render its workspace.

Reduce terminal font size or disable `show all lines` if the target text consumes
too much vertical space. `max line width` controls the word block width, not the
physical terminal size.

## Colors are missing or inaccurate

mtype uses terminal true color. Use a terminal with 24-bit color support and
check:

```sh
echo "$COLORTERM"
```

Theme colors are emitted through Ratatui/Crossterm; the terminal emulator and
its color settings ultimately determine the displayed result. An invalid custom
theme silently falls back to `serika_dark`.

## Esc restarts instead of opening config

`quick restart` may be set to `esc`. Change it through another config route:

1. Finish the current test.
2. Open config from the results screen with Esc.
3. Go to Behavior → Quick restart.
4. Select `off`, `tab`, or `enter`.

The `no_quit` funbox can also block leaving or restarting a running test. Finish
the test before clearing Behavior → Funbox → Clear all.

## Backspace does not work

Check Behavior → Confidence mode:

- `max` disables all backspacing.
- `on` allows editing the active word but prevents returning to previous words.
- `off` allows returning to an incorrect previous word.

Returning to a correct committed word still requires `freedom mode`.

## Space will not submit a word

Behavior → Stop on error may be set to `word`. In that mode, an incorrect word
must be fixed before Space can commit it. `strict space` also makes empty spaces
meaningful instead of ignoring them. Expert difficulty fails after an imperfect
word is submitted; master difficulty fails on the first incorrect key.

## A requested language falls back to English

Check whether the name is bundled in [Custom content](custom-content.md). For
additional content:

```sh
mtype sync language spanish
mtype --language spanish --mode words
```

Synced files live under the platform data directory in `languages/`. The file
must be valid Monkeytype language JSON and its filename must match the requested
name.

## Quote mode uses English quotes

Quote collections are separate from word lists. Sync the quote collection with
the base language name:

```sh
mtype sync quotes french
```

Sized word-list suffixes such as `_1k` are removed when choosing a quote
collection. If no local quote file is available, mtype falls back to the
embedded English quotes.

## No results appear in stats

A result is saved only when all of these are true:

- Result saving is on.
- The test did not fail.
- Duration is at least one second.
- At least one scored character was typed.

Failed expert/master/minimum tests and extremely short tests are not stored.
Check Feedback → Result saving, then complete a normal test lasting more than
one second.

## Wrong words or practice are empty

Word analytics require newly saved tests containing word outcomes. Practice is
language-specific, so changing the active language can produce an empty
candidate set even when another language has history. When no candidates exist,
practice falls back to random words.

Older imported history without word outcomes can contribute to lifetime stats
without producing detailed word analytics.

## Dashboard will not start

The default port may already be in use:

```sh
mtype stats serve --port 5050
```

Or let the operating system choose a port:

```sh
mtype stats serve --port 0 --no-open
```

mtype prints the selected loopback URL. If automatic browser opening fails, use
`--no-open` and copy that URL manually.

## Dashboard opens but shows an error

Check that local data paths are readable and that the process can create or open
`analytics.sqlite3`. The dashboard API initializes the database schema when the
database is absent. It reads lifetime history from `results.json` and word-level
details from SQLite.

Use the health route while the server is running:

```sh
curl http://127.0.0.1:4242/api/health
```

The expected response is `{"ok":true}`. A different port must be substituted
when `--port` was used.

## Import reports zero new results

Import deduplicates by `timestamp_ms`. Zero means every result timestamp in the
bundle already existed locally. Import still keeps the larger started-test
counter and rebuilds analytics for the merged history.

## Reset refuses to run

The explicit acknowledgement is required:

```sh
mtype data reset --yes
```

Reset is irreversible unless an export exists. Create a backup first when the
history may be needed later.

## Config changes are not retained

CLI flags apply only to the current process. Use the config workspace for
persistent settings. If `config.toml` cannot be written, check permissions on
the platform config directory.

Unreadable or invalid TOML causes mtype to load defaults. Unknown keys are
tolerated, and missing known fields receive defaults. To recover from a broken
file, move it aside and start mtype to generate settings on the next saved
change:

```sh
mv /path/to/config.toml /path/to/config.toml.bak
mtype
```

## Recover from damaged local history

Before modifying files manually, stop every mtype and dashboard process and
copy the entire config/data directory. `results.json` is the primary readable
history; SQLite is best-effort normalized analytics. A valid JSON export is the
safest recovery source:

```sh
mtype data import mtype-backup.json
```

If history should be discarded, use `mtype data reset --yes` rather than
deleting individual database tables.

## Collect diagnostic information

Useful non-sensitive diagnostics include:

```sh
mtype --version
uname -a
echo "$TERM"
echo "$COLORTERM"
mtype --help
```

Do not publish `results.json`, `analytics.sqlite3`, exports, or custom-text
config without reviewing them; they can contain everything typed during saved
tests.
