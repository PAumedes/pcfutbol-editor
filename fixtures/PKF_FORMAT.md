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
| Real domestic (Argentina) team records | **Located, one likely-River record extracted.** Team-info fields (short_name through president) **confirmed high-confidence** via 5 independently-checkable real-world facts (stadium, legal name, capacity, founding year, president). Coach chain start **confirmed** (real coach name "Ramón Díaz" decodes exactly). First player record found (medium-high confidence). Tactics boundary and full player roster framing still open. See §6. |
| Character map | **77 confirmed byte↔glyph pairs.** The original 37 (`fixtures/charmap/confirmed_real_map.txt`, from the manual's hex-editing appendix) plus **40 new ones** (`fixtures/charmap/confirmed_real_map_v2.txt`), derived by decoding all 473 stub-table records (§3) and cross-referencing every gap against real football names — see §7. This supersedes and reconciles §6.8's 13 provisional single-fact inferences from the domestic-team investigation (11 of 13 match exactly; 1 byte, `0x50`, is corrected — see §7.3). |
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

### 6.3 What comes after `president` — team stats / palmarés-shaped region (hypothesized, medium confidence)

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

### 6.6 First player record — medium-high confidence

Immediately after the coach chain's career-default pattern (offset 1504)
and a few more unresolved bytes, a marker byte `0x01` appears at absolute
blob offset **1238** (`0x4D6`):

```
offset 1238   1   01                 <- marker (matches override PLAYER_MARKER)
offset 1239   2   00 19              <- u16 LE = 6400 (pointer? unusually large)
offset 1241   1   01                 <- number (dorsal) = 1
offset 1242+  ... -> short_name "Saccone", long_name "Alejandro SACCONE"
```

`short_name`/`long_name` mixed-case convention (`"Saccone"` /
`"Alejandro SACCONE"`, surname capitalized) matches the coach record's
style exactly (`"Ramón Díaz"` / `"Ramón Angel DIAZ"`), which is corroborating
evidence this is a genuine field boundary and not a coincidence, even
though the pointer value (6400) doesn't obviously match a plausible
`player_block_for_load_order` range and there are a few unaccounted bytes
between `number` and the first string (not yet resolved — this is a gap
between here and the override format's expected marker→pointer→number→
short_name sequence, similar in spirit to the "extra bytes" pattern in
§6.4). Confirms 1 more inference (`0x0B='j'`).

**Confidence: medium-high** on "this is where player records start and
the marker/name-field shape roughly matches override format"; **low** on
the exact byte-for-byte layout of the fixed fields between `number` and
`short_name` (roles/nationality/skin/hair/demarcation/birth/etc. per
`pcf_model::Player` don't obviously fit in the ~4 bytes actually observed
there) — this container's player *header* is evidently shorter or
differently laid out than the override `.dbc` format's, even though the
marker byte and the two name strings' shape/style match.

### 6.7 Player count estimate — low-medium confidence

`investigate_domestic_team.rs`'s player-marker heuristic (`0x01` + u16
"pointer" in `1..=2000` + a plausible dorsal `1..=40`) finds **39
candidates** across the blob, but this scan:

- **Misses the actual first player** (§6.6's hit at offset 1238 has
  pointer=6400, outside the `1..=2000` heuristic window) — confirms the
  heuristic under-counts.
- Includes an obvious false-positive cluster at offsets `0xD9`–`0xE4`
  (10 hits within 12 bytes, deltas of 1–2) that are almost certainly
  incidental `0x01` bytes inside the still-unresolved stats/history region
  from §6.3, not real player boundaries.
- Its remaining ~24 "spaced out" hits (offsets 6621 through 90714) have
  deltas mostly in the 3,000–6,000 byte range, consistent with sizeable
  per-player records (biography-heavy: 8+ free-text fields per
  `pcf_model::Player`) but not confirmed record boundaries individually.

Given the confirmed first player at offset ~1238 and the blob's total
length of 92,956 bytes, an average per-player size in the observed
3,000–6,000 byte range would suggest something in the range of **roughly
20–28 players** — consistent with, if perhaps slightly on the low side of,
a realistic 1998 Argentine top-flight squad (typically 25–35 registered
players). **Not independently confirmed** — no attempt was made in this
pass to walk every candidate boundary and verify a coherent player record
between each pair (attributes block, roles array, etc.); this is a rough
order-of-magnitude estimate only.

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

## Investigation tools (all under `crates/pcf-codec/examples/`)

| Tool | Purpose |
|---|---|
| `investigate_pkf.rs` | First-pass investigator: finds banner occurrences (both spellings), delta stats between them, hex-dumps the pre-first-banner region and the 32 bytes after the first banner. Usage: `cargo run -p pcf-codec --example investigate_pkf -- <path.pkf>` |
| `investigate_pkf_dir.rs` | Second-pass investigator: finds the 13-byte directory signature, groups occurrences into contiguous blocks, verifies offset/length fields against real banner positions, and can dump+decode any chosen block's record bodies. Usage: `cargo run -p pcf-codec --example investigate_pkf_dir -- <path.pkf> [block_index]` |
| `investigate_domestic_team.rs` | Third-pass investigator (§6): parses the confirmed team-info fields (through `president`) out of a real domestic team record, plus heuristic scans for the coach-chain marker (`02 02`) and player-record markers (`0x01`). Uses the 37-pair confirmed charmap plus 13 newly-inferred pairs (see §6.8, kept local to this tool; reconciled against §7's larger corpus). Usage: `cargo run -p pcf-codec --example investigate_domestic_team -- [path-to-blob]` (defaults to `fixtures/golden/real_river_9001_container_blob.raw`). |
| `dump_stub_table.rs` | Fourth-pass investigator (§7): dumps ALL 15 stub-table blocks' `short_name`/`stadium_name` fields, lossy-decoded (unmapped bytes render as `[XX]`), across all 473 records — the corpus used to expand the charmap from 37 to 77 confirmed pairs. Usage: `cargo run -p pcf-codec --example dump_stub_table -- <path.pkf> [charmap-path]` (charmap path defaults to `fixtures/charmap/confirmed_real_map.txt`; point it at `confirmed_real_map_v2.txt` to see the improved decode). |
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
