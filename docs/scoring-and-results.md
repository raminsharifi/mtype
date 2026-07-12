# Scoring, metrics, failure rules, and results

[Documentation index](README.md) · [Usage](usage.md) · [Stats and practice](stats-and-practice.md)

## Timing model

The test clock starts on the first printable non-control character. A leading
Space does not start the test. All engine input functions receive explicit
elapsed milliseconds, making scoring deterministic in tests.

Duration is the difference between start and finish timestamps in seconds,
rounded to two decimal places in the final result. Time mode finishes on the
periodic tick at or after the configured duration. Finite modes finish when the
last target is completed. Zen finishes when Enter calls the explicit finish
path.

## Five-character word convention

All base speed formulas use the standard five-character word:

```text
WPM = character count / 5 / (duration seconds / 60)
```

For example, 250 scored characters in 60 seconds is 50 WPM. The engine rounds
final WPM and raw WPM to two decimal places.

Every non-final committed target includes a trailing space for character
classification. This means correctly committing a word contributes its
inter-word space. The active final/partial word is evaluated without a trailing
space.

## Character classification

The engine classifies target/input positions into these counters:

| Counter | Meaning |
| --- | --- |
| `all_correct` | Position-level correct characters, including correct characters in an otherwise invalid word |
| `correct_word` | Characters credited to net WPM because the entire committed word is correct, or because a timed active word is a correct prefix |
| `incorrect` | A typed character disagrees with an existing target position |
| `extra` | Input extends beyond the target, or occupies the target-space position incorrectly |
| `missed` | A target position has no input when final classification requires it |

The result screen's `characters` line is:

```text
correct_word / incorrect / extra / missed
```

`char_total` is `all_correct + incorrect + extra`; it is used by the validity
gate to determine whether anything was typed.

## Net WPM

Net WPM uses only `correct_word` characters:

```text
net WPM = correct_word / 5 / minutes
```

An incorrect committed word contributes zero net characters even if many of
its individual letters matched. During a timed test, the active last word can
receive partial credit when its typed content is a correct prefix of the target.

## Raw WPM

Final raw WPM uses all position-level correct characters plus incorrect and
extra characters:

```text
raw WPM = (all_correct + incorrect + extra) / 5 / minutes
```

Raw WPM represents physical character production before invalid-word penalties.
The difference `raw WPM - net WPM` is called speed leakage in the browser
dashboard. A large gap usually means bursts are not converting into clean
completed words.

The per-second raw chart counts letter keystrokes recorded by each second
boundary. The final raw metric is based on classified characters. These two
paths are designed to represent the same physical activity but are calculated
from different event summaries.

## Accuracy

Accuracy is keystroke-level:

```text
accuracy = correct recorded keystrokes / all recorded keystrokes × 100
```

Recorded keystrokes include typed characters and committing spaces. An
incorrect character remains an incorrect keystroke even if it is later erased.
An incorrect Space rejected by stop-on-error word or expert handling also
counts as incorrect. Backspace events themselves are stored for replay but are
not part of the accuracy keystroke denominator.

This is why a finally correct word can still lower accuracy and appear as
`had_error` in analytics.

## Burst WPM

Word burst measures the physical length of one word plus its trigger over the
time since the word started:

```text
burst WPM = (typed characters + committing space) / 5 / word minutes
```

For an uncommitted final word, word outcome burst omits the committing-space
character. Burst is rounded to two decimals in each stored word outcome. The
live burst indicator shows the last committed burst.

## Consistency

Consistency uses the array of per-word burst speeds:

1. Compute the population mean.
2. Compute the population standard deviation.
3. Divide standard deviation by mean to obtain coefficient of variation.
4. Apply Monkeytype's `kogasa` mapping:

```text
100 × (1 - tanh(cov + cov³/3 + cov⁵/5))
```

A constant burst series produces 100. Higher relative variation lowers the
score. No usable burst mean produces 0. Final consistency is rounded to two
decimal places.

## Per-second result graph

For each elapsed whole-second boundary:

- net history sums `correct_word` characters from words committed by that
  boundary and divides by elapsed time;
- raw history counts recorded letter keystrokes up to the boundary and divides
  by elapsed time.

The result graph needs at least two history points and enough terminal height.
Raw uses the theme sub color; net uses the theme main color. Start graphs at
zero controls the lower bound. When disabled, the lower bound is 90% of the
minimum observed series value, clamped at zero.

## Difficulty

| Difficulty | Failure condition |
| --- | --- |
| Normal | Incorrect keys and words are allowed and scored normally |
| Expert | The test fails when an imperfect word is submitted |
| Master | The test fails immediately on the first incorrect character attempt |

Master failure also occurs when stop-on-error letter rejects the wrong
character, because the incorrect attempt is still recorded. Failed tests reach
the result screen with a reason but are not valid for saving.

## Stop on error

Stop on error is separate from difficulty:

- **off** inserts incorrect characters and allows imperfect word commits;
- **letter** records an incorrect character attempt but does not insert it;
- **word** allows incorrect characters in the active word but rejects Space
  until the word is correct.

Expert and master rules can still fail the test where applicable.

## Minimum failure conditions

Minimum checks begin after one elapsed second while the test is running:

- minimum WPM compares live net WPM;
- minimum accuracy compares live keystroke accuracy;
- minimum burst compares the newest committed word burst when one exists.

Falling below an enabled threshold fails immediately with `min wpm not met`,
`min accuracy not met`, or `min burst not met`. The checks do not wait for the
end of the test.

## Lazy mode

Lazy comparison first checks exact equality. When enabled, it then decomposes
Unicode to NFD and removes combining marks before comparing. This makes base
letters and accent-composed variants compare equal in many cases. It is not a
general spelling correction and does not change stored target/typed strings.

## Blind and visual settings

Blind mode affects rendering, not scoring. Incorrect input is displayed with
the normal correct-looking color, but keystrokes, word validity, accuracy, and
analytics retain actual correctness. Hide extra letters blocks insertion past
the target length after recording the attempted input event and increasing the
word's corrected-error counter. The current engine returns before adding that
blocked key to the keystroke series, so it does not lower the accuracy metric.

## Result validity and saving

A result is eligible for storage only when:

1. Result saving is enabled.
2. The test did not fail.
3. Duration is at least 1.0 second.
4. `char_total` is greater than zero.

Valid results append to `results.json`. SQLite recording is best-effort after
the readable JSON history is saved; a database problem does not interrupt the
typing flow or discard the JSON result.

## Personal bests

Only time and words modes can produce a PB banner. A result must be strictly
greater than the previous best; equal WPM is not a new PB.

PB categories require all of these fields to match:

- mode;
- mode2, such as time duration or word count;
- punctuation;
- numbers;
- language;
- difficulty.

Quote, Zen, custom, and practice results can be saved and included in lifetime
stats but are not PB-eligible.

## Result summary fields

| Field | Source |
| --- | --- |
| Headline speed | Net WPM converted to the selected display unit |
| Accuracy | Keystroke accuracy percentage |
| Raw | Raw WPM converted to the selected display unit |
| Consistency | Kogasa score over word bursts |
| Characters | correct-word / incorrect / extra / missed |
| Time | Rounded elapsed seconds |
| Test | Mode plus duration/count/practice kind and punctuation/number modifiers |
| Source | Quote source when available |

Always show decimals forces two decimal places. Otherwise a mathematically
integral metric is displayed without decimals and a fractional metric uses two.

## Word outcomes

Each reached word can store:

- zero-based word index;
- target and final typed form;
- preceding target word;
- word duration in milliseconds;
- burst WPM;
- final correct flag;
- `had_error`, true for a final miss or any corrected error;
- incorrect-keystroke count;
- correct, incorrect, extra, and missed character counts.

Committed words are stored as they are submitted. A nonempty active final word
is appended at result construction if it has not already been committed.

## Input events and replay

Input events contain:

- milliseconds since the first character;
- active word index;
- kind: character, commit, backspace, or word backspace;
- character value for character events;
- correctness for character/commit attempts where applicable.

Replay reconstructs typed strings by applying these events in order. Commit
events advance the historical interaction but do not directly mutate the replay
string; word indexes on subsequent events place input in the next word.
