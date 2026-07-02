# Apertura team-pointer table

## Real reference data (landed)

- `team_pointers.csv` — the full carky12 team-pointer catalog (pointer →
  name/country) for every PC Fútbol 6.x edition, transcribed from
  `jandro996/EditorPCFutbol6`'s `Manuales/Punteros Equipos.pdf`. Confirms
  Boca = `9013` (matches `pointers::team_filename`'s worked example) and
  the whole `9000`-`9061` Argentina block.
- `country_pointers.csv` — the country-code table (byte → country name)
  from the same repo's `Manuales/Punteros Paises.pdf`. Confirms Argentina
  = `0x03` (the mocks previously used a placeholder `1`, i.e. Albania).

Both are community reference metadata (numeric pointer/code tables), not
extracted Dinamic binary asset data — see "Source" below.

**Still missing:** the `eq003003_pointers.csv`-style *load-order* mapping
described below — `team_pointers.csv` gives the static pointer→name
catalog, not which position each team occupies when a specific
`EQ003003.PKF` is loaded (that's what `resolve_player_block` actually
needs to recover a DBC's 50-player block).

## What goes here

Whatever file(s) let us reconstruct a `pcf_model::TeamIndex` — i.e., for
every team in `EQ003003.PKF`, its **load order**, decimal **pointer**, and
**short name** + **country code**. This is the input `load_pkf` (PLAN.md
§4.3) parses, and it's what
`pcf_model::pointers::resolve_player_block` (`crates/pcf-model/src/pointers.rs`)
needs to recover a DBC's correct 50-player block on open instead of always
falling back to `1..=50`.

Suggested format (a placeholder convention until whoever consumes it first —
Agent A's PKF reader, or Agent D's `load_pkf` — pins the real one): a CSV
named e.g. `eq003003_pointers.csv` with a header row:

```csv
load_order,pointer,short_name,country
1,9013,BOCA,1
2,9014,RIVER,1
...
```

`load_order` is 1-indexed, matching `player_block_for_load_order` (team 1 →
player pointers `1..=50`, team 2 → `51..=100`, etc., PLAN.md §4.2).

## Source

The community pointer-table docs (carky12/EditorPCFutbol6, pcfutbolmania.com
— PLAN.md Appendix C), or a direct dump of `EQ003003.PKF` once Agent A's PKF
reader exists.

Unlike `fixtures/golden/*.dbc` and `fixtures/manager/manager.exe`, this
table isn't extracted Dinamic binary asset data — it's a community-derived
lookup table (pointer/name/country), so it's fine to commit once you have a
real one. If you'd rather not, `.gitignore` a specific filename yourself
before committing.
