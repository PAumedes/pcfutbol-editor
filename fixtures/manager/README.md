# Manager.exe fixture

**Empty right now.** See `fixtures/README.md` item 4 for context.

## What goes here

`manager.exe` — an unmodified, **real Apertura 98/99** manager executable,
size ≈ **2397 KB** (the sanity guard from PLAN.md Appendix B). Agent C's
`crates/pcf-manager` needs this to:

- confirm the Y2K-fix byte sequence (`6C070000760881FDD0070000` →
  `01000000760881FDFFFF0000`) actually appears at the expected offset in
  *this* edition's binary,
- locate this edition's season-start-year (and, if ever unblocked,
  calendar/competition) offsets — PLAN.md Risks §9.4 explicitly defers
  calendar editing until this is confirmed against a real Apertura
  `manager.exe`,
- exercise `verify()` (patched vs. unpatched detection) and the
  backup/restore path against a real file instead of a hand-crafted one.

Keep a copy aside before letting any patcher touch it — `pcf-manager`
always writes a `.bak` first (PLAN.md §8 Safety), but a second, out-of-tree
copy costs nothing and de-risks fixture experiments.

This directory is `.gitignore`d (`manager.exe` and `*.bak` are copyrighted
binary game data, not repo content) — besides this README, nothing here is
committed.
