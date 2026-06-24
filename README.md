# mtype

A typing speed test that runs in your terminal. It is a port of [Monkeytype](https://github.com/monkeytypegame/monkeytype),
so if you have used monkeytype.com you already know how it works. Words show up,
you type them, and it measures your typing speed and accuracy in real time. No
browser, no account, no network. It works fully offline.

mtype is a command line typing test, a terminal WPM test, and a Monkeytype style
typing trainer for macOS and Linux.

```
  30

  the quick brown fox jumps over the lazy dog and then keeps
  going while the timer counts down and your wpm and accuracy
  update as you type each word in the test

  tab restart    esc menu    ctrl+c quit
```

## Why this exists

I wanted Monkeytype without leaving the terminal, so I could run a quick typing
test between other work without opening a browser or signing in. mtype keeps the
parts that matter (the test, the modes, the live stats, the results graph) and
drops the account system. Your results and personal bests are saved on your own
machine instead.

## Features

- Test modes: time, words, quote, zen, and custom text
- Punctuation, numbers, and three difficulty levels (normal, expert, master)
- Live words per minute, accuracy, and a timer while you type
- A results screen with a WPM over time graph, raw WPM, consistency, and a full
  character breakdown
- Personal best tracking saved locally, which is the offline replacement for the
  online account
- Every English word list from Monkeytype bundled and ready offline: english,
  english_1k, english_5k, english_10k, english_25k, english_450k, and themed sets
  like english_medical, english_legal, and english_shakespearean
- A command palette (press Esc) with fuzzy search to change any setting
- Funbox modifiers such as rot13, ALL_CAPS, sponge case, morse, binary, hex, and
  gibberish
- True color themes

## Install

You do not need Rust to run mtype if you grab a prebuilt binary.

### Option 1: download a prebuilt binary

Open the releases page and download the file for your system:

https://github.com/raminsharifi/mtype/releases/latest

macOS on Apple Silicon (M1, M2, M3, and newer):

```sh
curl -L -o mtype https://github.com/raminsharifi/mtype/releases/latest/download/mtype-macos-arm64
chmod +x mtype
xattr -d com.apple.quarantine mtype
./mtype
```

The xattr line clears the flag macOS puts on downloaded files. The binary is not
code signed, so without it macOS may refuse to open the file the first time. You
can also right click the file in Finder and choose Open.

To run it from anywhere, move it onto your PATH:

```sh
sudo mv mtype /usr/local/bin/
mtype
```

If your platform is not on the releases page yet, use Option 2.

### Option 2: build it from source

This works on Linux, Intel Macs, and Apple Silicon. The command below installs
Rust first if you do not already have it, then builds and installs mtype. It
takes about a minute.

```sh
# skip this line if you already have Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

cargo install --git https://github.com/raminsharifi/mtype
```

The binary is installed to `~/.cargo/bin/mtype`, which rustup adds to your PATH.
Open a new terminal and run `mtype`.

## Usage

Run it with no arguments to start a 30 second test:

```sh
mtype
```

Type to begin. A few examples:

```sh
mtype --time 60 --punctuation
mtype --mode words --words 50
mtype --mode quote
mtype --mode zen
mtype --language english_10k --time 30
mtype --custom "the quick brown fox jumps over the lazy dog"
```

### Keys

- Type to take the test
- Space moves to the next word
- Backspace deletes a letter, Ctrl and Backspace deletes a word
- Tab restarts the test
- Esc opens the command palette, where you can change any setting
- Ctrl and C quits

On the results screen, Tab or Enter starts a new test and Q quits.

If you want the text to look bigger, use your terminal's own font zoom (on macOS
that is Cmd plus and Cmd minus, on most Linux terminals Ctrl plus and Ctrl minus).

## More languages and quotes

mtype ships with every English word list offline. If you want another language
or more quotes, download them from the Monkeytype repository into your local data
folder:

```sh
mtype sync language spanish
mtype sync quotes french
mtype --language spanish --mode words
```

This is the only feature that uses the network, and it is optional. Everything
else works with no connection.

## Where your data is stored

On macOS this lives under `~/Library/Application Support/com.monkeytype.mtype/`,
and on Linux under `~/.config/mtype/` and `~/.local/share/mtype/`:

- `config.toml` holds your settings
- `results.json` holds your test history and personal bests
- `languages/` and `quotes/` hold anything you downloaded with `mtype sync`

## Develop

```sh
git clone https://github.com/raminsharifi/mtype
cd mtype
cargo run --release
cargo test
```

The scoring and word generation logic is pure and unit tested, so most of the
behavior is covered without needing a terminal.

## Credits and license

mtype is a port of [Monkeytype](https://github.com/monkeytypegame/monkeytype).
The English word lists, the quotes, and the core algorithms for word generation
and scoring come from that project. Thank you to the Monkeytype maintainers and
contributors for building it and for releasing it as open source.

Monkeytype is licensed under the GNU General Public License version 3. Because
mtype is derived from it, mtype is licensed under the GPL-3.0 as well. See the
[LICENSE](LICENSE) file for the full text and [NOTICE.md](NOTICE.md) for the
details of what was ported.

mtype is not affiliated with, endorsed by, or sponsored by Monkeytype.
