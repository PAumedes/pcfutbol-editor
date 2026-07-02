# Golden DBC fixtures

**Empty right now except for whatever synthetic placeholders Agent A drops
here for TDD.** See `fixtures/README.md` item 1 for what's needed and why.

## What goes here

Real, unmodified `*.dbc` team files copied from a real Apertura 98/99
install (`DBDAT\EQ003003\...DBC` overrides, or per-team extracts from the
base `EQ003003.PKF` container). These are the byte-fidelity oracle: the
round-trip harness in `tests/tests/round_trip.rs` asserts
`Dbc::write(Dbc::read(bytes)) == bytes` for every file here.

Aim for variety, not just volume:
- at least one national/playable team (has coach + player records),
- at least one foreign team (`is_foreign = true`; no coach/players — PLAN.md
  Appendix A league-flag note),
- any file-version variants you can find (PLAN.md Risks §9.3: standing
  capacity / stadium latitude bytes are present in some versions and not
  others — the round-trip gate is what actually proves Agent A's codec
  handles this instead of hardcoding one shape).

## Naming convention (important for the harness)

The round-trip harness classifies files by name so its report distinguishes
real fixtures from placeholders:

- any filename whose lowercased form contains `synthetic` is treated as a
  **synthetic placeholder**, not a real fixture — e.g. `synthetic_team.dbc`.
  Agent A may drop such files here for their own codec TDD; that's fine and
  expected, they just don't move the "real fixtures validated" count.
- everything else ending in `.dbc`/`.DBC` is treated as **real** and is
  expected to be a byte-identical round-trip.

This directory (besides synthetic placeholders and this README) is
`.gitignore`d — see the root `.gitignore` and `fixtures/README.md` for why.
