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
| Real domestic (Argentina) team records | **55 records located file-wide** (§9), decoded via a corrected 4-byte signature (`0D 02 00 00` at header+2 — the naive 6-byte match only worked for River by coincidence, see §8's UPDATE). River's team-info fields (short_name through president) **confirmed high-confidence** via 5 independently-checkable real-world facts. Coach chain start **confirmed** (real coach name "Ramón Díaz" decodes exactly). **Full player roster confirmed** for River: exactly 27 players, walked end-to-end, real historically-documented names (Burgos, Bonano, Sorín, Gallardo, Saviola, Aimar, etc.) — see §6.6-§6.7. Tactics-block byte offsets confirmed exactly; field identity (jornada/formation_blob) medium-high confidence — see §6.3. |
| Character map | **90 confirmed byte↔glyph pairs.** The original 37 (`fixtures/charmap/confirmed_real_map.txt`, from the manual's hex-editing appendix) plus **53 new ones** (`fixtures/charmap/confirmed_real_map_v2.txt`): 40 derived by decoding all 473 stub-table records (§3, see §7), **5 more** derived from the 55 real Argentina domestic team records (§8.3) — `(`, `)`, digits `2`/`3`/`5`, and `"` (double quote) — and **8 more** (§10) derived by cross-referencing a large external corpus of real override-format DBC files from the community "EDITOR-PM9798" tool, including `0x56` (`'7'`), which resolves the previously-open San Martín (SJ) blocker. This supersedes and reconciles §6.8's 13 provisional single-fact inferences from the domestic-team investigation (11 of 13 match exactly; 1 byte, `0x50`, is corrected — see §7.3). One byte, `0xD5`, remains deliberately open — see §7.4/§10. |
| Full container parser in `pcf-codec` | **Team-info + coach-chain + full player-roster parsing implemented and verified against the real file** in `crates/pcf-codec/src/container.rs` (`parse_team_record`, `find_domestic_team_records`, `parse_player_record`, `parse_player_roster`, `parse_pkf_container`/`parse_pkf_container_verbose`) — new, local types (`ContainerTeamRecord`/`ContainerCoachStub`/`ContainerPlayerRecord`), not a reuse of `pcf_model::Team`/`Coach`/`Player`. With the charmap fix (§8.3), the `0x56` fix (§10), and player-roster parsing (§8.4) all landed, `examples/dump_container.rs` finds **55 real domestic records** and parses **all 55 of them** end-to-end (San Martín SJ, the sole remaining failure as of §8.3, now parses too — see §10), and **all 55 successfully-parsed teams also get a fully-parsed, non-empty player roster** — including River's exactly-27-player roster matching the real 1998-99 squad name-for-name (§6.6-§6.7). See `crates/pcf-codec/examples/dump_container.rs` for an end-to-end demo (now also printing each team's player count and a couple of sample names). **UPDATE (§11):** a real-world bug report on Vélez Sarsfield's own record found and fixed a coach/roster search-order bug (the coach-marker scan wasn't bounded to the region before the player roster, so it could — and for Vélez did — false-positive on prose inside player data and silently discard real players before that point); `budget` remains deliberately unconfirmed/not wired in (see §11.3). |

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

**Now done (see §6):** walked the team-info fields (confirmed, high
confidence) and located the coach-chain start (confirmed via a real coach
name match) and a first plausible player-record marker (medium-high
confidence). **Still not done:** a byte-exact tactics-block boundary, the
full per-player fixed-field layout (the marker/name-string shape matches
but the fields in between don't cleanly match override format), an
end-to-end walk confirming every player boundary in one record, and
locating/comparing additional domestic team records (there should be
roughly 15-20 more: Boca, River, San Lorenzo, Independiente, ... plus
special entries like `9900 Estrellas España`, `9950 Jugadores Libres`,
`9955-9958 Juveniles ...` per the pointer catalog).

## 5. Open questions / next steps

1. ~~**Expand the character map.**~~ **Done** — see §7 (77 confirmed pairs).
2. ~~**Walk one full domestic team record byte-by-byte.**~~ **Done** for the
   player-record layout (§6.6-§6.7: all 27 players confirmed end-to-end,
   very high confidence) and for the team-stats/jornada/tactics region
   (§6.3: exact byte offsets confirmed, field identity medium-high
   confidence; `palmares` appears absent and `formation_blob` appears
   fixed-size rather than length-prefixed in this container). **Still
   open**: the meaning of the 22-byte unexplained block at team-info
   offset 150-172 (§6.3), the variable-length player-record "gap" before
   `short_name` (§6.6), and exactly what `TeamStats.played=17,408`
   represents (possibly a mid-season/not-yet-played placeholder, since
   this data snapshot is "PC Apertura 98/99" mid-tournament).
3. ~~**Enumerate all domestic team records.**~~ **Done** — see §9: 55
   records found (53 real clubs + 2 specials) using a corrected 4-byte
   signature, a near-complete match to the ~60-club Argentina pointer
   catalog block.
4. **Reconcile with `pcf_codec::dbc::Dbc::read`/`write`.** The
   override-file format those functions implement (single
   `BANNER`+`MAGIC_FE06`+team+tactics+coach+players, per PLAN.md Appendix
   A) is NOT the same as this container's internal per-record framing
   (banner + a different header, e.g. `E9 07 0D 02 00 00 ...` for River
   specifically, `0D 02 00 00` at header+2 in general — see §8's UPDATE).
   Whether the *exported* `EQ97####.DBC` override files (which we still
   don't have a real sample of) match Appendix A's documented format, or
   something closer to the container's internal framing, is still an
   open question — this is a deliberate design decision to make later,
   not something to guess at now. (Still open; not addressed by this
   pass.)

## 6. Domestic team record internal structure

Walked byte-by-byte on `fixtures/golden/real_river_9001_container_blob.raw`
(the likely-River record from §4). **Method note:** this section uses the
37-pair confirmed charmap (`fixtures/charmap/confirmed_real_map.txt`) plus
13 *newly inferred* byte→glyph pairs turned up during this pass (listed in
§6.6). Every new inference is corroborated by an independent real-world
fact (a real stadium name, a real founding year, a real president's name,
a real head coach's name) — treated as high-confidence working hypotheses,
not yet merged into the confirmed map file pending a second corpus (per
the open "expand the charmap" effort running in parallel). No
`confirmed_real_map_v2.txt` appeared during this investigation, so this
section stands entirely on the 37-pair map + these 13 inferred pairs.

All offsets below are **relative to the start of the extracted blob**
(i.e. relative to the record's own banner, absolute file offset `629003 +
offset`).

### 6.1 Header (confirmed)

```
offset 0   36    banner "Copyright (c)1996 Dinamic Multimedia"
offset 36   6    E9 07 0D 02 00 00   <- fixed domestic-record header
offset 42   2    05 00               <- u16 LE length prefix for short_name (first string field)
```

The header's last byte (offset 41, part of the 6-byte header: `...00 00`)
is `0x00` for this domestic record vs `0x01` for the foreign-club stubs in
§3 — confirms the domestic/foreign distinction noted in §4 and pins it to
a specific byte position within a 6-byte (not 8-byte) fixed header,
immediately followed by the first string's own length prefix (there is no
gap — `investigate_domestic_team.rs` confirms `header == [E9,07,0D,02,00,00]`
byte-for-byte).

### 6.2 Team info fields (through `president`) — CONFIRMED, high confidence

Field order matches `pcf_model::Team` / `dbc.rs::read_team` **positionally**
almost exactly, with two small unexplained extra bytes (see below). Strings
use the exact same wire shape as the override format's `Reader::string`
(u16 LE length prefix + charmap bytes, no padding).

| offset | bytes | field | value | evidence |
|---|---|---|---|---|
| 44–48 | `33 08 17 04 13` | `short_name` (5-byte string) | `"River"` | matches `team_pointers.csv`'s `9001,River,Argentina`; needs 1 inferred byte (`0x17='v'`) |
| 49–72 | 24-byte string | `stadium_name` | `"Antonio Vespucio Liberti"` | **exact real match** — River's stadium is "Estadio Antonio Vespucio Liberti" (El Monumental). Confirms 2 new byte inferences (`0x37='V'`, `0x11='p'`) |
| 75 | `03` | `country` | `0x03` | positionally where override's single `country` byte sits |
| 76 | `de` | *(unexplained)* | `0xDE` | extra byte not present in override format's `Team` struct — see §6.4 |
| 79–103 | 25-byte string | `long_name` | `"Club Atlético River Plate"` | **exact real match** to River's full legal name. Confirms 1 new inference (`0x31='P'`) |
| 104–107 | `8F 2B 01` + `00` | `capacity` (u24 LE) + zero separator | `76,687` | **matches the real, historically-cited Estadio Monumental capacity figure exactly** |
| 108–111 | `00 00 00` + `00` | `standing_capacity` (u24 LE) + zero separator | `0` | positionally confirms `STANDING_CAPACITY_PRESENT=true` layout (8 bytes total for both capacity fields) |
| 112–115 | `46 00 69 00` | pitch-size pair (2×u16 LE) | `(70, 105)` | **one byte off** from override's fixed `PITCH_SIZE` constant `46 00 6A 00` = `(70, 106)` — see §6.4 for reinterpretation |
| 116–117 | `6D 07` | `founded` (u16 LE) | `1901` | **exact real match** — River Plate was founded in 1901 |
| 118–119 | `00 00` | *(unexplained)* | — | extra 2 bytes not present in override format — see §6.4 |
| 120–123 | `18 F6 00` + `00` | `members` (u24 LE) + zero separator | `63,000` | plausible round member count (medium-high confidence; not independently verifiable like the other fields) |
| 124–146 | 21-byte string | `president` | `"Alfredo Angel Dávicce"` | **exact real match** — Alfredo Dávicce was River Plate's president 1997–2001, contemporaneous with this data. Confirms 3 new inferences (`0x07='f'`, `0x80='á'`) |

Cursor position after `president`: byte offset **147**.

**Confidence: very high.** Five independently-checkable real-world facts
(stadium name, full legal name, stadium capacity, founding year, and
club president's name) all decode correctly and self-consistently using
only 6 new inferred byte pairs total (reused across multiple words) — an
implausible coincidence if the field order or charmap inferences were
wrong.

### 6.3 What comes after `president` through the coach marker — REVISED, high confidence on offsets, medium-high on field identity

Re-examined with exact cursor arithmetic (`investigate_tactics_block.rs`,
new) instead of the earlier rough offset estimates. Cursor position 147
(right after `president`) to the coach marker at 482 is exactly 335 bytes,
which decompose as follows:

```
147–150   3    budget (u24 LE) = 2025           <- matches override position
150–172  22    UNEXPLAINED block (see below)
172–176   4    FF FF FF FF                       <- plausibly affiliate1=affiliate2=0xFFFF ("none"),
                                                     NOT immediately after budget as override does it
176–196  20    league_history[10], but in (division, position) order,
               NOT override's (position, division) -- division=0x00 (First)
               for all 10 entries, positions = 1,5,1,10,7,15,1,1,1,7
               (a plausible historical top-flight finish record for River)
196–210  14    TeamStats-shaped block (6x u16 LE + 2x u8) -- but `played`
               reads as 17,408 if taken literally: implausible for a real
               stat, more likely this represents an in-progress/not-yet-
               played "current season" placeholder (this is a mid-season
               1998/99 snapshot) than a layout error
210–302  92    a block of exactly JORNADA_LEN (92) bytes: a mix of small
               integers (1-21ish, with repeated small-int runs) followed by
               ~40 zero-padding bytes then a few more small values --
               shape-consistent with `Team.jornada`'s own doc comment
               ("opaque positional blob... editor always writes zeros
               here"), except here it's mostly NON-zero, consistent with
               being REAL historical data rather than an unset default
302–476 174    a run of 87 packed u16-LE-shaped values (every other byte
               0x00, all values <256) -- candidate Tactics `formation_blob`.
               NO separate 2-byte length prefix found anywhere nearby
               (scanned exhaustively): most likely this container stores
               the formation blob at a FIXED 174-byte size rather than
               length-prefixed the way the override format does
476–482   6    50 28 00 00 00 00 = touch_pct=80, counter_pct=40, then 4
               zero bytes -- ONE BYTE SHORT of override's 7-byte
               TacticsFixedRaw tail (touch+counter+5 enum bytes); the 4
               zero bytes are each independently valid as
               attack/tackling/marking/clearance (all enums have a valid
               0x00 variant), but there's no room for a 5th (`pressing`)
               byte -- either `pressing` is genuinely absent in this
               container's tactics tail, or it's folded into one of the
               other bytes
```

**New finding: no room for a separate `palmares` blob at all.** The
92-byte `jornada`-shaped block (210-302) is immediately followed by the
174-byte packed-value run with **zero bytes in between** — there is no
34-byte gap anywhere in this region that could hold `PALMARES_LEN`. Best
current reading: this container's domestic-team records include `jornada`
(present, real data, exactly 92 bytes matching the override constant) but
**do not include a separate `palmares` blob** — either it's genuinely
absent for domestic records, or it's zero-length/omitted rather than a
fixed 34-byte reserved region the way the override format always writes
one.

**The 22-byte block at 150-172 remains unexplained**, but its raw bytes end
in `... FF FF FF FF` (bytes 172-176 as read above) — a strong match for
`affiliate1`/`affiliate2`'s `0xFFFF = "none"` sentinel convention (per
`pcf_model::Team`'s own doc comment), which is NOT how the override format
orders it (override puts affiliate1/2 immediately after budget, with no
gap). This is a plausible field-order difference between the container's
native layout and the override's, medium confidence — the 18 bytes before
the FFFF pair (`00 00 00 07 00 30 34 28 2D 2C 24 32 06 00 20 25 28 25`) are
still unidentified.

**Confidence:** high on all byte offsets/counts above (recomputed with
exact cursor arithmetic, reproducible); medium-high on the `jornada`
identification (exact length match + plausible shape); medium on the
`league_history`/`TeamStats` identification (division/position values look
real, but `played`=17,408 is unexplained); low-medium on the
`formation_blob` identification (positionally plausible, shape plausible,
but no length prefix found to confirm it); medium on `affiliate1/2` being
relocated to 172-176 rather than 150-154.

<details>
<summary>Original hypothesis-only writeup (rough offsets; superseded by the exact-offset version above, kept for history)</summary>

Bytes 147–~215 (`0x93`–`0xD7`ish) contain a run of small integer values
consistent with `budget`/`affiliate1`/`affiliate2` (override order) but
not independently verified against a known real fact, followed by a
20-byte block (offset ~176–195, `0xB0`–`0xC3`) of 10 `(division_byte,
position_byte)`-shaped pairs — **division byte is `0x00` (First) for all
10 entries**, and the position bytes decode to `1, 5, 1, 10, 7, 15, 1, 1,
1, 7` — a plausible top-flight finishing-position history for a
historically dominant club like River (several 1st-place finishes).
**Note this is (division, position) order, the reverse of
`dbc.rs::read_league_history`'s (position, division) read order** — either
the container stores it reversed, or this identification is wrong;
flagged as a discrepancy, not silently reconciled.

A 14-byte block immediately after (offset ~196–209, `0xC4`–`0xD1`) is
positionally consistent with `TeamStats` (6×u16 + 2×u8) but the decoded
`played` value (17,408 if read literally) is implausibly large for a
straightforward interpretation — **not resolved**, flagged as unknown.

**Important reclassification vs an earlier draft of this investigation:**
a long stretch of readable prose starting around offset ~300 that mentions
"River Plate" repeatedly alongside "Metro"/"Nacional" (Argentine 1970s–80s
tournament names) and *also* several Italian/French club names is **not**
the team's own palmarés — a team can't have played in Italy or France. It
decodes real football facts consistent with a specific *person's* career
(see §6.5), which means it structurally belongs to the **coach chain**,
not `Team`. This means the actual `Team.jornada`/`Team.palmares`-equivalent
fields (if present at all in this container's layout) are packed into a
much smaller span than initially assumed — between the stats block (~209)
and the coach marker (§6.5, at offset 482), i.e. **at most ~270 bytes**
remain for `jornada` + `palmares` + all of `Tactics` (formation_blob + the
7-byte fixed tail). This is far less than the override format's `92 +
34`-byte fixed blobs would need, so **either those fields are much shorter
in this container, encoded differently (e.g. a length-prefixed string
rather than a fixed blob), or largely absent for domestic records** — not
resolved. A ~174-byte run of packed `u16`-shaped small values (offset
~302–475, `0x12E`–`0x1DB`, all values <256) sitting right before the coach
marker is a plausible candidate for the tactics `formation_blob` (an
"opaque, variable-length positional blob" per `pcf_model::Tactics`) purely
by position, but no 2-byte length prefix matching its byte count (174) or
element count (87) was found immediately before it — **unconfirmed,
low-medium confidence.** The 6 bytes immediately before the coach marker
(`50 28 00 00 00 00`) are plausibly `touch_pct=80, counter_pct=40` (both
in-range percentages) followed by 4 zero bytes where the override format's
7-byte tactics tail would need 5 enum bytes — one byte short; **not fully
resolved.**

</details>

### 6.4 The "extra bytes" pattern (hypothesized)

Twice in the team-info section, the container inserts bytes not present in
the override format's field list, positioned exactly where an override
field boundary falls:

- 1 extra byte (`0xDE`, offset 76) right after `country` and before
  `long_name`'s length prefix.
- 2 extra bytes (`00 00`, offset 118–119) right after `founded` and before
  `members`.

Both are **plausible container-specific fields the exported override
`.dbc` format simply omits** (e.g. reserved/pointer/version bytes specific
to the multi-team container's internal bookkeeping) rather than parsing
errors — the fields on either side of each gap decode to verified-correct
real values, so the field *order* itself is solid; only the *presence of
extra bytes at these two spots* is new information. Not resolved further.

Similarly, the pitch-size 4-byte value (`46 00 69 00` = `(70, 105)`) is one
byte away from the override format's hardcoded constant `PITCH_SIZE = [0x46,
0x00, 0x6A, 0x00]` = `(70, 106)`. Given everything else in this record is
real, non-synthetic data, this is more likely evidence that **this field
is genuinely variable per-team pitch dimensions** (which the override
format's `write_team` just happens to always write as a fixed default) than
a decode error — medium confidence.

### 6.5 Coach chain — CONFIRMED, high confidence

A 2-byte marker `02 02` (**not** the override format's single-byte
`COACH_MARKER = 0x02`) at absolute blob offset **482** (`0x1E2`), followed
by:

```
offset 482   2   02 02              <- coach marker (2 bytes here, not 1)
offset 484   2   AA 04              <- u16 LE "pointer" = 1194
offset 486   2   0A 00              <- length prefix = 10
offset 488  10   33 00 0C 92 0F 41 25 8C 00 1B  -> "Ramón Díaz"
offset 498   2   10 00              <- length prefix = 16
offset 500  16   ... -> "Ramón Angel DIAZ"
```

**"Ramón Díaz" is an exact match to the real head coach of River Plate in
1998, Ramón Ángel Díaz** (River manager 1995–2001). This independently
confirms 4 more new byte inferences (`0x0C='m'`, `0x92='ó'`, `0x8C='í'`,
`0x3B='Z'`, the last one being an uppercase surname convention matching
the earlier `president` field's mixed-case style). This is the single
strongest piece of evidence in this whole investigation — a real name
decoding correctly using inferences made independently, in a different
part of the file, from the ones that decoded the stadium/president.

Later in the coach chain (offset ~1228, `0x4CC`, well past `long_name`),
a run of readable text lists tournament/club names — "Metro River Plate",
"Nacional River Plate" (× several), then **Napoli, Avellino, Fiorentina,
Inter, Monaco** (all foreign clubs) — consistent with a `palmares` or
`career_*` field. Per the guardrails, the exact prose isn't reproduced
here, but structurally: those specific clubs match Ramón Díaz's real,
publicly documented playing career (River Plate → several Italian clubs
in the 1980s → AS Monaco) closely enough that this is almost certainly his
`career_player` or `career_coach` text (he "was_player" before coaching),
not team data — this is what forced the reclassification in §6.3. The
"ND,ND,ND,ND,ND==" career-field default from the reference manual (byte
pattern `2F 25 4D` repeated) was also found nearby, verbatim, at absolute
blob offset 1504 (`0x5E0`), confirming 1 more inference (`0x6B='='`,
completing the trailing `=`).

`investigate_domestic_team.rs`'s coach-marker heuristic scan (`02 02` + u16
pointer + short length-prefixed string with few unmapped chars) finds this
exact hit at `0x1e2` plus 3 more later `02 02` occurrences (offsets
`0x4c03`, `0x6b80`, `0xa581`) whose decoded text fragments ("...dores
juveniles", "...do albiceleste,", "...dor nacional, s...") look like
mid-sentence fragments of ordinary prose rather than new coach records —
i.e. `02 02` is not a uniquely-identifying marker on its own (it also
occurs by chance inside prose byte sequences); the offset-482 hit is
trusted because of the real-name match, not the marker pattern alone.

### 6.6 Full player record layout — CONFIRMED, high confidence (revised)

**This supersedes the original medium-high-confidence writeup below the
line, which was written before the fixed-field layout was cross-checked
byte-for-byte against `dbc.rs::read_player`'s actual field order.** The key
correction: `read_player` puts `short_name`/`long_name` **immediately**
after `marker`+`pointer`+`number` — `slot`/`origin`/`roles[6]`/etc. come
*after* both name strings, not between `number` and `short_name`. Once the
tool (`investigate_player_layout.rs`, new) was fixed to match this order,
it walks **27 consecutive, fully self-consistent player records**, back to
back, from the first marker at blob offset 1238 all the way to the very
last byte of the 92,956-byte blob — with **zero unmapped/garbage bytes**
anywhere in any `short_name`/`long_name`, and every fixed-field enum byte
landing in its valid range.

Confirmed byte layout (marker to next marker):

```
offset+0    1    marker             0x01 (matches override PLAYER_MARKER)
offset+1    2    pointer (u16 LE)   large, team-external load-order-style value (e.g. 6400, 25632, 8787...)
offset+3    1    number             dorsal (matches override `Player::number`)
offset+4    N    gap                unexplained, VARIABLE length (see below) -- NOT part of override's layout
...         2    short_name length prefix (u16 LE)
...         L1   short_name         charmap string
...         2    long_name length prefix (u16 LE)
...         L2   long_name          charmap string
...         1    slot
...         1    origin             (0 = continues, per override semantics)
...         6    roles[6]           each byte 0x00-0x12 (Role enum), matches override exactly
...         1    nationality
...         1    skin               1..=3 (Skin enum), matches override
...         1    hair               1..=6 (Hair enum), matches override
...         1    demarcation        0..=3 (Demarcation enum), matches override
...         4    birth              day(u8), month(u8), year(u16 LE) -- matches override DateRaw exactly
...         1    height_cm
...         1    weight_kg
...         1    birth_country
...         2+L  birthplace         length-prefixed charmap string
...         2+L  x9 more length-prefixed strings, in EXACT override order:
                 debut_club, international, profile, characteristics,
                 palmares, internationality, anecdotes, last_season, career
...         10   attrs[10]          velocidad,resistencia,agresividad,calidad,
                                    remate,regate,pase,tiro,entradas,portero
                                    -- each byte independently in 0..=99
```

This is **byte-for-byte identical** to `dbc.rs::read_player`'s field order
from `slot` onward (i.e. everything after the two name strings matches the
override format exactly, including field count, order, and byte widths) —
the only container-specific deviation is the variable-length `gap` right
after `number` and before `short_name`'s own length prefix.

**The `gap` is genuinely variable, not a fixed extra-bytes pattern**: it is
**3 bytes** for the very first player in the blob (Saccone, offset 1238)
but **0 bytes** for every one of the other 26 players walked. Its meaning
is unresolved; a plausible guess is that it's related to the first
player's position immediately following the coach chain (rather than a
per-player field that's simply usually empty), but this is speculation —
`investigate_player_layout.rs` searches forward from `number` for the first
position where a `u16` LE value in `1..=40` is followed by that many
mostly-decodable charmap bytes, rather than assuming a fixed width, and
reports whatever gap length that search lands on.

**Independent real-world verification — very high confidence.** All 27
decoded `short_name`/`long_name` pairs are real, historically documented
1998-99 River Plate first-team-squad players (only club/player identifying
names are reproduced here per this doc's own guardrails, not their
free-text profile/career fields):

Saccone, Costanzo, **Burgos** ("Germán Adrián Ramón BURGOS" — the real
Germán Burgos, River/later Atlético Madrid's long-time backup keeper),
**Bonano** ("Roberto Oscar BONANO" — real River/Rosario Central
goalkeeper), Biscay ("Matías BISCAY" — real River defender, later a
Guardiola-era Barcelona/Man City assistant coach), Villalba, Acosta,
Martínez, Sarabia, Placente ("Diego PLACENTE" — real River/later
international defender), Paz, Hernán Díaz, **Berizzo** ("Eduardo BERIZZO"
— real River/Celta/national-team defender), **Sorín** ("Juan Pablo SORIN"
— real River/Argentina captain), Gómez, **Gallardo** ("Marcelo Daniel
GALLARDO" — real River legend, later its own record-breaking manager),
Astrada, Escudero, Gancedo, Berti, **Solari** ("Santiago Hernán SOLARI" —
real River/Real Madrid player, later Real Madrid manager), **Saviola**
("Javier Pedro SAVIOLA" — real River/Barcelona striker, breaking through
in exactly this 1998-99 season), **Angel** ("Juan Pablo ANGEL Arango" —
real River/Aston Villa striker), Castillo, **Pizzi** ("Juan Antonio PIZZI
Torroja" — real River/Spain international striker), Rambert, and
**Aimar** ("Pablo César AIMAR" — real River/Valencia playmaker, also
breaking through in this season). This roster is essentially a complete,
correct match to River Plate's real 1998-99 squad — an implausible
coincidence if the layout, marker-boundary logic, or charmap were wrong.

**The `attrs.portero` (goalkeeper ability) field independently confirms
`demarcation`/`roles`**: the requested "recognizably realistic pattern"
shows up cleanly —

| player | demarcation | portero |
|---|---|---|
| Saccone | Gk (0) | 75 |
| Costanzo | Gk (0) | 78 |
| Burgos | Gk (0) | 85 |
| Bonano | Gk (0) | 90 |
| Biscay | Def (1) | 12 |
| Villalba | Def (1) | 17 |
| Aimar | Mid (2) | 12 |

— all 4 goalkeepers cluster at 75-90, all non-goalkeepers checked are
≤20. `birth` dates are all plausible (e.g. Burgos 16/04/1969, Aimar
03/11/1979), and several `birthplace`/`debut_club` values are real,
independently-checkable Argentine facts (e.g. Aimar's real hometown is
Río Cuarto, Córdoba — decoded exactly).

One soft anomaly: `height_cm`/`weight_kg` read as `0` for a few of the
less-prominent players (Saccone, Villalba) while reading correctly for
the more prominent ones (Burgos 188cm/75kg, Bonano 188cm/75kg, Biscay
183cm/73kg, Aimar 168cm/60kg — all plausible). Given every other field
for those same "zero" players decodes correctly (birth date, birthplace,
career text), this looks like incomplete data entry in the original 1998
database for lesser-known squad members, not a layout error.

**The record boundary is exact**: every one of the 27 parses ends exactly
on the next record's `0x01` marker byte, with **no slack, drift, or
extra/missing bytes accumulating** across 27 consecutive records. The
final (27th) player, Aimar, ends 1 byte before the very end of the
92,956-byte blob — that trailing `0x00` byte is presumably an end-of-roster
terminator.

**Confidence: very high** on the entire fixed-field layout, the boundary
logic, and the record count (27 players, complete). The only open point is
the meaning of the variable-length `gap` before `short_name`.

---

<details>
<summary>Original medium-high-confidence writeup (superseded by the above; kept for history)</summary>

Immediately after the coach chain's career-default pattern (offset 1504)
and a few more unresolved bytes, a marker byte `0x01` appears at absolute
blob offset **1238** (`0x4D6`):

```
offset 1238   1   01                 <- marker (matches override PLAYER_MARKER)
offset 1239   2   00 19              <- u16 LE = 6400 (pointer? unusually large)
offset 1241   1   01                 <- number (dorsal) = 1
offset 1242+  ... -> short_name "Saccone", long_name "Alejandro SACCONE"
```

This was originally read as "player header doesn't cleanly match
`pcf_model::Player`'s field order" because `slot`/`origin`/`roles`/etc.
were assumed to sit *before* the name strings (mirroring the doc's naive
reading of struct field order) rather than *after* them, as
`dbc.rs::read_player` actually implements it. Once corrected, the layout
matches essentially exactly -- see the "CONFIRMED" version above.

</details>

### 6.7 Player count — CONFIRMED: exactly 27 players

Superseding the earlier heuristic-based estimate: walking the full,
corrected field layout (§6.6) from the confirmed first marker at offset
1238 finds **exactly 27 player records**, back-to-back, consuming the
entire remainder of the blob (the 27th player, Aimar, ends 1 byte before
the blob's end). This is a real, plausible 1998-99 Argentine top-flight
registered-squad size, and matches the real roster almost name-for-name
(§6.6).

`investigate_domestic_team.rs`'s older heuristic (`0x01` + u16 "pointer" in
`1..=2000` + dorsal `1..=40`) undercounted for two reasons now understood
precisely: (1) it excluded the many real players whose `pointer` value
exceeds 2000 (confirmed real pointers range from ~6400 to ~25600), and (2)
its false-positive cluster at offsets `0xD9`-`0xE4` falls inside the
Team-info region now identified in §6.3 as part of the league-history/stats
blocks, not player data.

### 6.8 New charmap inferences from this pass (not yet merged into `confirmed_real_map.txt`)

| byte | glyph | corroborating word |
|---|---|---|
| `0x17` | `v` | "Ri_er" → River |
| `0x37` | `V` | "_espucio" → Vespucio |
| `0x11` | `p` | "Ves_ucio" → Vespucio |
| `0x31` | `P` | "River _late" → Plate |
| `0x07` | `f` | "Al_redo" → Alfredo |
| `0x80` | `á` | "D_vicce" → Dávicce |
| `0x6B` | `=` | "ND,ND,ND,ND,ND=_" (manual's documented default) |
| `0x50` | `0` | "_,_,_,_,_====" numeric-default field near coach chain |
| `0x0C` | `m` | "Ra_ón" → Ramón |
| `0x92` | `ó` | "Ram_n" → Ramón |
| `0x8C` | `í` | "D_az" → Díaz |
| `0x3B` | `Z` | "DIA_" → DIAZ |
| `0x0B` | `j` | "Ale_andro" → Alejandro |

These are held only in `crates/pcf-codec/examples/investigate_domestic_team.rs`
(as an `EXTRA_INFERRED` table) pending reconciliation with whatever the
parallel charmap-expansion effort produces from the foreign-clubs stub
corpus — deliberately not written into `fixtures/charmap/confirmed_real_map.txt`
by this investigation, since that file's own header says every entry there
should be cross-checked across independent real strings the way the
original 37 were, and this pass didn't do that level of cross-checking
(each new pair here is corroborated by exactly one real-world fact, not
several).

**Reconciliation (§7 below):** the stub-table cross-reference pass
independently re-derived 11 of these 13 pairs (`0x17,0x37,0x11,0x31,0x07,
0x80,0x0C,0x92,0x8C,0x3B,0x0B`) from a much larger, multi-citation corpus —
all 11 match exactly, a strong cross-validation between two unrelated
parts of the file (foreign stub records vs. a domestic team+coach record).
One pair, `0x50`, is **corrected**: this section guessed `'0'` from a
single ambiguous placeholder-field context; §7.3 shows 3 independent real
club/stadium names (`"Munich 1860"`, `"1º de Maio"`, `"19 de Mayo"`) all
requiring `0x50='1'` instead, with `0x51='0'`. `0x6B='='` isn't addressed
either way — it never came up in the (name-only) stub-table corpus.

## 7. Character map expansion via stub-table cross-reference

Per the open question in §5.1: the 473 stub records decoded in §3 are a
large known-plaintext corpus (each `short_name`/`stadium_name` is a real,
independently verifiable club or stadium name). This section cross-checked
every one of them against real football knowledge (not just
`team_pointers.csv` directly — see §7.1) to expand the charmap from 37 to
**77 confirmed pairs**, written to the new file
`fixtures/charmap/confirmed_real_map_v2.txt` (kept separate from the
original `confirmed_real_map.txt`, whose provenance is the unrelated
hex-editing manual — see that file's own header, and
`fixtures/charmap/README.md`, for why they're not merged into one file).

### 7.1 Method

`crates/pcf-codec/examples/dump_stub_table.rs` (new tool, alongside
`investigate_pkf_dir.rs`) walks all 15 directory blocks end to end and
lossy-decodes every record's `short_name` and `stadium_name` fields (the
`long_name`/numeric-fields region past `stadium_name` uses a different,
not-yet-understood framing — see §7.4 caveat below — so it was **excluded**
from charmap evidence entirely, only `short_name`/`stadium_name` were
used). Unmapped bytes render as `[XX]` so gaps are visible.

The first surprise: the stub table's physical order does **not** follow
`team_pointers.csv`'s fixed pointer numbering (e.g. record index 5 decodes
to "Valencia C.F.", not pointer `0006`'s "Sevilla"). Instead the physical
order follows the **real 1998–99 season's domestic top-flight standings**
for each country in turn — Spain's 20 La Liga clubs, then Italy's 18
Serie A clubs, England's 20 Premier League clubs, Germany's 18
Bundesliga clubs, and on through dozens more countries in roughly (but not
exactly) the same country sequence as `team_pointers.csv`'s row order,
before ending mid-block on special entries like "Jugadores Libres". Once
this was understood, each gappy decode was resolved using plain football
history/geography knowledge (which real club/stadium name is the only one
that fits the visible letters), then cross-checked against every other
occurrence of the same byte anywhere else in the corpus.

### 7.2 Result: 40 new pairs, zero contradictions in the alphabet itself

All 40 new pairs (full list with hex-offset citations in
`confirmed_real_map_v2.txt`'s comments) plus the original 37 were
cross-checked against all ~940 decoded string fields with **no byte found
decoding inconsistently** anywhere in the alphabet/punctuation/digit
mappings. Highlights:

- **Full lowercase+uppercase consonant alphabet** confirmed, following the
  same systematic `lowercase + 0x20 = uppercase` pattern as the original
  37 pairs (verified directly, not just assumed, for every new letter
  except uppercase X, which never appeared in the corpus and is left
  unconfirmed).
- **10 accented vowels**: á à é(orig) è í ï ó ö ú ü, plus ñ and ç.
- **Digits** `0,1,4,6,8,9` (embedded inside literal name strings like
  "Schalke 04", "Munich 1860", "19 de Mayo" — not the separate binary
  numeric fields).
- **Punctuation**: apostrophe (`'`), hyphen (`-`), masculine ordinal
  indicator (`º`).

### 7.3 One real contradiction found and resolved: byte `0x50`

The domestic-team investigation (§6.8) had tentatively guessed `0x50='0'`
from a single ambiguous placeholder-field context. This pass found **3
independent, unambiguous real-name citations** all requiring `0x50='1'`:
`"Munic[09] [50][59][57][51]"` = "Munich 1860" (global record 70),
`"[50][DB] Maio"` = SC Braga's real stadium "1º de Maio" (global 104), and
`"[50][58] de [0C]a[18]o"` = Samsunspor's stadium rendered "19 de Mayo"
(global 180) — with `0x51='0'` confirmed separately from the same two
"1860"/"Schalke 04" words. `0x50='1'` is adopted; the earlier `'0'` guess
is superseded.

### 7.4 One open, unresolved byte: `0xD5`

`0xD5` appears exactly once, in `"[2B]ose[D5] Go[0C]es"` (global record
107, Estrela da Amadora's stadium, expected "José Gomes") — sitting right
where an accented é would complete "José". `0x88` is already the
independently-confirmed é (from the original 37-pair corpus, re-verified
throughout this one too), so two different bytes both meaning "é" would be
an unusual font-table duplication. Left **unresolved** rather than forced
— a single occurrence isn't enough to say whether it's a real second
glyph, a data-entry variant, or something else.

### 7.5 One data anomaly (not a codec issue): "Tofi[p] Bakhramov"

Byte `0x11='p'` is confirmed by **110+ independent occurrences** across a
dozen languages (Camp Nou, Sporting, Perugia, "Park", Apoquindo,
Pachencho...). One record (global 414, Neftchi's stadium) decodes as
"Tofi[11] Bakhramov" — the real name is "Tofiq Bakhramov" (Azerbaijani
transliteration), which would need `0x11='q'`. Given the overwhelming,
consistent evidence for `p`, this is almost certainly a data-entry quirk
in the *original 1998 game's own database* (a p/q transliteration mixup),
not a codec ambiguity — `0x11='p'` was kept, and this is flagged here as a
curiosity rather than a contradiction requiring a fix.

### 7.6 What's still not decodable

The `long_name` field (and whatever numeric fields follow it) inside stub
records uses a framing that `dump_stub_table.rs`'s simple
"2-byte-length-prefix" parser cannot reliably locate — attempting it
produces garbled text mixed with what's almost certainly raw binary
(mostly `0x00` padding, which happens to decode as `'a'`) rather than a
real second name string. This was excluded entirely from the charmap
evidence in this pass (see §7.1) rather than risk contaminating the map
with mis-parsed binary. Understanding the `long_name`/trailing-fields
framing remains open future work.

## 8. `crates/pcf-codec/src/container.rs` — a real parser, and a bug it exposed

Beyond investigation-only example tools, `crates/pcf-codec/src/container.rs`
is a genuine, tested production module implementing §6's confirmed
team-info + coach-chain fields (`ContainerTeamRecord`, `ContainerCoachStub`,
`parse_team_record`, `find_domestic_team_records`, `parse_pkf_container[_verbose]`)
— deliberately **not** reusing `pcf_model::Team`/`Coach` (this container's
confirmed structural differences from the override-`.dbc` format mean those
frozen types shouldn't be stretched to also describe it) and **not**
attempting player parsing (§6.6-§6.7's layout isn't confirmed enough yet).

### 8.1 A real bug, caught by actually running the real-fixture test

The first version of `find_domestic_team_records` computed a "stub table
end" floor from the **last directory block's last entry's own
`offset + length` fields**, then only looked for domestic records past
that floor. This shipped with 23 passing unit tests — all against
hand-built synthetic bytes, none of which could have caught this, because
the bug was purely about whether that floor was *correct on real data*.

Running the real-fixture test with the actual file mounted
(`docker run ... -v /c/PCF6AR:/c/PCF6AR:ro ...`, matching the hardcoded
path `/c/PCF6AR/DBDAT/EQ003003.PKF` the test checks for) immediately
failed: `find_domestic_team_records` returned **zero** ranges, even
though River's record is real and present. Diagnosis with
`investigate_pkf_dir.rs`:

```
block 13: 32 entries, file bytes [627056, 628272)
    entry 31: ... offset=1654868 (expected Some(1654868)) length=4270 (expected Some(5491)) flag=1 trailing=0x04  <-- MISMATCH
```

Block 13's *last* entry — one of the 14 "mismatch" entries §3 already
flagged as an artifact — doesn't point to the physically-next banner
within its own block's local run; it points to file offset **1,654,868**,
which is *inside block 14's own range* (`[1659139, 1660089)`). This means
blocks 0-13 and block 14 are **not one contiguous stub table**: there are
at least two separate clusters of directory-described stub records in the
file — one ending around byte ~628,272, another starting around
~1,654,868 — with River's real domestic record sitting in the gap between
them at 629,003. Using "the last directory block's last entry" as a
stub-table-end floor was built on a false assumption (that all 15 blocks
describe one contiguous run), and it silently excluded the one real
domestic record that actually exists in the file.

**Fix:** `find_domestic_team_records` no longer computes or uses any
directory-derived floor at all. It scans every banner occurrence in the
*whole* file and keeps only the ones whose following 6-byte header
matches the confirmed domestic shape (`E9 07 0D 02 00 00`, vs `...01` for
foreign stubs) — a check that's unambiguous regardless of where in the
file a record physically sits. Re-run against the real file after the
fix: `parses_real_river_record_from_the_users_own_pkf_if_present` passes,
asserting River's exact real facts (stadium, capacity, founded year,
president, coach name) end to end.

### 8.2 A genuine, honest finding: only ONE domestic record exists in this file

With the fix in place, `examples/dump_container.rs` against the real
`EQ003003.PKF` finds **exactly one** domestic team record in the entire
1,779,284-byte file: River. Not the ~60 Argentina teams the pointer
catalog (`fixtures/pointers/team_pointers.csv`, pointers 9001-9061 plus
specials) might suggest should be present.

This is **not** a bug — the header-based scan is unconditional and
file-wide, so there's no floor or filter left that could be hiding
records. It means either: (a) this particular install's `EQ003003.PKF`
genuinely only ships one fully-fleshed-out domestic team (plausible for a
game that otherwise generates/derives most teams' full data at
runtime/first-load, with River perhaps being a bundled "sample" or
default), (b) the other ~60 teams exist somewhere in this file under a
**different** record header/shape entirely (not `E9 07 0D 02 00 00`) that
hasn't been identified yet, or (c) they live in a different file
altogether. Whichever it is, don't assume scanning for "more records shaped like
River" will find them — a genuinely different signature would be needed.
Left as an open question for further investigation (see §5).

**UPDATE (see §9): resolved — it's (b), and the "genuinely different
signature" needed is smaller than expected.** The 6-byte match
`E9 07 0D 02 00 00` only worked for River by coincidence: its **first 2
bytes are NOT a fixed constant** across domestic records — they vary per
team (confirmed values include `21 07`, `E2 05`, `47 06`, `39 07`, ...).
The real, verified-constant domestic-record signature is just the **4
bytes at header offset +2** (i.e. banner+38): `0D 02 00 00`. Scanning the
whole file for *that* narrower signature finds **55 domestic records**,
not 1 — see §9 for the full list and cross-reference against the pointer
catalog.

**UPDATE 2: `container.rs` has been fixed to match** — `DOMESTIC_HEADER`
(the old, wrongly-strict 6-byte constant) was replaced with
`DOMESTIC_HEADER_TAIL` (4 bytes) + a new `header_prefix: [u8; 2]` field on
`ContainerTeamRecord` (the varying leading bytes, kept verbatim rather
than discarded, since their meaning isn't confirmed). Re-running
`examples/dump_container.rs` against the real file after the fix: **39 of
the 55 real domestic records now parse successfully** end-to-end through
`parse_pkf_container_verbose`, with real, independently-checkable data —
e.g. Boca's decoded `president` is "Mauricio Macri", who really was Boca
Juniors' president in 1998. The remaining 16 fail with a typed
`charmap_unknown_byte` error (not a panic — `parse_pkf_container_verbose`
correctly isolates the failure to that one record and keeps going): every
failing team's `short_name` contains a parenthesis (e.g. "San Martín
(Tuc)", "Estudiantes (LP)", "Talleres (Cba)") — parentheses aren't in the
77-pair `confirmed_real_map_v2.txt` charmap yet. Adding `(`/`)` (and
re-running against these exact real failures to confirm) is a
well-scoped, low-risk follow-up, not attempted in this pass. Most
successfully-parsed teams show `coach: (none found)` — expected per the
module's design (`Option<ContainerCoachStub>`); the confirmed `02 02`
coach-marker heuristic is real but not guaranteed present/locatable for
every team's data (smaller/lower-tier clubs plausibly have thinner
records).

### 8.3 Charmap follow-up: the 16 `(`/`)` (and digit/quote) failures — RESOLVED, 54/55 now parse

The well-scoped follow-up flagged in §8.2 UPDATE 2 is done. Diagnosis (via a
temporary scratch investigator, since removed, that lossy-decoded every
domestic record's `short_name`/`stadium_name`/`long_name` and flagged every
unmapped byte, not just the first-blocking one per field like
`parse_team_record`'s hard-erroring `?` naturally stops at): the 16 failures
were NOT all parentheses-only. Five distinct new bytes were needed, each
confirmed by a complete, exact-length real-name decode (methodology
identical to §7 — infer from the one real name/fact that fits, then
cross-check every other occurrence for self-consistency):

- **`0x49='('`, `0x48=')'`** — confirmed by 14+ occurrences, always in
  matched pairs immediately bracketing a real disambiguating abbreviation,
  matching §9's own already-decoded short-name list exactly (e.g. "Gim.
  Esgrima (LP)", "San Martín (Tuc)", "Talleres (Cba)", and the
  `long_name`-field instances "Club Atlético Belgrano (Córdoba)"/"Talleres
  (Córdoba)"). Zero contradictions.
- **`0x52='3'`, `0x53='2'`** — confirmed by a complete, exact-length (12/12
  bytes, zero gaps) decode of Club Gimnasia y Esgrima de Jujuy's real
  stadium name (`banner@0x14f5d8`): `"23 de agosto"` — the real "Estadio 23
  de Agosto", named for the 23 August 1812 Éxodo Jujeño. `0x53='2'` is
  independently corroborated by 2 further occurrences elsewhere in the
  corpus (San Martín SJ's and Newell's stadium fields, both reading
  plausible "2? de <month>" patterns), though those two aren't independently
  fact-checked the way the Jujuy citation is.
- **`0x54='5'`** — confirmed by a complete, exact-length (11/11 bytes)
  decode of Club Atlético Unión (Santa Fe)'s real stadium name
  (`banner@0x18c2a0`): `"15 de Abril"` — the real, well-documented "Estadio
  15 de Abril". 1 citation, but a complete/exact-length match.
- **`0x43='"'`** (double quote) — confirmed by a complete, exact-length
  (43/43 bytes) decode of Almirante Brown (Arrecifes)'s real stadium name
  (`banner@0x1a5ef5`): `Municipal "General San Martín" de Arrecifes` — a
  standard Argentine municipal-stadium naming convention, with the byte used
  consistently as both the opening and closing delimiter. 1 citation, but a
  complete/exact-length match.

All 5 pairs were added to `fixtures/charmap/confirmed_real_map_v2.txt`
(same file, same provenance/methodology — this is an extension of the
existing corpus-cross-reference effort, not a new source). Re-running
`examples/dump_container.rs` against the real file: **54 of the 55 real
domestic records now parse successfully end-to-end** (up from 39).

**One byte was left open at the time**: `0x56`, appearing once in San
Martín (San Juan)'s stadium field (`"2[56] de Septiembre"`, presumably
`'7'`, `banner@0x1955c7`). An apparent byte-table pattern (`0x50`/`0x51`,
`0x52`/`0x53`, `0x54`/`0x55` all being adjacent-byte-pair-swapped digit
pairs, which would predict `0x56`/`0x57` similarly, and `0x57='6'` is
already independently confirmed) made `0x56='7'` plausible, but this pass
could not independently fact-check "27 de Septiembre" against a known real
fact for this specific club the way every other pair above was verified —
left unresolved rather than forced, same precedent as `0xD5` in §7.4. This
was the one remaining failure in `dump_container.rs`'s 55-record run.
**Resolved in §10** via an unrelated external corpus.

### 8.4 Full player-roster parsing landed in `container.rs` — two more real bugs caught by real-file testing

§6.6-§6.7's confirmed player-record layout is now implemented in
production code: `ContainerPlayerRecord`, `parse_player_record`,
`parse_player_roster`, wired into `ContainerTeamRecord::players` (replacing
the old `trailing_raw`-only placeholder — `trailing_raw` now just holds
whatever's left over after the best-effort roster walk, normally the
1-byte end-of-roster terminator §6.7 already documented). Consistent with
this module's whole history (§8.1/§8.2), the first version that passed all
synthetic unit tests still failed against the real file, in two distinct
ways:

**Bug 1: the gap-search accepted the wrong (too-short) gap length.** The
original gap-search (mirroring `investigate_player_layout.rs`) only checked
that a candidate `short_name` length-prefix's own bytes decoded via the
charmap. That was safe when the charmap was small, but now that
`confirmed_real_map_v2.txt` covers 82 pairs — most of the printable byte
range — a short run of essentially arbitrary bytes at the *wrong* candidate
gap length would often happen to decode to *something* anyway, without
erroring. Running against River's real roster found this immediately:
gap_len=3 (the confirmed-correct gap for the very first player, Saccone)
kept losing to gap_len=0/1/2, which decoded successfully but nonsensically.
**Fix:** the gap-search now requires the *entire* rest of the player
record — `short_name` onward, via a new `parse_player_body` helper — to
both parse successfully and land every enum-shaped byte in its confirmed
real-data range, not just that `short_name`'s own bytes happen to
charmap-decode. A misaligned gap reliably breaks that stronger check even
when it wouldn't have broken the old, narrower one.

**Bug 2: `long_name` and the 9 free-text fields don't all decode strictly
under the current charmap.** Even with bug 1 fixed, parsing stalled after
7 of River's 27 players: player 8 (Martínez)'s `long_name`, "Jorge Daniel
MART[byte]NEZ", contains a byte (`0xAC`, plausibly an accented uppercase
Í) that isn't in the 82-pair charmap at all — confirmed by a single clean,
byte-boundary-isolated citation, but not independently cross-checked
enough to add to `confirmed_real_map_v2.txt` with this file's usual rigor,
and (per this project's own guardrails) full biographical prose fields
like `profile`/`career`/`anecdotes` are expected to need many more such
bytes the charmap was never built to cover (it was built from short club
names, not life-story prose). Requiring every string field to decode
strictly would block roster parsing for any player whose full legal name
or biography happens to need an uncommon byte — which turned out to be
the common case, not an edge case. **Fix:** `short_name` alone stays on
the strict `Reader::string` path (§6.6 confirmed zero unmapped bytes
across all 27 real players for this specific field, and it's what the
gap-search's plausibility check actually depends on); `long_name` and the
10 fields from `birthplace` through `career` now decode **tolerantly** via
a new `read_lossy_string`/`decode_lossy` helper (substituting
`'\u{FFFD}'` for any charmap-unmapped byte instead of erroring). This is a
deliberate, documented leniency for identity/biographical *text content*
specifically — it does not touch `charmap.rs`'s own strict `decode`/
`encode` contract (still hard-erroring, still what `dbc.rs`'s
byte-exact override-format round-trip depends on) — and does not weaken
the roster's *structural* correctness, which still rests on `short_name`
decoding strictly plus every enum-shaped byte (`roles`/`skin`/`hair`/
`demarcation`/`attrs`) landing in its confirmed real-data range.

With both fixes, `parses_real_river_record_from_the_users_own_pkf_if_present`
(`crates/pcf-codec/src/container.rs`) passes against the real file,
asserting `players.len() == 27`, the real goalkeeper/portero pattern from
§6.6's table, and several more of the historically-documented real players
by name at their confirmed roster positions. Running
`examples/dump_container.rs` file-wide: of the 55 real domestic records,
**54 parse successfully** (§8.3's still-open `0x56` charmap byte blocks the
55th, San Martín SJ) and **all 54 of those also get a complete,
non-empty player roster** — e.g. San Lorenzo (23 players), Boca (23),
Independiente (21), all the way down to smaller clubs like Newell's (1
player — plausibly a thinner real record for a lower-priority club, the
same "thinner records for smaller clubs" pattern §8.2 already noted for
missing coach chains) and the special "Juveniles ARGENTINA" entry (52).

## 9. Enumerating all domestic team records — CONFIRMED, high confidence

Using the corrected 4-byte signature (`0D 02 00 00` at banner+38, §8's
UPDATE) and scanning the whole 1,779,284-byte file for every banner
occurrence, `enumerate_domestic_teams.rs` (new) finds **55 domestic team
records**, from River (banner @629,003) through the end of the file. Every
record's `short_name` (first length-prefixed string, at header+8) decodes
cleanly with the 77-pair `confirmed_real_map_v2.txt` charmap — zero
unmapped bytes in any of the 55 names.

Decoded `short_name`s, in physical file order (banner offsets in the raw
`.PKF`, not the golden blob):

River, San Lorenzo, Vélez, Argentinos Jrs., Newell's, Belgrano, Lanús,
Banfield, Rosario Central, Gim. Esgrima (LP), Independiente, Racing, Boca,
Huracán, Platense, Gimnasia (J), Ferro, Dep. Español, Colón, Estudiantes
(LP), Dep. Morón, Arsenal, Tigre, San Martín (Tuc), Ctral. Córdoba, Los
Andes, Talleres (Cba), All Boys, Gimnasia y Tiro, Unión, Godoy Cruz, San
Martín (SJ), Atl. Rafaela, Huracán (Ctes), Nueva Chicago, Instituto (Cba),
Quilmes, Douglas Haig, Atl. Tucumán, Chacarita Jrs., Atlanta, San Miguel,
Defensa y Just., Aldosivi (M.P.), Almagro, Alte. Brown (Ar.), Cipoletti
(RN), Estudiantes (B.A.), Olimpo, San Martín (Mza), Juv. Antoniana, Gim.
Entre Ríos, El Porvenir, then two specials: **Estrellas ARGENTINA** and
**Juveniles ARGENTINA**.

**Cross-reference against `fixtures/pointers/team_pointers.csv`'s
Argentina block** (pointers 9001-9061, ~60 real clubs after excluding the
gap at 9049, plus specials `9903 Estrellas Argentina` / `9958 Juveniles
Argentina` elsewhere in the catalog): **53 of the ~60 numbered club
pointers have a matching record here, in the exact same relative order as
the catalog**, plus both of those 2 specials. **Missing** from the file
(present in the catalog but no matching record found): `9050 Dep.
Italiano`, and the catalog's final 5 lowest-ranked entries — `9057
Argentino Ros.`, `9058 Temperley`, `9059 Independente (Mza)`, `9060 Villa
Mitre`, `9061 Racing (Cor)`. Also not found: the catalog's `9900 Estrellas
España` (a Spain-context special, not expected in the Argentina block
anyway) or `9950 Jugadores Libres` (a free-agents pool, plausibly stored
under a different, non-domestic-team shape entirely, or in a different
file).

This is a clean, close-to-complete match (53/60 clubs + both found
specials = 55/62 catalog entries), consistent with the guardrail's
expectation that "not every pointer-catalog entry necessarily has a full
DBC-style record in every install" — the handful of missing entries are
plausibly lower-division/placeholder clubs whose data wasn't fully
populated in this particular install/edition, not evidence the enumeration
method itself is incomplete (the scan is unconditional and file-wide, with
no floor/filter that could hide records, same as `container.rs`'s own
approach once given the corrected signature).

## 10. Character map: EDITOR-PM9798 cross-reference — 8 new pairs, `0x56` resolved

A community-shared editor tool bundle for PM97/PM98/PCPREMIER60 (earlier/
related entries in the same Dinamic Multimedia "PC Fútbol"/"PC Premier
Manager" engine family — **not** this project's own game, and **not**
copied into this repo; treated purely as an external reference corpus, the
same way `fixtures/pointers/*.csv` is treated as community-sourced
reference metadata rather than a redistributable game asset) was found to
contain 1,434 real `EQ97####.DBC` override-format files: `DBCS/PM97/DBDAT/
EQUIPOS` (480 files), `DBCS/PM98/DBDAT/EQUIPOS` (476 files), and `DBCS/
PCPREMIER60/DBDAT/EQ030022` (478 files) — confirmed same
`"Copyright (c)1996 Dinamic Multimedia"` banner and same length-prefixed
charmap-encoded string shape as this project's own understanding of the
override format (PLAN.md Appendix A). These are Spanish/English/Italian/
other European team and player records, giving a much larger and more
diverse real-name corpus than the single Argentina `.PKF` this project has
direct access to, especially for accented-letter coverage.

### 10.1 Method

Two throwaway investigation tools (not part of the codec contract, not
committed as anything but disposable scratch code — see the tools table
below): `investigate_editor_pm_dbc.rs` decodes every file's `short_name`/
`stadium_name`/`long_name` (the team-info strings, which sit at a fixed,
version-independent offset right after the banner+6-byte-header) lossily,
the same way `dump_stub_table.rs` does. A second tool,
`investigate_editor_pm_dbc_full.rs`, attempted a full structural walk using
this project's own `dbc.rs` field layout, but that immediately surfaced a
real, useful negative result (§10.2) rather than more names, so the bulk of
the new evidence below comes from a third technique: lossily decoding the
**raw remaining bytes of each file** after `long_name`, with no framing
assumed at all. Because most of a real DBC file's bytes are still
charmap-mapped text (names/free-text fields) interleaved with the raw
binary numeric fields (which mostly happen to decode to `'a'`, since
`0x00='a'` is already confirmed and zero-padding is common), this crude
approach still surfaces long, readable, real player-name fragments any
place the byte alignment happens to land on a string, without needing to
solve this corpus's exact field layout at all. Every new byte below was
inferred from a real, checkable name/fact the same way §7's methodology
requires, then cross-checked against every other occurrence found.

### 10.2 A genuine negative finding: this corpus's team-info layout differs from `dbc.rs`'s assumed one

`investigate_editor_pm_dbc_full.rs`'s structural walk (mirroring
`dbc.rs::read_team`/`read_player` field-for-field) fails partway through
every single file: the two bytes this project's `dbc.rs` assumes are a
literal `FE 06` magic right after the banner are **not** `FE 06` in this
corpus (confirmed not a bug in the tool — skipping exactly 6 bytes
positionally, without checking their content, reliably lands on a valid
`short_name` length prefix for all 1,434 files), and the `pitch_size` field
is confirmed (again) to be genuinely per-team variable data, not the fixed
constant `dbc.rs` hardcodes (consistent with §6.4's own finding on the real
Argentina file). Past that point, the `capacity`/`standing_capacity`/
`president`/tactics-block layout diverges enough (this corpus's files are
only ~2.7–4.1 KB, vs. ~93 KB for River's real container-internal record)
that the structured walker hits EOF a few hundred bytes in, every time.
**This is a real, useful finding, not a bug to fix**: it means the literal
byte-for-byte layout `dbc.rs` currently implements (based on PLAN.md
Appendix A's undocumented, unverified assumptions, per open question §5.4)
is likely specific to this project's own edition/version, not a universal
constant across the whole engine family — flagged here rather than
"corrected" against a different, equally-unverified-for-*this*-game
layout. `dbc.rs` was **not** modified as a result of this finding (out of
scope for this pass, and doing so without a real Apertura 98/99-specific
sample to verify against would just trade one unverified assumption for
another).

### 10.3 Eight new confirmed pairs

All 8 are documented with their full citation trail directly in
`fixtures/charmap/confirmed_real_map_v2.txt` (search for "third pass" in
that file) rather than duplicated here; summary:

| byte | glyph | confidence basis |
|---|---|---|
| `0x56` | `7` | 2 independent real club-name facts: Finnish club MyPa's real full name "Myllykosken Pallo **-47**" (founded 1947) and Dutch club AZ's real historical name "AZ **'67**" (founded 1967) both require this byte to complete a real, checkable year. **Resolves the previously-open San Martín (SJ) blocker** (§8.3) — see §10.4. |
| `0x47` | `&` | 3 real English club official names, each cited twice: Millwall ("Football & Athletic Company"), Brighton & Hove Albion, Rushden & Diamonds. |
| `0x39` | `X` (uppercase) | 5+ citations: Neuchâtel Xamax FC (real Swiss club), Basque surnames Goikoetxea/Etxeberria (×2), "Alexis", "Felix". Completes the lower/upper `+0x20` pattern already established for every other letter (lowercase `x=0x19` already confirmed). |
| `0x83` | `â` | 1 citation, same word (Châteauroux, a real French club/city) referenced twice. |
| `0xA6` | `Ç` (uppercase) | 2 citations: real Brazilian footballer Flávio Conceição, and a player record's own short_name "Bjeliça" (lowercase, already correctly decoding) matching its own long_name's uppercase form — self-consistent. |
| `0xA8` | `É` (uppercase) | 6+ citations, all real Portuguese first names or a real player's documented name: André (Alves da) Cruz, Sérgio (×2), Rogério, Eugénio. |
| `0xB0` | `Ñ` (uppercase) | 6 citations, all real Spanish football names/words: Cañizares, (de la) Peña, Iñigo, Ureña, Cañas, Muñiz. |
| `0xBD` | `Ü` (uppercase) | 4 citations, each self-consistent with the same record's own already-correct lowercase "Müller" short_name: Uwe Müller, Krisztian Müller, Martin Müller, and a Brazilian player nicknamed "Müller". |

`0xD5` (see §7.4) was **not** resolved by this pass — if anything, a new
occurrence ("Stark[0xD5]s Park", Raith Rovers' real stadium "Stark's
Park", which would need `0xD5="'"`) directly **contradicts** the original
"plausibly é" hypothesis from the `.PKF` corpus, reinforcing rather than
resolving the ambiguity. Left deliberately unresolved, per this file's own
standing rule against forcing a decision between two mutually exclusive
single-citation guesses.

### 10.4 `0x56` resolved: San Martín (SJ) now parses end-to-end

With `0x56='7'` added to `fixtures/charmap/confirmed_real_map_v2.txt`,
re-running `examples/dump_container.rs` against the real
`EQ003003.PKF` finds **all 55 of 55** real domestic team records parsing
end-to-end (up from 54/55 in §8.3) — the San Martín (San Juan) team-info
fields (including the previously-blocked stadium name, "25 de Septiembre")
now decode cleanly, and (per §8.4's player-roster support) it also gets a
fully-parsed player roster. `crates/pcf-codec/src/container.rs` gained a
new real-fixture-aware test (`parses_real_san_martin_sj_record_if_present`,
mirroring `parses_real_river_record_from_the_users_own_pkf_if_present`'s
"skip gracefully if absent, assert real facts if present" pattern) that
specifically exercises this: it looks up the record whose `short_name`
decodes to `"San Martín (SJ)"` (only resolvable now that the charmap
covers `0x56`) and asserts the stadium name decodes cleanly (with zero
`charmap_unknown_byte` errors) to `"27 de Septiembre"`. Note: unlike
River's independently-fact-checked stadium name, this specific date was
NOT separately verified against a known real-world fact for this specific
club (the `0x56='7'` byte itself is confirmed from unrelated real names —
MyPa, AZ — per §10.3, not from this club's own history) — the assertion
here is about the charmap gap being closed and the record parsing
end-to-end, not an independent historical fact-check of this one date the
way §6.2's five River facts were.

## 11. UPDATE: Vélez Sarsfield real-world bug report — coach/roster search-order bug fixed, budget still unconfirmed

The user tried the real running app on their own real club, Vélez
Sarsfield, and reported 4 concrete bugs (budget always `0`, only 5 players
in the squad, one player with jersey number `0`, and a garbled coach name).
Investigated with a new throwaway tool, `investigate_velez.rs` (see the
tools table below), which isolates Vélez's real record and instruments the
coach-marker and player-marker candidate scans directly against the real
file.

### 11.1 Root cause for the roster undercount (#2) and the garbled coach (#4) — CONFIRMED, high confidence, FIXED

Both bugs share one root cause. `find_coach_stub` (used by
`parse_team_record`) originally scanned for the first `02 02` byte pair
anywhere in the **entire remainder of the record**, including all player
data — this happened to be safe for River only because River's real coach
marker is the very first `02 02` occurrence in the whole blob (§6.5).
Vélez's record has **no locatable coach chain at all** before its player
roster begins (unlike River, `find_coach_stub` finds no plausible `02 02`
match in the true pre-roster region — the coach chain is either genuinely
absent for this team's record or encoded somewhere this heuristic can't
find, same "thinner records for smaller/other clubs" pattern already noted
in §8.2 for missing coach chains generally). With no early match, the old
unbounded scan kept going deep into player data and found a coincidental
`02 02` byte pair sitting inside one player's own free-text biography field
— the two "strings" it decoded (`"dor lo convocó "` / `"ra integrar el
pl"`) are ordinary Spanish sentence fragments (roughly "...manager called
him up..." / "...to join the squad..."), not a real person's name, but they
still passed the coach-shape validity check (non-empty, plausible length,
fully charmap-decodable). This consumed those bytes as a fake "coach" and
started the player-roster search from that point onward — silently
discarding every real player that came *before* it in the file, including
Vélez's real, legendary goalkeeper **José Luis CHILAVERT** (a 1998-era
world-famous Paraguayan international) and **Ariel DE LA FUENTE**. Whatever
real players happened to remain *after* the false match (5 of them) still
parsed correctly (their raw bytes are genuinely real player data, just
starting from the wrong point in the roster) — this is exactly why the bug
manifested as "5 real-looking players", not garbage/a crash.

**Fix**: `parse_team_record` now locates the player roster's real start
FIRST (via `find_first_player_record`, which already requires the *entire*
downstream player-record structure to parse and every enum-shaped byte to
land in its confirmed real-data range — a much stronger, more reliable
anchor than `find_coach_stub`'s "first `02 02` match" heuristic), and only
searches for a coach marker in the bytes strictly *before* that point. A
coincidental `02 02` inside player data can no longer be mistaken for the
coach chain, because player data is never even in the coach search's scope
anymore.

**Verified against the real file**: Vélez's `players.len()` goes from **5
(wrong) to 20 (real squad, including Chilavert, De la Fuente, and 18 more —
all real, era-correct Vélez Sarsfield 1998-99 players by name, e.g.
Bassedas, Cubero, Sotomayor, Cardozo, Méndez)**, and `coach` goes from
`Some(ContainerCoachStub { short_name: "dor lo convocó ", ... })` (garbage)
to `None` (honest — this team's record genuinely has no locatable coach
chain before its roster, same as the great majority of the file's other 53
non-River teams — see §11.4 below). River's own coach/roster (`Ramón
Díaz`, 27 players) and San Martín (SJ)'s roster are unaffected — re-running
both existing real-fixture tests plus a new
`parses_real_velez_record_from_the_users_own_pkf_if_present` test (asserts
`players.len() > 10`, Chilavert and De la Fuente present by name,
`coach.is_none()`) all pass.

### 11.2 Jersey number `0` (#3) — investigated, NOT a bug, same root cause but not a new defect

Confirmed it's the *same* root cause as #2 (the misaligned roster start
made every downstream field, including `number`, come from the wrong
byte offset for the 5 players it did find) — but after the §11.1 fix, one
of Vélez's *correctly-aligned, correctly-parsed* real players (Favio Héctor
ZARATE) still reads `number == 0`. A file-wide sanity pass (all 55 teams,
via the same `investigate_velez.rs` tool) found `number == 0` recurring
across the **great majority of teams' rosters** — e.g. Banfield (5 of 18
players), Arsenal (29 of 30 players!), Dep. Morón (8 of 25) — not an
isolated anomaly. This is far too widespread and inconsistent with a
byte-alignment bug (which would show up as garbage names/attrs alongside
the bad number, and it doesn't here — every other field for these
`number == 0` players decodes to a plausible real name/attrs). Most likely
explanation: `0` is this container's real sentinel for "no shirt number
assigned yet" (reserve/non-first-team squad members, common in a database
covering an entire division's full registered squads, not just each
club's nominal best XI) — plausible, not confirmed against an independent
real fact, but clearly not a parser defect. **Left as-is** (not "fixed",
because there's nothing to fix): `ContainerPlayerRecord::number` is kept
raw exactly as before.

### 11.3 Budget (#1) — investigated, still honestly UNCONFIRMED (not wired in)

§6.3's original "budget = u24 LE right after `president`" was a hypothesis
by analogy with the override format's field order, never independently
checked against real bytes. Checking it now, across all 55 real domestic
records (`investigate_velez.rs`'s "budget-offset check" pass): the 2-3
byte value at that exact position is followed a few zero-padding bytes
later by a length-prefixed string that decodes to real, historically
accurate **sponsor names** — `"QUILMES"` (River, Boca, Vélez — Quilmes beer
was Argentina's dominant football sponsor in this era), `"CABLEVISION"`
(San Lorenzo — a real Argentine cable-TV company), `"MULTICANAL"` (Racing —
another real Argentine cable-TV company), `"NO TIENE"` (Independiente —
literally "doesn't have [a sponsor]"). This is a genuine new finding (a
sponsor-name block, not documented before), but it argues AGAINST the
"budget" reading for the preceding numeric value, not for it:

- River and Boca (Argentina's two biggest, most historically dominant
  clubs) read the exact same value (2025); San Lorenzo and Independiente
  (both separately, also top-5-tier clubs of the era) read a different but
  also exactly-shared value (1860). A real per-team currency budget
  shouldn't tie exactly between two different clubs unless by
  coincidence — shared values across peer-tier clubs looks more like a
  categorical "tier"/"reputation" rating than a literal peso figure.
- Independiente's value (1860) is nonzero despite that same record's
  sponsor field explicitly reading `"NO TIENE"` (no sponsor) — if the
  number were sponsorship income specifically, an unsponsored club
  reading a nonzero value tied to a *sponsored* club (San Lorenzo) would
  be inconsistent. This decorrelation from the adjacent sponsor field
  argues the number isn't "sponsor deal value" either.
- Roughly half of the 55 teams (including several genuine, well-known
  1998-99 top-flight clubs, e.g. Dep. Español, Arsenal, San Martín (Tuc))
  read exactly `0` — plausible for a "not populated for this record"
  default, but also exactly the same value the honest hardcoded fallback
  already returns, so wiring in "sometimes-0, sometimes-tied-across-clubs"
  data wouldn't obviously be an improvement.

No independently-checkable real-world fact (unlike River's stadium
capacity, founding year, or president's name) exists to confirm either
reading (literal budget vs. tier/reputation rating vs. something else
entirely) for this specific numeric field. Per this project's own
charmap-provenance rigor standard (never force an unconfirmed guess just
to fill in a number), **`ContainerTeamRecord` does NOT gain a `budget`
field from this pass**, and `container_bridge.rs`'s `Team.budget` stays
honestly `0` — see its own updated comment for the full reasoning inline.
The UI-facing currency label was still fixed independently of this
(`ui/src/routes/TeamScreen.svelte`'s "Budget (pesetas)" label was wrong
regardless of the parsing question — pesetas are Spanish, not Argentine;
changed to "Budget (Pesos Argentinos)").

### 11.4 Coach chain absence is now understood to be the norm, not the exception

With the §11.1 fix, re-running the file-wide sanity pass shows **only
River has a locatable coach chain**; all other 54 teams (including Vélez)
now honestly report `coach: None`. This is consistent with — and now much
better explained than — the earlier "successfully-parsed teams show
`coach: (none found)` — expected" note from §8.2: it's not that the
coach-marker heuristic is flaky per-team, it's that a locatable `02 02`
coach-chain marker genuinely appears to exist (at least in the region
before the roster) for only a small minority of records in this file.

## Investigation tools (all under `crates/pcf-codec/examples/`)

| Tool | Purpose |
|---|---|
| `investigate_pkf.rs` | First-pass investigator: finds banner occurrences (both spellings), delta stats between them, hex-dumps the pre-first-banner region and the 32 bytes after the first banner. Usage: `cargo run -p pcf-codec --example investigate_pkf -- <path.pkf>` |
| `investigate_pkf_dir.rs` | Second-pass investigator: finds the 13-byte directory signature, groups occurrences into contiguous blocks, verifies offset/length fields against real banner positions, and can dump+decode any chosen block's record bodies. Usage: `cargo run -p pcf-codec --example investigate_pkf_dir -- <path.pkf> [block_index]` |
| `investigate_domestic_team.rs` | Third-pass investigator (§6): parses the confirmed team-info fields (through `president`) out of a real domestic team record, plus heuristic scans for the coach-chain marker (`02 02`) and player-record markers (`0x01`). Uses the 37-pair confirmed charmap plus 13 newly-inferred pairs (see §6.8, kept local to this tool; reconciled against §7's larger corpus). Usage: `cargo run -p pcf-codec --example investigate_domestic_team -- [path-to-blob]` (defaults to `fixtures/golden/real_river_9001_container_blob.raw`). |
| `dump_stub_table.rs` | Fourth-pass investigator (§7): dumps ALL 15 stub-table blocks' `short_name`/`stadium_name` fields, lossy-decoded (unmapped bytes render as `[XX]`), across all 473 records — the corpus used to expand the charmap from 37 to 77 confirmed pairs. Usage: `cargo run -p pcf-codec --example dump_stub_table -- <path.pkf> [charmap-path]` (charmap path defaults to `fixtures/charmap/confirmed_real_map.txt`; point it at `confirmed_real_map_v2.txt` to see the improved decode). |
| `dump_container.rs` | **Uses the real production parser**, not investigation-only code: calls `pcf_codec::container::parse_pkf_container_verbose` (see `crates/pcf-codec/src/container.rs`, the new §6-implementing module) on a whole `.PKF` file and prints a summary table (team short name, capacity, founded, president, coach short name) for every domestic team record found, plus a parsed/failed count. Usage: `cargo run -p pcf-codec --example dump_container -- <path.pkf> [charmap-path]` (charmap path defaults to `fixtures/charmap/confirmed_real_map_v2.txt`). Note (§8 UPDATE): its 6-byte domestic-header match is too strict and currently only finds River; not modified by this investigation pass (owned by a parallel effort). |
| `investigate_player_layout.rs` | Fifth-pass investigator (§6.6-§6.7, new): walks player records from a given marker offset using the override `read_player` field order (corrected so `short_name`/`long_name` come right after `number`, before `slot`/`origin`/`roles`/etc.), auto-detecting the variable-length gap before `short_name`'s length prefix. Confirms all 27 River players end-to-end. Usage: `cargo run -p pcf-codec --example investigate_player_layout -- [path-to-blob] [start-offset] [max-records]` (defaults: golden blob, offset 1238, 6 records). |
| `investigate_tactics_block.rs` | Sixth-pass investigator (§6.3, new): dumps and annotates the region between `Team.president` and the coach marker with exact cursor arithmetic against the override field order, and scans for candidate length prefixes. Usage: `cargo run -p pcf-codec --example investigate_tactics_block -- [path-to-blob]`. |
| `enumerate_domestic_teams.rs` | Seventh-pass investigator (§9, new): scans a whole `.PKF` file for the corrected 4-byte domestic-record signature (`0D 02 00 00` at banner+38) and decodes every record's `short_name`. Found 55 domestic records where the naive 6-byte signature found only 1 (see §8 UPDATE). Usage: `cargo run -p pcf-codec --example enumerate_domestic_teams -- <path-to-EQ003003.PKF> [charmap-path]`. |
| `build_synthetic_golden.rs` | Unrelated to PKF investigation — regenerates `fixtures/golden/synthetic_minimal.dbc` from the in-code synthetic `Dbc` builder (Agent A's TDD fixture, not real data). |
| `investigate_editor_pm_dbc.rs` | Eighth-pass investigator (§10, new): lossy-decodes `short_name`/`stadium_name`/`long_name` (plus, optionally, a raw unstructured tail dump) from a directory of real `EQ97####.DBC` override files from an EXTERNAL corpus (the community "EDITOR-PM9798" tool — never bundled with or copied into this repo). Usage: `cargo run -p pcf-codec --example investigate_editor_pm_dbc -- <dir-with-EQ97-files> [charmap-path]`. |
| `investigate_editor_pm_dbc_full.rs` | Ninth-pass investigator (§10.2, new): attempts a full structural walk of one of those external DBC files using this project's own `dbc.rs` field layout, lossily. Surfaced a genuine negative finding (§10.2: this corpus's team-info layout differs from `dbc.rs`'s current assumptions) rather than being the main source of new charmap evidence. Usage: `cargo run -p pcf-codec --example investigate_editor_pm_dbc_full -- <dir-with-EQ97-files> [charmap-path]`. |
| `investigate_velez.rs` | Tenth-pass investigator (§11, new): isolates Vélez Sarsfield's real record and instruments the coach-marker/player-marker candidate scans byte-by-byte, plus a file-wide "budget-offset" dump (bytes right after `president` for every team) and a file-wide `number == 0` sanity pass. Root-caused §11.1's coach/roster search-order bug and §11.3's sponsor-name discovery. Usage: `cargo run -p pcf-codec --example investigate_velez -- <path-to-EQ003003.PKF> [charmap-path]`. |

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
