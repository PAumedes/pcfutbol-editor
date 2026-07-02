# Fixtures — what you need to supply

Everything under `fixtures/` is **reference/test data**, not shipped product.
Per PLAN.md §1 ("own-copy only"), this project ships **no proprietary Dinamic
data**. The directories below are checked into the repo as *structure only*;
the real files must come from your own legally-owned copy of PC Apertura
98/99 and are `.gitignore`d so they never get committed (see the "Real game
data" block near the top of the root `.gitignore`).

Until the real files land, `cargo test --workspace` still passes — the
round-trip harness in `tests/` is written to report "0 real fixtures found"
rather than fail the build (see `tests/tests/round_trip.rs`). Dropping real
files here is what turns that report into the actual byte-fidelity gate from
PLAN.md §6/§7.

Some subfolders already contain **SYNTHETIC placeholder** files dropped by
Agent A/B/C for their own TDD — those are clearly marked (filenames contain
`synthetic`, and/or a README in the same folder says so in bold). They are
intentionally invented, not real, and don't count toward "real fixtures" in
any report. Leave them alone; they're not yours to remove and they're not a
substitute for the real files below.

## Checklist

| # | Path | What it is | Used by |
|---|---|---|---|
| 1 | `fixtures/golden/*.dbc` | Unmodified team DBC files copied out of a real Apertura install's `DBDAT\EQ003003\` (or the base container `EQ003003.PKF` extracted per-team). At least a handful, covering: a national team (has coach + players), a foreign team (`is_foreign = true`, no coach/players), and if possible one of each file-version variant you can find (see PLAN.md Appendix A "version-variant fields"). | `tests/tests/round_trip.rs` (byte-fidelity gate), Agent A's own acceptance tests in `crates/pcf-codec` |
| 2 | `fixtures/charmap/map.txt` | The real byte↔char substitution table for Apertura 98/99 text encoding (PLAN.md Appendix A, the "Real Madrid C.F." proof). Source from the community (carky12/EditorPCFutbol6, pcfutbolmania.com — see Appendix C) or extract from the game's own resources. Format: whatever `CharMap::load`/`CharMap::parse` in `crates/pcf-codec` expects — as of this writing that's `HH\tC` per line (two hex digits, a tab, one character), matching the placeholder in `fixtures/charmap/synthetic_map.txt`; check that file's README before assuming the format hasn't changed. | `pcf-codec::CharMap`, decoding/encoding every string field in every record |
| 3 | `fixtures/pointers/<table>` | The Apertura team-pointer table: for every team in `EQ003003.PKF`, its load-order position, decimal pointer, short name and country code — i.e. enough to reconstruct a `pcf_model::TeamIndex` (see `crates/pcf-model/src/pointers.rs`). A simple CSV works: `load_order,pointer,short_name,country` (header row, one team per line, load-order 1-indexed). Source from the same community pointer-table docs, or by dumping the real PKF once Agent A's PKF reader exists. | `pcf_model::pointers::resolve_player_block` (recovering a DBC's correct 50-player block on open), `load_pkf` IPC command |
| 4 | `fixtures/manager/manager.exe` | An unmodified Apertura 98/99 `manager.exe`, size ≈ **2397 KB** (PLAN.md Appendix B). Used to confirm the Y2K-fix offset and locate the season-start-year offset for *this* edition before Agent C enables calendar editing. | `crates/pcf-manager` acceptance tests (`verify()`, `patch_y2k()`, offset confirmation) |

None of the four are present yet beyond the agents' own synthetic
placeholders. Supplying them (from your own game install) is what unblocks:

- the byte-fidelity round-trip gate (PLAN.md §1, §6 Agent A, §7 M1),
- real string decoding instead of the invented 11-glyph subset,
- correct player-block recovery instead of always falling back to `1..=50`,
- confirmed (not just Y2K-known) manager-patch offsets.

## Why these aren't committed

`fixtures/golden/*.dbc` and `fixtures/manager/manager.exe` are `.gitignore`d
— they're copyrighted game data extracted from your own purchase, so keep
them local. `fixtures/pointers/*` is **not** blanket-ignored: a pointer
table is a community-derived lookup (pointer/name/country), not extracted
Dinamic binary data, so it's fine to commit once you have a real one (see
`fixtures/pointers/README.md` if you'd rather keep it local anyway). Agents'
explicitly-marked synthetic placeholders stay tracked in git for TDD.
