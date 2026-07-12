# Complete CLI reference

[Documentation index](README.md) · [Usage](usage.md) · [Configuration](configuration.md)

## Command shape

```text
mtype [OPTIONS] [COMMAND]
```

Running without a command launches the TUI. Top-level flags modify the loaded
configuration for that process only; they are not persisted. Persistent changes
must be made in the config workspace or by editing `config.toml`.

```sh
mtype
mtype --time 60 --punctuation
mtype stats
mtype stats serve
mtype practice missed --words 25
mtype data export backup.json
mtype sync language spanish
```

## Global informational flags

| Flag | Effect |
| --- | --- |
| `-h`, `--help` | Print the full top-level help and exit successfully |
| `-V`, `--version` | Print the package version and exit successfully |

Subcommands also support `-h` and `--help`.

## Test options

| Option | Value | Effect |
| --- | --- | --- |
| `--mode` | `time`, `words`, `quote`, `zen`, `custom`, `practice` | Select the test mode for this process; `practice` is accepted by the config enum even though the short help text lists the five general modes |
| `--time` | Unsigned integer seconds | Set the timer and imply `--mode time` |
| `--words` | Unsigned integer count | Set the target count and imply `--mode words` |
| `--punctuation` | No value | Enable punctuation for this process |
| `--numbers` | No value | Enable number injection for this process |
| `--difficulty` | `normal`, `expert`, `master` | Select the difficulty for this process |
| `--custom` | Text string | Set custom text and imply `--mode custom` |
| `--quote-id` | Unsigned integer ID | Select a quote ID and imply `--mode quote` |
| `--language` | Language name | Select a bundled or synced language |

Examples:

```sh
mtype --mode time --time 15
mtype --time 120 --numbers --punctuation
mtype --mode words --words 100 --difficulty expert
mtype --mode quote --language english
mtype --quote-id 42
mtype --mode zen
mtype --custom "correctness before speed"
mtype --language english_25k --words 50
```

### Option precedence

The parser applies options in this order: explicit mode, time, words,
punctuation, numbers, difficulty, custom text, quote ID, and language. Options
that imply a mode can therefore override an earlier `--mode`.

Examples:

- `mtype --mode quote --time 30` starts time mode because `--time` implies it.
- `mtype --time 30 --words 50` starts words mode because words is applied later.
- `mtype --custom "abc" --quote-id 7` starts quote mode because quote ID is
  applied after custom text.

Use one mode-implying option per command unless the precedence is intentional.

Invalid strings supplied to `--mode` or `--difficulty` do not become valid
enum values; mtype retains the corresponding loaded configuration value. A
language name that is neither bundled nor synced prints a note and falls back
to the base English word list.

`--mode practice` uses the saved practice source/count and regenerates its local
analytics target during App startup. The dedicated `mtype practice` command is
clearer when source or length should be supplied explicitly.

`sync`, `data`, and `stats serve` take early non-TUI paths before config loading
and test-option application. Top-level test flags have no effect on those
commands. Plain `mtype stats` and `mtype practice` do enter the normal config/TUI
path.

## `mtype stats`

```text
mtype stats [COMMAND]
```

With no nested command, mtype opens the terminal stats screen directly. Normal
config loading and top-level test options still occur before the TUI opens.

```sh
mtype stats
```

### `mtype stats serve`

Starts the local browser dashboard and blocks until Ctrl+C stops the server.

```text
mtype stats serve [--port PORT] [--no-open]
```

| Option | Default | Effect |
| --- | --- | --- |
| `--port PORT` | `4242` | Bind `127.0.0.1:PORT` |
| `--no-open` | Off | Print the URL without launching the default browser |

```sh
mtype stats serve
mtype stats serve --port 5050
mtype stats serve --port 0 --no-open
```

Port `0` asks the operating system to choose a free loopback port; mtype prints
the selected address. Dashboard assets and API responses are served by the
local process. See [Stats and practice](stats-and-practice.md).

## `mtype practice`

```text
mtype practice [KIND] [--words WORDS]
```

| Argument or option | Default | Values and effect |
| --- | --- | --- |
| `KIND` | `mixed` | `missed`, `slow`, or `mixed`; an unknown string falls back to `mixed` |
| `--words WORDS` | `25` | Requested practice length; values are clamped to at least 1 by CLI application |

```sh
mtype practice
mtype practice missed
mtype practice slow --words 10
mtype practice mixed --words 100
```

Practice filters historical words by the active language. If analytics does not
produce any candidates, the test falls back to random words from that language.

## `mtype data`

```text
mtype data <COMMAND>
```

Data commands do not open the TUI.

### Export

```text
mtype data export <PATH>
```

Writes a versioned, pretty-printed JSON bundle containing results, word
outcomes, replay events, and the started-test counter.

```sh
mtype data export mtype-backup.json
mtype data export "$HOME/backups/mtype-$(date +%F).json"
```

The parent directory must already exist. Existing files at the path are
replaced.

### Import

```text
mtype data import <PATH>
```

Reads a version-1 mtype export, merges results, removes duplicate timestamps,
updates the started-test counter, and rebuilds normalized SQLite analytics from
the merged history.

```sh
mtype data import mtype-backup.json
```

The command prints the number of new results added. Config, presets, themes,
and synced content are not part of the import.

### Reset

```text
mtype data reset --yes
```

`--yes` is mandatory. Without it, mtype exits with an error and changes
nothing. Reset removes results, the started-test counter, and analytics; it does
not remove config, presets, themes, or synced content.

## `mtype sync`

```text
mtype sync <KIND> <NAME>
```

| Argument | Values |
| --- | --- |
| `KIND` | `language` or `quotes`; accepted aliases are `languages`, `lang`, and `quote` |
| `NAME` | Monkeytype content name, such as `spanish`, `french`, or `code_rust` |

```sh
mtype sync language spanish
mtype sync language code_rust
mtype sync quotes french
```

This is the only public mtype command that intentionally accesses the network.
It downloads JSON from the Monkeytype repository into the platform data
directory. Requests have a 20-second timeout, downloaded JSON is parsed as the
expected language/quote schema before it is written, and the content is then
used offline.

## Exit status and errors

- Successful informational, data, sync, and server setup paths return status 0.
- CLI syntax errors are reported by Clap and return a nonzero status.
- Missing files, invalid export JSON, unsupported export versions, reset without
  `--yes`, content download failures, and dashboard bind failures return a
  nonzero status with an explanatory error.
- The interactive TUI restores the terminal before returning its runtime result.

## Diagnostic word generation

The binary has a hidden `--dump-words` developer option. It applies the current
test options, prints one generated batch separated by spaces, and exits without
opening the TUI:

```sh
mtype --language english_1k --words 25 --dump-words
```

This flag is useful for deterministic behavior checks around available content,
punctuation, numbers, and word generation. It is hidden from normal CLI help and
is not a stable end-user interface.
