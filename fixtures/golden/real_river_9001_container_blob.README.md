# `real_river_9001_container_blob.raw`

**Confidence: medium-high.** This is a raw byte extract from the real,
user-owned `EQ003003.PKF` container, not a `.dbc` override file, and it does
**not** currently parse via `pcf_codec::DbcCodec::read` (see "Why not
`.dbc`" below). It is saved for a future container-specific parser to
consume, per the investigation task that produced it.

## What this is

Bytes `[629003, 721959)` (92,956 bytes) of
`C:\PCF6AR\DBDAT\EQ003003.PKF` (the read-only game install; **not
modified**, only read), copied verbatim with `dd`. This is the *complete*
span between one banner occurrence and the next, and I have moderate-to-high
confidence it is the *entire* record for **River** (Argentina), pointer
`9001` per `fixtures/pointers/team_pointers.csv` (`9001,River,Argentina`).

## How I found it (reproducible)

1. Starting from an earlier pass (see `crates/pcf-codec/examples/investigate_pkf.rs`),
   the literal banner `Copyright (c)1996 Dinamic Multimedia` (no space —
   note this differs from an older, since-fixed version of
   `pcf_codec::dbc::BANNER`) was found 473 times in `EQ003003.PKF`.
2. `crates/pcf-codec/examples/investigate_pkf_dir.rs` found a 13-byte
   constant signature (`31 54 41 BB EF E8 E3 E0 0B C9 A3 E8 00`) recurring
   as part of a 38-byte directory-entry format:
   `id[8] + signature[13] + sub[4] + offset:u32LE + length:u32LE + flag:u32LE + trailing:u8`.
   The `offset`/`length` fields were verified byte-for-byte against the
   real banner positions (e.g. block 0 entry 0: `offset=1458`,
   `length=1816`; the next real banner is at `1458+1816=3274`, an exact
   match, repeated across dozens of entries).
3. These 38-byte directory entries turned out to be organized into 15
   contiguous "blocks": 14 full blocks of exactly 32 entries, plus one
   partial block of 25 entries starting at file offset 1,659,139. Dumping
   block 0's 32 records (see the investigation report) showed each one is a
   **short, self-contained "foreign reference club" record** — banner,
   small header, then `short_name`/`stadium_name`/`long_name` strings that
   decode cleanly via `fixtures/charmap/confirmed_real_map.txt` to real
   Spanish and Italian clubs in physical order (F.C. Barcelona, Real
   Madrid, Athletic, Valencia, Racing Santander, ... Milan, Juventus,
   Sampdoria, Lazio, ... Udinese) — clearly a "world clubs" reference
   table, not Argentine teams with rosters. Each such stub record is only
   ~1,500–2,000 bytes (team info only, no players), and its post-banner
   header always contains the 4-byte constant `0D 02 00 01`.
4. Searching the *whole file* for that `0D 02 00 01` constant found 418
   hits, all of which fall inside the 15 directory-covered regions —
   **except exactly one**, at offset 628,314 (the tail end of the last
   "foreign stub" region). This means the region *after* offset ~628,300 is
   structurally different: it contains no more foreign-club stubs.
5. In that different region, banner occurrences are much further apart
   (deltas of tens of thousands of bytes instead of ~1,500–2,000), matching
   the 22 "large" deltas (>10,000 bytes) found in the very first
   investigation pass. The banner at offset 629,003 is followed by:
   `E9 07 0D 02 00 00 05 00` then the charmap-decodable bytes
   `33 08 17 04 13` ("R", "i", ?, "e", "r"). Byte `0x17` is **not** in
   `confirmed_real_map.txt` (37 confirmed pairs only), but every other byte
   in that run decodes cleanly to `R_i_er` with a 5-char string-length
   prefix (`05 00`) immediately before it — strongly implying `0x17 = 'v'`
   and the string is **"River"**, i.e. the team's own `short_name` field,
   immediately after a record header that differs from the foreign-stub
   one (`0D 02 00 00` here vs. `0D 02 00 01` for stubs — plausibly a
   type/league flag distinguishing "full domestic team" from "foreign
   reference stub").
6. The next banner after 629,003 is at 721,959 — a gap of exactly 92,956
   bytes, matching the single largest delta found in the very first
   investigation pass. That full span was extracted here.
7. Within the extracted span, real Argentine biographical text was also
   independently confirmed nearby in the same giant post-directory region
   of the file (not necessarily inside this exact 92,956-byte slice, but in
   the same structurally-distinct region): the charmap-encoded byte pattern
   for "Boca" (`23 0E 02 00`) decodes in context to "...a Boca Juniors..."
   at file offset 659,276 (`... A00 A23 0E 02 00 41 2B ...` → "a Boca
   ?uniors", with `0x2B` newly inferred as `'J'`), and "San Lorenzo" (a
   real Argentine club/place name, byte pattern `32 00 0F 41 2D 0E 13 04
   0F 1B 0E`) decodes cleanly hundreds of times throughout this same
   region — both consistent with dense, real Argentine football
   biographical prose (player `career`/`profile`/`birthplace` text fields
   per `pcf_model::Player`), not more short foreign-club stubs.

## Why medium-high confidence, not "confirmed"

- The `short_name` = "River" read is one unconfirmed byte (`0x17`) away
  from certain — but it's corroborated by (a) a length-prefix field that
  matches exactly, (b) `9001,River,Argentina` existing in the real pointer
  catalog, and (c) the surrounding region (not stub-shaped, dense with real
  Argentine club/place names in flowing text) being structurally
  consistent with "this is where the real domestic teams' full data
  lives."
- I did **not** verify the internal player count or a coach record inside
  this specific 92,956-byte span byte-by-byte — I only inspected the first
  ~900 bytes after the header (visible in the investigation transcript)
  and saw large blocks of small structured binary fields consistent with
  `pcf_model::Team`'s numeric fields (capacity/members/league history
  ranges), followed by dense repeating short chunks that plausibly
  correspond to further sub-records, but I did not walk them one by one to
  count ~25–30 players.
- The exact boundary (banner-to-next-banner) assumes the *entire* team
  (team info + tactics + coach + full roster) is wrapped in **one** banner
  for real domestic teams — unlike the ~448 individually-bannered foreign
  stubs earlier in the file. This is a reasonable inference (it matches
  `pcf_codec::dbc::Dbc::read`'s original one-banner-per-team design) but is
  not independently proven end-to-end for this specific slice.

## Why not `.dbc`

`tests/tests/round_trip.rs` scans `fixtures/golden/*.dbc` (case-insensitive
extension match) and asserts `Dbc::write(Dbc::read(bytes)) == bytes` for
every non-`synthetic`-named file. This blob does **not** parse via
`Dbc::read` as-is: the container's real per-record framing (banner +
`E9 07 0D 02 00 00 ...` header) does not match what `Dbc::read` expects
right after the banner (`MAGIC_FE06 = [0xFE, 0x06]`) — the override-file
format `Dbc::read`/`write` implement is for a *different* on-disk shape
(single exported team file) than this bigger `.PKF` container's internal
framing. Forcing this through `Dbc::read`/`write` or editing the codec to
accept it was explicitly out of scope for this investigation. Using a
`.raw` extension keeps it out of the round-trip gate's file discovery
(`collect_dbc_files` in `tests/tests/round_trip.rs` only matches `.dbc`)
so it can sit here as a preserved, honestly-labeled raw sample without
breaking `cargo test`.

## Reproducing the extraction

```
dd if=/c/PCF6AR/DBDAT/EQ003003.PKF \
   of=fixtures/golden/real_river_9001_container_blob.raw \
   bs=1 skip=629003 count=92956
```

(`EQ003003.PKF` was only read, never modified, per the read-only-source
guardrail.)
