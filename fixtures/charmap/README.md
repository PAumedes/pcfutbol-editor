# Charmap fixtures — SYNTHETIC PLACEHOLDER, not real game data

`synthetic_map.txt` in this directory is **invented** by Agent A for TDD purposes.
It is NOT the real PC Apertura 98/99 character map and must not be shipped or
treated as authoritative.

## What's real vs. invented

Only these 12 byte↔char mappings are verified, straight from PLAN.md Appendix A's
"Real Madrid C.F." encoding proof (`3304000D412C000513080541224F274F`):

| byte | char | byte | char | byte | char |
|------|------|------|------|------|------|
| 0x33 | R    | 0x2C | M    | 0x4F | .    |
| 0x04 | e    | 0x05 | d    | 0x22 | C    |
| 0x00 | a    | 0x13 | r    |      |      |
| 0x0D | l    | 0x08 | i    |      |      |
| 0x41 | (sp) | 0x27 | F    |      |      |

Every other mapping in `synthetic_map.txt` (the rest of the Latin alphabet,
digits, and a handful of punctuation marks needed for the editor's default
strings like `"x"` and `"ND,ND,ND,ND,ND=="`) is **arbitrarily invented** by
Agent A, chosen only to be internally consistent (no byte reused for two
chars) so that `CharMap` round-trip tests have something to exercise. Treat
any glyph outside the 11 verified ones above as almost certainly wrong for
the real game.

## File format

Plain text, one mapping per line: `HH\tC` — two uppercase hex digits, a tab,
then exactly one character. Because trailing whitespace tends to get
stripped by editors/tools, the literal space character is written as the
two-character escape `\s` instead of an actual space (e.g. `41\t\s`); no
other escapes are defined. Lines starting with `#` and blank lines are
ignored. This is a
guess at a `map.txt`-style format; the community's real file may differ
slightly. `CharMap::load` / `CharMap::parse` is the single place to adjust
the parser if the real file's format doesn't match — no other code depends
on the file's shape, so swapping in the real map only requires either
(a) reformatting it to match this parser, or (b) adjusting `CharMap::parse`
once we've seen it.

## What's still needed from the user

A real `map.txt` (or equivalent) for PC Apertura 98/99, sourced from the
community (see PLAN.md Appendix C: carky12/EditorPCFutbol6, pcfutbolmania.com)
or extracted from the game's own resources. Until that lands in this
directory (e.g. as `map.txt`), nothing decoded through `CharMap` should be
trusted as matching the real game's text.
