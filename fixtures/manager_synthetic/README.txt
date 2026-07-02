SYNTHETIC FIXTURE — NOT REAL GAME DATA
=======================================

PLACEHOLDER_manager_98_99.bin in this directory is a hand-constructed,
synthetic stand-in for an Apertura 98/99 `manager.exe`. It is generated (and
overwritten) by `cargo test -p pcf-manager` — see the
`synthetic_fixture_is_materialized_on_disk_and_reads_as_unpatched` test in
`crates/pcf-manager/src/lib.rs`.

It is NOT a real executable:
- It opens with an ASCII banner ("PCF-MANAGER-SYNTHETIC-PLACEHOLDER ...")
  instead of an `MZ` PE header, precisely so it can never be confused with a
  real Windows binary.
- The rest of the file is filler bytes (0xAA), sized to ~2397 KB to match
  the documented Apertura manager.exe size (PLAN.md Appendix B), with the
  known pre-patch Y2K byte pattern spliced in at a fixed offset so
  `pcf-manager`'s patch/verify/backup logic has something real to exercise.

Why it's here (not in fixtures/golden, fixtures/charmap, or
fixtures/pointers): Agent C (`crates/pcf-manager`) needs a fixture to TDD
`patch_y2k()` / `verify()` / backup+restore against, but no real Apertura
`manager.exe` has been supplied by the user yet. This lets Agent C's test
suite be red/green without fabricating a fake "real" binary and without
touching another agent's fixtures.

Once a real Apertura 98/99 `manager.exe` (~2397 KB) is supplied, it should
replace this synthetic fixture as the basis for `pcf-manager`'s tests, and
this directory can be retired. It is also required to confirm the
season-start-year / calendar offsets before `set_start_year()` can be
implemented (see the TODO(C) in `crates/pcf-manager/src/lib.rs` and PLAN.md
risk #4).
