# Tests, modes, and controls

[Documentation index](README.md) · [CLI reference](cli-reference.md) · [Configuration](configuration.md) · [Scoring](scoring-and-results.md)

## Test lifecycle

1. mtype loads persistent config from `config.toml`.
2. Top-level CLI flags override that in memory for the current process.
3. The word generator selects the mode source, applies content modifiers, and
   creates the initial target.
4. The test waits in a before-start state. The timer begins on the first normal
   character, not when the screen first appears.
5. Character, commit, and backspace attempts are recorded with elapsed time and
   word position.
6. The mode ends naturally, the user finishes Zen, or a failure rule stops the
   test.
7. mtype calculates the result and, when valid and enabled, saves history and
   normalized analytics.
8. The results screen offers another test, stats, input history, replay, or
   adaptive practice.

Opening config pauses timed progression and live failure checks. Applying a
setting that needs a restart creates a fresh test behind the still-open config
workspace.

## Start the default test

```sh
mtype
```

The defaults are time mode, 30 seconds, base English, normal difficulty,
punctuation off, and numbers off. Type a normal character to start the clock.

Command-line flags are temporary:

```sh
mtype --time 60 --punctuation
mtype --mode words --words 50
mtype --mode quote
mtype --mode zen
mtype --language english_10k --time 30
mtype --custom "the quick brown fox jumps over the lazy dog"
```

Use the [config workspace](configuration.md) for saved changes and the
[CLI reference](cli-reference.md) for exact precedence.

## Modes

### Time

Time mode generates a streaming word pool and ends when elapsed time reaches
the configured duration. Choices exposed in config are 15, 30, 60, and 120
seconds. `--time` accepts an unsigned integer and implies time mode.

The engine keeps at least a few future words available as the cursor advances.
At timeout, the active partial word can receive correct-character credit when
it is still a correct prefix of the target. The last partial word has no
committing-space requirement.

```sh
mtype --time 15
mtype --time 60 --numbers --punctuation
```

### Words

Words mode creates a finite target and ends when the final word is completed.
Config offers 10, 25, 50, and 100 words. `--words` implies words mode.

The final word ends the test when it exactly matches. With Quick end enabled,
words mode may finish after the typed final word reaches the target length even
if it is imperfect; normal scoring still determines correctness.

```sh
mtype --mode words --words 25
mtype --words 100 --difficulty expert
```

### Quote

Quote mode selects an ordered quote from the collection associated with the
base language. Length choices are short, medium, long, thicc, or the union of
all four bands. The target preserves quote punctuation and order.

`--quote-id` requests one exact quote ID and implies quote mode. If the ID is not
present, mtype chooses a random quote from the configured bands. Sized language
names such as `english_1k` use the base `english` quote collection.

```sh
mtype --mode quote
mtype --quote-id 42
```

The results screen displays the quote source when the collection provides one.

### Zen

Zen mode has no generated target. Typed words become their own targets, so
input is treated as correct. Press Enter after typing has started to finish the
test. Time does not automatically end Zen mode even though the engine treats it
as a streaming timed-style mode for partial-character accounting.

```sh
mtype --mode zen
```

Zen is useful for free writing and cadence measurement. The input can still be
saved and replayed when it passes the normal result-validity rules.

### Custom

Custom mode splits configured text on whitespace and uses the resulting tokens
in order. Provide text for one process with `--custom`, or open System → Custom
text in config. The editor replaces newlines and tabs from paste with spaces.

```sh
mtype --custom "accuracy first, then speed"
```

In the editor, Ctrl+S trims, saves, switches to custom mode, and starts a fresh
test. Esc discards the current edit and returns to a test. Enter and Tab insert
spaces rather than literal newline or tab characters.

### Practice

Practice mode builds an ordered target from historical word analytics. It has
three sources:

- **missed** ranks submitted mistakes and corrected errors;
- **slow** ranks the lowest average clean burst speed;
- **mixed** combines mistake severity and slowness.

```sh
mtype practice missed --words 25
mtype practice slow --words 25
mtype practice mixed --words 50
```

Practice is filtered by the active language. If analytics returns no words,
mtype fills the requested count with random words from the active language.
Candidate words repeat as needed to reach the requested length, while avoiding
identical adjacent entries when more than one candidate exists.

## Generated-word behavior

Time and words modes draw from the selected word pool. The generator avoids the
same normalized word appearing among the previous two emissions when possible.
With punctuation off, it filters symbol-bearing entries, digit-bearing entries,
and capital `I`, and lowercases stray capitals. With numbers off, digit-bearing
dictionary items are avoided.

### Punctuation

Punctuation mode capitalizes sentence starts and probabilistically adds periods,
questions, exclamations, commas, quotes, parentheses, colons, hyphens, and
semicolons. The final word in a bounded words test receives sentence-ending
punctuation. Ordered quote and custom text are not randomly rewritten by this
pool-generation pipeline.

### Numbers

Numbers mode gives generated pool words a 10% chance of being replaced by a
one-to-four-digit number whose first digit is nonzero.

### British English

British English maps a fixed set of common American spellings to British forms
during generated pool tests, including color/colour, favorite/favourite,
center/centre, organize/organise, behavior/behaviour, and gray/grey.

### Funboxes

Generator funboxes can replace the source word or transform the final word.
Multiple transformations run in selected order. The first active generator-type
funbox supplies the base word. See [Custom content](custom-content.md) for all 20
supported funboxes and their exact effects.

## Test controls

| Key | Context and action |
| --- | --- |
| Printable character | Start the test if needed, record the attempt, and type into the active word |
| Space | Attempt to commit the active word |
| Backspace | Delete one character; at an empty current word, return to an eligible previous word |
| Ctrl+Backspace | Clear the active word; terminals that report Alt+Backspace use the same word-delete path |
| Ctrl+W | Clear the active word |
| Tab | Restart unless `no_quit` blocks leaving a running test |
| Esc | Open config, or restart when Quick restart is `esc`; `no_quit` can block it during a running test |
| Enter | Finish a running Zen test; ignored for normal test input |
| Ctrl+C | Quit globally |

Leading Space before the test starts does nothing. With Strict space off, Space
on an empty active word is also ignored. Stop on error and difficulty can change
whether an imperfect character or word is accepted; see
[Scoring and results](scoring-and-results.md).

## On-screen test elements

The top line can show:

- remaining seconds in time mode;
- completed/total words in finite modes;
- `zen` in Zen mode;
- optional live speed, accuracy, and last committed burst speed.

Timer/progress styles are off, text, bar, and mini. Text and mini share the same
numeric content in the current terminal renderer; bar uses a 12-cell progress
bar. Live speed and accuracy use shorter labels in mini mode. Live burst shows
the last committed word burst.

The target block wraps to the configured maximum width and normally keeps one
line of context above the active line. Show all lines begins from the first
line. Incorrect, extra, missed, caret, highlight, pace, and typo-indicator styles
are controlled from Appearance and Feedback.

## Results controls

| Key | Action |
| --- | --- |
| Tab or Enter | Start a fresh test with the active configuration |
| S | Open terminal stats |
| I | Toggle word-by-word input history |
| W | Start or restart animated replay |
| M | Start missed-word practice |
| L | Start slow-word practice |
| Esc | Return from input history/replay to summary; from summary, open config |
| Q | Quit |

The summary contains net speed, accuracy, raw speed, consistency, character
counts, duration, test descriptor, an optional quote source, and a per-second
net/raw graph. Failed tests show their reason. Eligible new records display a
personal-best banner.

## Input history

Input history lists saved word outcomes in test order. Each row contains target,
final typed form, word duration, burst WPM, and incorrect-keystroke count. A word
is marked as having an error even if the final submitted spelling is correct
after backspacing.

## Replay

Replay applies recorded input events in elapsed-time order: character, single
backspace, word backspace, and commit. The view shows the target, reconstructed
typed state, playback time, and complete/playing status. Press W to restart from
the beginning and Esc to return to the result summary.

## Stats controls

In terminal stats, Tab, Enter, or Esc returns to a fresh test. Q quits. A small
terminal shows a size warning instead of the full profile.

## Config and editor controls

See [Configuration](configuration.md) for the full tab and setting reference.
The custom editor uses Ctrl+S to save/start, Esc to cancel, Backspace to delete,
Enter or Tab to add a space, and printable characters to edit. Paste is accepted
only by the editor and normalizes carriage returns, newlines, and tabs to spaces.

## Practical workflows

### Establish a baseline

```sh
mtype --time 30
```

Complete at least ten tests with the same language, duration, punctuation,
numbers, and difficulty. This gives the dashboard a meaningful recent block and
keeps PB categories comparable.

### Improve accuracy

Set a modest minimum accuracy, use normal difficulty, and aim for consistent
cadence rather than bursts. Review corrected errors in Input history and the
dashboard wrong-word table.

### Train weak words

```sh
mtype practice missed --words 25
mtype practice slow --words 25
```

Alternate mistake-focused and speed-focused practice, then retest the same
baseline category.

### Compare modes honestly

Time and words settings form distinct test categories. Language, punctuation,
numbers, and difficulty also distinguish PB categories. Compare like with like
when evaluating progress.
