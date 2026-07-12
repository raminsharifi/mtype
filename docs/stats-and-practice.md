# Stats, browser dashboard, insights, and adaptive practice

[Documentation index](README.md) · [Scoring](scoring-and-results.md) · [Data and privacy](data-and-privacy.md)

Every valid saved test contributes to local lifetime statistics. Word outcomes
and input events from those tests contribute to normalized SQLite analytics.
The terminal stats page reads result history; the browser dashboard combines
history with SQLite word detail.

## Open stats

### From results

Press S on the result summary.

### From config

Open System → Stats / progress, or type `view stats` from Current search.

### From the command line

```sh
mtype stats
```

Terminal stats controls:

| Key | Action |
| --- | --- |
| Tab, Enter, or Esc | Return to a fresh test |
| Q | Quit |

## Saved-result requirements

Stats only include results that pass the storage validity gate:

- result saving enabled;
- test not failed;
- duration at least one second;
- at least one scored input character.

Started tests are counted when the first printable character begins a test.
The profile's started count is never allowed to be lower than completed history
length.

## Terminal lifetime metrics

| Metric | Definition |
| --- | --- |
| Completed | Number of stored results |
| Started | Stored started-test counter, at least completed count |
| Time typing | Sum of stored result durations |
| Estimated words | Per result, `round(WPM × duration seconds / 60)`, then summed |
| Highest speed | Maximum stored net WPM, converted to selected display unit |
| Average speed | Arithmetic mean of stored net WPM |
| Last-10 speed | Mean of the newest up to 10 results |
| Highest raw | Maximum stored raw WPM |
| Average raw | Mean stored raw WPM |
| Highest accuracy | Maximum stored keystroke accuracy |
| Average accuracy | Mean stored accuracy |
| Last-10 accuracy | Mean of the newest up to 10 accuracy values |
| Highest consistency | Maximum positive consistency value |
| Average consistency | Mean of positive consistency values; zero-value undefined samples are excluded |

The terminal WPM-over-time graph uses every stored result in chronological
order. It appears after at least two results. The recent-results section uses
the newest 20 results in reverse chronological order.

## Activity and streaks

Activity groups results by whole UTC day: `timestamp_ms / 86,400,000`. Multiple
tests on one day increase that day's heatmap count.

Longest streak is the maximum historical run of consecutive active UTC days.
Current streak is nonzero only when the most recent active day is today or
yesterday in UTC. Starting from that day, it counts backward through consecutive
active days.

This UTC definition can differ from the local calendar near midnight.

## Start the browser dashboard

```sh
mtype stats serve
```

Default behavior:

1. Bind only to `127.0.0.1:4242`.
2. Print the local URL.
3. Launch the default browser with `open` on macOS or `xdg-open` on Linux.
4. Serve until Ctrl+C stops the process.

Options:

```sh
mtype stats serve --port 5050
mtype stats serve --no-open
mtype stats serve --port 0 --no-open
```

Port 0 chooses a free loopback port. If browser launch fails, the server remains
available and prints the URL for manual use.

## Dashboard privacy and delivery

- The listener uses IPv4 loopback, not a public interface.
- Every frontend asset is embedded in the executable with `include_str!`.
- No CDN, remote font, remote script, telemetry, login, or external API is used.
- Dashboard API data is generated locally for each refresh.
- Responses use `Cache-Control: no-store` and `X-Content-Type-Options: nosniff`.
- Content Security Policy restricts content/scripts/connections/images to the
  local origin, allowing inline style only for dynamic chart positioning.
- Only GET is accepted; other methods receive 405.

Local routes:

| Route | Response |
| --- | --- |
| `/` and `/index.html` | Dashboard HTML |
| `/dashboard.css` | Embedded stylesheet |
| `/dashboard.js` | Embedded application script |
| `/api/health` | `{"ok":true}` |
| `/api/dashboard` | Fresh local dashboard JSON |

## Dashboard overview metrics

| Metric | Definition |
| --- | --- |
| Tests completed/started | Same profile counters as terminal stats |
| Focused typing | Sum of valid saved result durations |
| Estimated words | Same profile estimate as terminal stats |
| Personal best | Highest stored net WPM across all history, not category-specific PB display |
| Average WPM | Mean across all stored results |
| Recent WPM | Mean of the newest up to 10 results |
| Previous WPM | Mean of the block immediately before the newest 10, up to 10 results |
| Trend percent | `(recent - previous) / previous × 100`; absent when previous is zero |
| Average accuracy | Mean across all stored results |
| Recent accuracy | Mean of newest up to 10 results |
| Average consistency | Mean of positive stored consistency values |
| Speed leak | Mean of `max(raw WPM - net WPM, 0)` across history |
| Current/longest streak | UTC streak definitions above |
| Mistake events | Sum of error-attempt counts among the dashboard wrong-word rows |
| Weak-word count | Number of wrong-word aggregates returned, capped by the query limit |

The hero uses recent WPM and changes its narrative based on trend and recent
accuracy. Numeric overview values are rounded to two decimal places by the API
and formatted for display by the browser.

## Progress chart

Each point includes timestamp, net WPM, raw WPM, accuracy, consistency, mode,
mode2, and language. Controls provide newest 30, newest 90, or all points and a
base-mode filter. The mode filter distinguishes `time` from `words` but does not
split durations/counts; the mode-performance section does split time and words
by mode2, such as `time 30` and `words 25`.

The chart draws:

- net WPM in the accent color;
- raw WPM in the secondary color;
- accuracy as a dashed line on an 80–100 scale;
- hover details containing WPM, accuracy, mode/mode2, and date.

Two points are required for a line. Empty, loading, and API-error states have
dedicated layouts. Animation respects `prefers-reduced-motion`.

## Mode performance

For each mode key, the dashboard calculates:

- test count;
- average net WPM;
- best net WPM;
- average accuracy.

Time and words keys include mode2. Other modes use the base mode name. Rows are
ordered by test count descending.

## Wrong words

The wrong-word query aggregates up to 100 targets with at least one `had_error`:

| Field | Definition |
| --- | --- |
| Attempts | Number of word events for the target |
| Error attempts | Sum of `had_error`; includes corrected and submitted errors |
| Missed attempts | Count where final `correct` is false |
| Error rate | `error attempts / attempts × 100` |
| Average burst | Average positive burst across attempts |
| Last seen | Newest containing session timestamp |
| Variants | Up to three final typed forms among error attempts, ranked by frequency |

Ranking weights a final miss as 3 and a corrected-only error as 1, divides that
weighted sum by attempts, then sorts by total error attempts and recency. This
places frequently or severely broken words above one-off light corrections.

The UI supports text search, displays the top 40 matching rows, and updates a
focus panel with error rate, attempt count, and common typed variants.

Dashboard wrong-word aggregation currently spans every language in SQLite;
target strings are grouped directly. Adaptive practice, by contrast, filters
the active language.

The browser heatmap renders 371 UTC day cells from today minus 370 days through
today. Each nonzero cell receives level 1–4 by scaling its test count against the
largest daily count in that window.

## Slow but accurate words

The dashboard selects up to 40 target words where final `correct = 1` and
`burst_wpm > 0`, grouped by target and ordered by lowest average burst, then
higher attempt count. The interface displays the first 18. This view finds
rhythm constraints that do not necessarily produce errors.

Like the dashboard wrong-word table, this aggregate currently spans all
languages.

## Character confusions

For every word event with `had_error = 1`, mtype compares final target and typed
characters by position up to the longer length. Different positions increment
an `(expected, typed)` pair:

- a missing typed character uses `missed`;
- an extra typed character uses `extra` as the expected side.

Pairs are sorted by count descending and capped at 24; the UI displays 16. This
is a positional comparison rather than an edit-distance alignment, so one early
insertion can create several downstream positional pairs.

## Insight rules

The API generates at most four insights in this order.

### No history

The dashboard explains that one saved test establishes the baseline.

### Recent trend

- At least +2%: reports the recent gain and recommends holding the same test mix
  for another 10 tests.
- At most -2%: reports the decline and recommends short accuracy-first sessions
  before chasing speed.
- Between -2% and +2%: no direction insight is added.

Trend compares newest up to 10 results with the preceding up to 10. A previous
block must have positive average WPM.

### Accuracy or conversion

- With at least three tests and recent accuracy below 96%, accuracy is identified
  as the limiting factor and the recommendation targets 97% for five tests.
- Otherwise, speed leak of at least 6 WPM produces a raw-to-net efficiency
  insight.

Only one of these two is emitted because the efficiency rule is the alternative
branch.

### Word constraint

The top wrong word is preferred. The insight includes its error rate, attempts,
and most frequent variant, then recommends missed-word practice. If there is no
wrong word, the slowest clean word is used and slow-word practice is recommended.

### Plateau

With at least 10 results, the API performs linear regression on the newest up
to 20 WPM values. An absolute slope below 0.12 WPM per test is classified as
stable/plateaued and triggers a mixed specific-practice recommendation.

### Balanced fallback

If none of the prior rules emit anything, the dashboard reports a balanced
foundation and recommends consistent settings for five more sessions.

## Adaptive practice database view

`word_practice_stats` groups by language and target word and provides attempts,
error attempts, final misses, average positive burst, and last-seen timestamp.

### Missed ranking

Candidates require at least one error attempt and sort by:

```text
missed_attempts × 3 + error_attempts
```

descending, then newest last-seen time.

### Slow ranking

Candidates require a non-null average burst and sort by lowest average burst,
then attempt count descending, then recency.

### Mixed ranking

Candidates need an error or burst sample and sort by this descending score:

```text
((missed_attempts × 4 + error_attempts) × 100 / attempts)
+ 600 / (coalesce(avg_burst_wpm, 0) + 10)
```

The first term emphasizes mistake severity/rate; the second increases priority
for slower words.

## Practice length and repetition

The SQL candidate limit is capped at 500 unique targets. mtype then cycles the
ordered candidate list until the requested practice count is reached. With more
than one candidate, it avoids placing the same word next to itself. CLI practice
count is clamped to at least one; the config workspace exposes 10, 25, 50, and
100.

## Training interpretation

- Use error rate and variants to identify spelling/key-sequence problems.
- Use slow clean words to find transitions that reduce cadence without obvious
  mistakes.
- Use speed leak to separate raw motor speed from usable scored speed.
- Keep the test category stable when evaluating a training block.
- Treat small samples as hypotheses: one attempt at 100% error is less reliable
  than a repeated pattern across many attempts.
