# EQ003003.PKF container format — reverse-engineering notes

This is the canonical, living document for what we've figured out about
the `.PKF` "teams container" format (PLAN.md Appendix B) by direct
byte-level analysis of the user's own legally-owned `EQ003003.PKF`
(PC Apertura 98/99, Argentina edition). No external tool or editor was
used to produce any of this — everything below was derived from the raw
bytes, cross-checked for internal consistency, and where possible,
independently verified.

**Do not commit real extracted game content here or in `fixtures/golden/`**
(see that directory's `.gitignore` rule) — this document should only ever
contain the *format knowledge* (structure, offsets, field layouts), not
copies of real biographical/roster text, beyond short illustrative
examples already covered by fair-use-style analysis (e.g. a handful of
real club names, which are factual and already public in
`fixtures/pointers/team_pointers.csv`).

## Status at a glance

| Question | Status |
|---|---|
| Is the container encrypted? | **No.** Confirmed — see §1. |
| Banner string | `Copyright (c)1996 Dinamic Multimedia` (no space after `(c)`) — confirmed, and `pcf_codec::dbc::BANNER` was fixed to match. |
| Directory format | **Decoded and verified** — see §2. |
| "Foreign reference clubs" stub table | **Decoded, ~473 records located and read** — see §3. |
| Real domestic (Argentina) team records | **Located, one likely-River record extracted (medium-high confidence)** — internal structure NOT yet fully walked. See §4. |
| Character map | 37 confirmed byte↔glyph pairs (`fixtures/charmap/confirmed_real_map.txt`) — a small subset of the full alphabet. Expansion in progress. |
| Full container parser in `pcf-codec` | **Not started.** Investigation only; no production parsing code has been written for this container format (the override-file `Dbc::read`/`write` format is separate and already implemented). |

## 1. No encryption

The first ~0xE8 bytes of `EQ003003.PKF` looked high-entropy on a first
glance, but turned out to be: a small per-file unique header (differs
across `EQ003003.PKF` / `TEXTOS.PKF` / `MINIESC.PKF`) followed by zero
padding — confirmed non-random because the zero-padding runs are
*identical* across files with different headers, which a keyed stream
cipher would not produce. Elsewhere in the file, apparent "noise" is
just raw binary numeric fields (player stats, offsets) whose byte values
happen to fall in the printable ASCII range.

## 2. Directory entry format (38 bytes, repeating)

```
offset  size  field
0       8     id (varies per entry — not yet decoded further)
8       13    signature, CONSTANT: 31 54 41 BB EF E8 E3 E0 0B C9 A3 E8 00
21      4     sub (varies — not yet decoded further)
25      4     offset : u32 LE — byte offset of this record's banner in the file
29      4     length : u32 LE — byte length of this record (offset[i]+length[i] == offset[i+1])
33      4     flag   : u32 LE — observed value 1 for all-but-last entry in a block, else varies
37      1     trailing byte — observed 0x02 for interior entries, 0x04 for the last entry of a block
```

Verified example (first entry, `EQ003003.PKF` offset 0xEE):
```
9a 91 9a bf 5f 68 73 01 | 31 54 41 bb ef e8 e3 e0 0b c9 a3 e8 00 | 83 25 0d bf | b2 05 00 00 | 18 07 00 00 | 01 00 00 00 | 02
```
- `offset` = `B2 05 00 00` = 1458 = the real first banner offset in the file (exact match).
- `length` = `18 07 00 00` = 1816; `1458 + 1816 = 3274` = the real second banner offset (exact match).

This pattern (`offset[i] + length[i] == offset[i+1]`) was verified across
multiple consecutive entries.

## 3. Directory blocks & the "foreign reference clubs" stub table

The 13-byte directory signature recurs in **15 contiguous blocks**: 14
full blocks of 32 entries + one partial block of 25 entries, spanning
file byte range `[238, 1660089)`. Each entry in these blocks points to a
short (~1,500–2,000 byte) **stub record**: banner, then a fixed 4-byte
marker `0D 02 00 01`, then a 2-byte little-endian string-length prefix,
then the string itself (team `short_name`), followed by more such
length-prefixed strings (`stadium_name`, `long_name`) and presumably
some numeric team-info fields — but **no player/coach data**.

Decoding block 0 end-to-end (37-confirmed-pair charmap) gives, **in
physical file order**: F.C. Barcelona, R.C. Deportivo (Riazor), Real
Zaragoza (La Romareda), Real Madrid C.F. (Santiago Bernabéu), Athletic,
Valencia, R. Racing (El Sardinero), R. Oviedo, C.D. Tenerife, Real
Sociedad, Club At. Madrid, R.C. Celta, R. Valladolid, R.C.D. Español, R.
Betis, R.C.D. Mallorca, Villarreal C.F., Salamanca, Extremadura, Alavés
— then it switches to Serie A: Milan (San Siro), Juventus, Sampdoria,
Lazio, Parma, Roma, Cagliari, Inter, Fiorentina, Bari, Piacenza, Udinese.

This lines up with `fixtures/pointers/team_pointers.csv`'s pointer
`0001`-`0276`-ish range (Spain then Italy) — **this stub table is a
"foreign/reference clubs" list, holding just enough data for cross-team
references (e.g. affiliate club pointers, international opponent
listings), not full playable rosters.**

The 4-byte marker `0D 02 00 01` occurs 418 times total, and **all but
one** occurrence falls inside these 15 directory-covered blocks — the
lone exception (file offset 628,314) marks the tail end of the last stub
region. Past that point, the marker never recurs.

## 4. Where the real Argentina domestic team data starts

Past file offset ~628,300, the stub marker `0D 02 00 01` stops appearing
entirely, and the spacing between banner occurrences jumps from
~1,500–2,000 bytes to **tens of thousands of bytes** — consistent with
full team records (team info + tactics + coach + a full player roster)
rather than short stubs.

One such record was located at banner offset **629,003**, immediately
followed by header bytes `E9 07 0D 02 00 00 05 00` (note: `0D 02 00 00`
here, vs. `0D 02 00 01` for foreign stubs — plausibly a
domestic-vs-foreign flag in the low byte), then a 5-byte length-prefixed
string decoding (37-pair charmap, one byte inferred) to **"River"**.
`fixtures/pointers/team_pointers.csv` has `9001,River,Argentina`. The
next banner after this one is at offset 721,959 — a 92,956-byte span,
extracted locally (never committed — see `fixtures/golden/.gitignore`
rule and `real_river_9001_container_blob.README.md` for the full
reasoning and confidence caveats).

**Not yet done:** walking this record's internal sub-structure
(tactics/coach/player boundaries), or locating and comparing additional
domestic team records (there should be roughly 15-20 more: Boca, River,
San Lorenzo, Independiente, ... plus special entries like `9900 Estrellas
España`, `9950 Jugadores Libres`, `9955-9958 Juveniles ...` per the
pointer catalog).

## 5. Open questions / next steps

1. **Expand the character map.** We have 473 real, known-plaintext club
   names available in the stub table (§3) — a much bigger known-plaintext
   corpus than the handful of manual examples that produced the current
   37-pair map. This should let us confirm digits, accented letters
   (á é í ó ú ñ Á É...), and more punctuation.
2. **Walk one full domestic team record byte-by-byte** (start with the
   likely-River blob) to determine: where team info ends and tactics
   begins, the coach chain's boundaries, and how individual player
   records are delimited (fixed marker? length-prefixed block? another
   banner?).
3. **Enumerate all domestic team records** past offset ~628,300 by
   banner position, and see if their count/order matches the Argentina
   block of `fixtures/pointers/team_pointers.csv` (9001-9061 plus
   specials).
4. **Reconcile with `pcf_codec::dbc::Dbc::read`/`write`.** The
   override-file format those functions implement (single
   `BANNER`+`MAGIC_FE06`+team+tactics+coach+players, per PLAN.md Appendix
   A) is NOT the same as this container's internal per-record framing
   (banner + a different header, e.g. `E9 07 0D 02 00 00 ...`). Whether
   the *exported* `EQ97####.DBC` override files (which we still don't
   have a real sample of) match Appendix A's documented format, or
   something closer to the container's internal framing, is still an
   open question — this is a deliberate design decision to make later,
   not something to guess at now.

## Investigation tools (all under `crates/pcf-codec/examples/`)

| Tool | Purpose |
|---|---|
| `investigate_pkf.rs` | First-pass investigator: finds banner occurrences (both spellings), delta stats between them, hex-dumps the pre-first-banner region and the 32 bytes after the first banner. Usage: `cargo run -p pcf-codec --example investigate_pkf -- <path.pkf>` |
| `investigate_pkf_dir.rs` | Second-pass investigator: finds the 13-byte directory signature, groups occurrences into contiguous blocks, verifies offset/length fields against real banner positions, and can dump+decode any chosen block's record bodies. Usage: `cargo run -p pcf-codec --example investigate_pkf_dir -- <path.pkf> [block_index]` |
| `build_synthetic_golden.rs` | Unrelated to PKF investigation — regenerates `fixtures/golden/synthetic_minimal.dbc` from the in-code synthetic `Dbc` builder (Agent A's TDD fixture, not real data). |

All of these are run inside the Docker dev container (Rust isn't
installed on the host): `docker compose -f docker-compose.dev.yml exec
dev cargo run -p pcf-codec --example <name> -- <args>` from the repo
root. Pointing them at a real file under `/c/PCF6AR` or `/z` requires an
extra bind mount since `docker-compose.dev.yml` only mounts the repo by
default, e.g.:
```
docker run --rm -v "$(pwd)":/workspace -v /c/PCF6AR/DBDAT:/gamedata:ro \
  -w /workspace pcfutbol-editor-dev:latest \
  cargo run -p pcf-codec --example investigate_pkf_dir -- /gamedata/EQ003003.PKF 0
```
