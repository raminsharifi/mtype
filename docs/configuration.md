# Configuration workspace and complete setting reference

[Documentation index](README.md) · [Usage](usage.md) · [Scoring](scoring-and-results.md) · [Data paths](data-and-privacy.md)

## Persistence model

mtype loads `config.toml` at process start. The schema uses Serde defaults:
missing known fields receive defaults, unknown fields are tolerated, and an
unreadable or invalid file causes the entire configuration load to fall back to
defaults. Config actions save the complete configuration after applying.

Top-level CLI flags modify the loaded configuration only in memory. They do not
rewrite `config.toml`.

The following fields are transient and are not serialized:

- `quote_id`, used for a command-line-selected quote;
- `practice_text`, generated from local analytics for the current test.

## Open and navigate config

Press Esc during a test or from the result summary. Config pauses timed ticking
and overlays the underlying screen.

| Key | Action |
| --- | --- |
| Tab / Shift+Tab | Next or previous tab |
| Right / Left | Next tab; Left exits a drill-down before moving tabs |
| 1 through 6 | Jump to Current, Test, Behavior, Appearance, Feedback, or System from a root view |
| Up / Down | Move selection; on Current, scroll the snapshot |
| Ctrl+J / Ctrl+K | Move down or up |
| Enter | Apply a single-action setting or open/apply a multi-value setting |
| Backspace | Remove one filter character; with no filter, exit a drill-down |
| Esc | Close config |

Boolean toggles apply from the category root with one Enter. Multi-value rows
open a choice list. Applying a value keeps config open and preserves the active
tab, row, query, and drill-down. Values and active markers refresh immediately.

Actions that intentionally navigate away—Stats, Custom text, and Quit—close the
workspace.

## Search behavior

Typing in Test, Behavior, Appearance, Feedback, or System fuzzy-matches concrete
commands within that tab. Typing from Current searches every tab. Search uses a
case-insensitive subsequence match and ignores spaces in the query. A result is
a concrete action, so Enter applies it directly.

Examples:

- `punc` matches punctuation.
- `mowo` matches `mode > words`.
- `theme nord` matches the Nord theme option.

Backspace to an empty query returns to the normal tab view.

## Current tab

Current is a read-only snapshot grouped into Test, Behavior, Appearance, and
Feedback. It displays every persistent setting plus custom-text status. On wide
terminals it uses three columns, then two columns, then one compact column as
width decreases. Up and Down scroll when all rows do not fit.

## Test tab

Settings marked **restart** create a fresh engine/test behind the open config
workspace.

**Restart: No** means the setting action does not automatically replace the
current engine. Display settings read by the renderer can appear immediately.
Engine-owned behavior such as Freedom mode or Strict space is guaranteed on the
next fresh test because the Engine holds a configuration clone created at
restart.

| Setting | Choices | Default | Effect | Restart |
| --- | --- | --- | --- | --- |
| Mode | `time`, `words`, `quote`, `zen`, `custom`, `practice` | `time` | Selects the target and finish model | Yes |
| Time | 15, 30, 60, 120 seconds | 30 | Sets time duration and switches to time mode | Yes |
| Words | 10, 25, 50, 100 | 50 | Sets finite count and switches to words mode | Yes |
| Practice | missed/slow/mixed × 10/25/50/100 words | mixed, 25 | Selects analytics ranking and switches to practice mode | Yes |
| Quote length | all, short, medium, long, thicc | medium | Chooses one band or the union of all bands and switches to quote mode | Yes |
| Language | Every bundled and synced language | `english` | Selects the pool and language-specific analytics | Yes |
| Difficulty | `normal`, `expert`, `master` | `normal` | Controls immediate failure rules | Yes |
| Punctuation | on/off | off | Enables random punctuation in generated pool modes | Yes |
| Numbers | on/off | off | Enables probabilistic numeric targets in generated pool modes | Yes |

Mode-specific stored fields remain available while another mode is active. For
example, time can remain 30 while words mode is active; selecting Time 60 both
changes the stored duration and activates time mode.

## Behavior tab

| Setting | Choices | Default | Effect | Restart |
| --- | --- | --- | --- | --- |
| Stop on error | `off`, `letter`, `word` | `off` | Off accepts errors; letter rejects an incorrect character; word rejects Space until the word is correct | Yes |
| Confidence mode | `off`, `on`, `max` | `off` | On blocks returning to previous words; max disables all backspace | Yes |
| Strict space | on/off | off | When off, Space on an empty word is ignored; when on, the empty commit participates in normal word handling | No |
| Quick restart | `off`, `esc`, `tab`, `enter` | `off` | Stores the preferred restart trigger; Esc is implemented on the test screen, while Tab already restarts and Enter is reserved by mode behavior | No |
| Quick end | on/off | off | Allows words mode to finish when the last typed word reaches target length even if imperfect | Yes |
| Freedom mode | on/off | off | Allows backspacing from an empty word into a correct previous word; without it, only an incorrect previous word is eligible | No |
| Blind mode | on/off | off | Renders typed characters as visually correct even when they are not; scoring still records actual correctness | No |
| Lazy mode | on/off | off | Compares Unicode characters after removing combining marks; useful for accent-insensitive matching | Yes |
| British English | on/off | off | Maps a fixed set of generated American spellings to British forms | Yes |
| Hide extra letters | on/off | off | Blocks insertion beyond target length while retaining the attempt in input/error history; the blocked key is not added to the accuracy-keystroke series | No |
| Repeat quotes | on/off | off | On restart, retains the current quote ID when quote mode is active | No |
| Funbox | Clear all or toggle any supported funbox | none | Replaces/transforms generated words or changes restart behavior | Yes |

### Confidence and previous-word editing

At the start of a word:

- max confidence blocks the backspace before any edit;
- on confidence prevents returning to previous words;
- off confidence can return to an incorrect previous word;
- freedom mode additionally permits returning to a correct previous word.

Re-entering a committed word removes its prior committed/burst record so the
final result reflects the revised path.

## Appearance tab

| Setting | Choices | Default | Effect | Restart |
| --- | --- | --- | --- | --- |
| Theme | 8 bundled themes plus valid local TOML themes | `serika_dark` | Replaces the terminal color palette immediately | No |
| Caret style | `off`, `default`, `block`, `outline`, `underline` | `default` | Controls the active input cell; default and outline are terminal approximations using an underlined caret-colored cell | No |
| Smooth caret | `off`, `slow`, `medium`, `fast` | `medium` | Adds no blink, slow blink, or rapid blink terminal modifiers | No |
| Highlight mode | `off`, `letter`, `word`, `next_word` | `letter` | Chooses the active/next target emphasis behavior | No |
| Indicate typos | `off`, `below`, `replace`, `both` | `off` | Chooses whether wrong typed/target glyphs are shown inline, below, or in both positions | No |
| Max line width | full, 40, 60, 80, 100, 120 | full (`0`) | Caps the centered target width; full uses available terminal width | No |
| Show all lines | on/off | off | Starts target rendering at the first line instead of a moving active-line window | No |
| Colorful mode | on/off | off | Uses the theme's brighter colorful error palette | No |
| Flip test colors | on/off | off | Swaps normal target/typed text color roles | No |

### Typo indicator rendering

- **off** and **replace** currently render the typed incorrect character inline
  in the error color.
- **below** keeps the target inline in the error color and places the typed
  character below.
- **both** keeps the typed character inline and places the target below.

Committed missing characters use the error color. Extra characters use the
error-extra palette unless Hide extra letters suppresses insertion.

## Feedback tab

| Setting | Choices | Default | Effect | Restart |
| --- | --- | --- | --- | --- |
| Timer / progress | `off`, `text`, `bar`, `mini` | `mini` | Shows remaining time, finite word progress, or Zen label | No |
| Live speed | `off`, `text`, `bar`, `mini` | `off` | Shows live net WPM; mini removes the `wpm` label | No |
| Live accuracy | `off`, `text`, `bar`, `mini` | `off` | Shows keystroke accuracy; mini removes the `acc` label | No |
| Live burst | `off`, `text`, `bar`, `mini` | `off` | Shows the last committed word burst; current renderer uses one textual format for non-off choices | No |
| Speed unit | `wpm`, `cpm`, `wps`, `cps`, `wph` | `wpm` | Converts displayed speed values; stored/scored base values remain WPM | No |
| Pace caret | `off`, `average`, `pb`, `last`, `custom` | `off` | Displays a comparison caret moving at a resolved historical/custom speed | Yes |
| Pace speed | 30, 60, 90, 120, 150, 200 WPM | stored default 100 | Sets custom pace speed and selects custom pace | Yes |
| Pace style | Same five caret styles | `default` | Controls the comparison caret rendering | No |
| Minimum WPM | off, 20, 40, 60, 80, 100, 120 | off | Fails once live net WPM is below the threshold after one second | Yes |
| Minimum accuracy | off, 80, 90, 95, 98, 100% | off | Fails once live keystroke accuracy is below the threshold after one second | Yes |
| Minimum burst | off, 20, 40, 60, 80, 100, 120 WPM | off | Fails when the most recently committed burst is below the threshold | Yes |
| Result saving | on/off | on | Controls JSON history, PB evaluation, and SQLite analytics recording | No |
| Start graphs at zero | on/off | on | Sets the result/stats graph lower Y bound; off uses 90% of the observed minimum where supported | No |
| Always show decimals | on/off | off | Forces two decimal places in result metrics; otherwise integers display without decimals | No |
| Out of focus warning | on/off | on | Shows a centered focus warning when the terminal loses focus | No |
| Caps lock warning | on/off | on | Shows a centered warning when terminal key events report Caps Lock | No |

For live speed and live accuracy, `bar` currently renders the same labeled
numeric form as `text`; `mini` removes the label. Live burst currently uses one
textual form for every non-off style. Timer/progress is the indicator that uses
an actual bar when `bar` is selected.

### Pace source resolution

Historical pace matching requires the same mode, language, punctuation, and
numbers. It does not additionally filter mode duration/count or difficulty.

- **last** uses the newest matching result.
- **pb** uses the highest matching WPM.
- **average** uses the average of up to the 10 newest matching results.
- **custom** uses the stored custom speed.
- A historical source with no matching result produces no pace caret.

The pace position advances by `WPM × 5 × elapsed seconds / 60` target
characters, including one inter-word space per word.

### Speed unit conversions

| Unit | Conversion from WPM |
| --- | --- |
| WPM | unchanged |
| CPM | WPM × 5 |
| WPS | WPM ÷ 60 |
| CPS | WPM × 5 ÷ 60 |
| WPH | WPM × 60 |

## System tab

| Action | Behavior |
| --- | --- |
| Stats / progress | Closes config and opens the local terminal profile |
| Custom text | Closes config and opens the editor with the stored custom text |
| Presets → save → slot 1–5 | Saves the complete current configuration and keeps config open |
| Presets → load → slot 1–5 | Appears for saved slots, replaces the complete configuration, restarts the test, and keeps config open |
| Quit mtype | Ends the application |

Preset files are local and are not included by `mtype data export`. Loading a
preset includes its mode, theme, behavior, feedback, custom text, and all other
serialized fields.

## Direct TOML editing

The application writes a complete pretty-printed TOML representation. Manual
editing is possible while mtype is not running. Enum values use the strings in
the tables above; option values for disabled minimums are omitted/represented by
TOML's serialized optional behavior when mtype writes the file.

Prefer the config workspace because it constrains choices and refreshes the
engine/theme correctly. Keep a backup before editing by hand. See
[Data and privacy](data-and-privacy.md) for the exact path.

## Complete default TOML

Optional minimum fields are absent while set to off. A newly saved default
configuration otherwise corresponds to:

```toml
mode = "time"
time = 30
words = 50
quote_length = ["medium"]
custom_text = ""
practice_mode = "mixed"
practice_word_count = 25
punctuation = false
numbers = false
language = "english"
difficulty = "normal"
freedom_mode = false
confidence_mode = "off"
stop_on_error = "off"
strict_space = false
quick_end = false
quick_restart = "off"
blind_mode = false
lazy_mode = false
british_english = false
indicate_typos = "off"
hide_extra_letters = false
funbox = []
caret_style = "default"
smooth_caret = "medium"
pace_caret = "off"
pace_caret_custom_speed = 100
pace_caret_style = "default"
timer_style = "mini"
live_speed_style = "off"
live_acc_style = "off"
live_burst_style = "off"
theme = "serika_dark"
highlight_mode = "letter"
flip_test_colors = false
colorful_mode = false
show_all_lines = false
max_line_width = 0
typing_speed_unit = "wpm"
start_graphs_at_zero = true
always_show_decimal_places = false
show_out_of_focus_warning = true
caps_lock_warning = true
repeat_quotes = false
result_saving = true
```

When enabled, optional minimum fields use integer values:

```toml
min_wpm = 60
min_acc = 95
min_burst = 40
```
