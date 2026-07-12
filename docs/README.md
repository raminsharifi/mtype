# mtype documentation

[Back to the project README](../README.md)

This is the complete user and contributor documentation for mtype. The terminal
test, local history, adaptive practice, browser dashboard, configuration, and
content system are covered separately so each page can work as both a tutorial
and a reference.

## User guides

| Guide | What it covers |
| --- | --- |
| [Installation](installation.md) | Supported release binaries, source installation, PATH setup, macOS quarantine, updating, and uninstalling |
| [Usage](usage.md) | Test lifecycle, all six modes, test/results/stats/editor controls, modifiers, and recommended workflows |
| [CLI reference](cli-reference.md) | Every flag, subcommand, argument, default, precedence rule, exit behavior, and example |
| [Configuration](configuration.md) | Workspace navigation and every persistent setting, allowed value, default, and restart effect |
| [Scoring and results](scoring-and-results.md) | WPM/raw/accuracy/consistency formulas, character classes, difficulty, fail conditions, PB rules, history, and replay |
| [Stats and practice](stats-and-practice.md) | Terminal profile, browser dashboard metrics and insight thresholds, wrong words, confusions, and adaptive ranking |
| [Data and privacy](data-and-privacy.md) | Platform paths, JSON and SQLite schemas, saving rules, export/import/reset semantics, and network boundaries |
| [Custom content](custom-content.md) | All bundled lists/themes/funboxes, synced languages and quotes, JSON formats, and custom theme fields |
| [Troubleshooting](troubleshooting.md) | Installation, terminal, config, dashboard, history, content, and recovery problems |

## Contributor guide

[Development](development.md) documents the crate structure, runtime flow,
testing layers, embedded assets, database behavior, release process, and the
source files responsible for each feature.

## Suggested reading paths

### New user

1. [Install mtype](installation.md).
2. Learn the [test and results controls](usage.md).
3. Open the [configuration workspace](configuration.md).
4. Save several tests, then use [stats and adaptive practice](stats-and-practice.md).

### Advanced user

1. Read the [scoring definitions](scoring-and-results.md) before comparing metrics.
2. Review [data and privacy](data-and-privacy.md) before scripting backups or inspecting SQLite.
3. Add [custom content and themes](custom-content.md).
4. Keep the [CLI reference](cli-reference.md) available for automation.

### Contributor

1. Read [Development](development.md).
2. Review [scoring and results](scoring-and-results.md) before changing the engine.
3. Review [data and privacy](data-and-privacy.md) before changing persistence.
4. Run the complete verification suite documented in the contributor guide.
