# Attribution and license notice

mtype is a terminal typing test for the command line. It is an independent Rust
program, but its behavior is a port of [Monkeytype](https://github.com/monkeytypegame/monkeytype),
and several of its algorithms were translated directly from the Monkeytype source.

Monkeytype is copyright its contributors and is licensed under the GNU General
Public License version 3 (GPL-3.0). Because mtype is derived from that code, mtype
is also licensed under the GPL-3.0. The full license text is in the `LICENSE` file.

## What was ported from Monkeytype

The following logic was translated from the Monkeytype TypeScript source into
Rust. The wording and structure are reimplemented, but the algorithms and
numeric behavior follow the originals:

- Word generation and the probabilistic punctuation pass
  (`frontend/src/ts/test/words-generator.ts`)
- Word selection and the no-repeat rule (`frontend/src/ts/test/wordset.ts`)
- Speed, accuracy, and consistency math: words per minute, raw words per minute,
  and the `kogasa` consistency function
  (`frontend/src/ts/utils/numbers.ts`, `packages/util/src/numbers.ts`)
- Character classification and the trailing-space-per-word rule
  (`frontend/src/ts/utils/strings.ts`, `frontend/src/ts/test/events/stats.ts`)
- Funbox text transforms and generators
  (`frontend/src/ts/test/funbox/funbox-functions.ts`, `frontend/src/ts/utils/generate.ts`)

The bundled English word lists and quote collection are taken from
`frontend/static/languages/` and `frontend/static/quotes/` in the Monkeytype
repository.

## What is new

The terminal interface, the input and rendering engine, local results storage,
the command palette, the configuration system, and the offline content packaging
are original work written for this project.

## Affiliation

This project is not affiliated with, endorsed by, or sponsored by Monkeytype or
its maintainers. "Monkeytype" is the name of that project and is used here only
to describe what mtype is based on.

## Copyright

Copyright (C) 2026 Ramin Sharifi and mtype contributors.
Portions copyright the Monkeytype contributors.

This program is free software: you can redistribute it and/or modify it under the
terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version. This program is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more
details.
