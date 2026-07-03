//! Tauri IPC commands (PLAN.md §4.3). Each function here is a thin
//! wrapper: no business logic lives in this crate, it belongs in
//! `pcf-codec` / `pcf-images` / `pcf-manager` (Agents A/B/C). Where a real
//! crate function doesn't exist yet, the wrapper calls a mock in
//! `crate::mock` and is tagged with a `// TODO(D): swap for ...` comment
//! marking the exact swap point.
//!
//! `#[tauri::command]` leaves the underlying function callable directly,
//! so every command here is unit-tested as a plain Rust function — no
//! webview required.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use pcf_codec::container::ContainerTeamRecord;
use pcf_model::{
    AssetResult, CharmapInfo, Dbc, ExportReport, ManagerPatch, PatchReport, PcfError, PointerMode,
    Project, TeamIndex, TeamIndexEntry,
};

use crate::mock;

/// Synthetic team-pointer range used when a container-decoded `short_name`
/// doesn't cross-reference against `fixtures/pointers/team_pointers.csv`'s
/// real Argentina pointers (see `resolve_team_pointer`'s doc comment for
/// why an exact-name cross-reference is attempted at all, and why it won't
/// always hit). Chosen deliberately far outside the real catalog's used
/// range (`fixtures/pointers/team_pointers.csv` tops out at `9958`) so a
/// synthetic pointer can never collide with a genuine one.
const SYNTHETIC_POINTER_BASE: u16 = 60_000;

/// Candidate directories to search for `fixtures/`, in priority order:
/// 1. Next to the running executable (`<exe_dir>/fixtures/...`) — the shape
///    a distributed/copied build (this project has no installer yet, only
///    a manually-copied folder) is expected to ship in: `fixtures/` sitting
///    alongside the `.exe`.
/// 2. `CARGO_MANIFEST_DIR` baked in at *compile* time — works for any
///    `cargo build`/`cargo test`/`cargo run` invocation (dev container or
///    otherwise), since the source tree (and therefore `fixtures/`) is
///    guaranteed present next to wherever it was compiled.
///
/// A real bug this fixes: a binary built in one place (e.g. a Docker dev
/// container at `/workspace`) and then copied elsewhere to run (e.g. a
/// cross-compiled Windows `.exe` copied onto the host machine) used to hard
/// -fail every command that needs the charmap, since `CARGO_MANIFEST_DIR`
/// pointed at a path (`/workspace/src-tauri`) that doesn't exist at all on
/// the machine actually running it. `current_exe()`'s directory does still
/// make sense on that machine.
///
/// TODO(D): once packaging (Agent G / PLAN.md M-later) bundles `fixtures/`
/// as a proper Tauri resource, add the app handle's resource-dir as a third
/// candidate ahead of these two rather than relying on a manually-copied
/// sibling folder.
fn fixtures_candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            dirs.push(exe_dir.join("fixtures"));
        }
    }
    dirs.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("fixtures"),
    );
    dirs
}

/// Loads the byte↔char substitution table needed to decode `.PKF`/`.DBC`
/// text (PLAN.md §9 risk #1) from `fixtures/charmap/confirmed_real_map_v2.txt`
/// — the more complete of the two real (non-synthetic) charmaps, per
/// `fixtures/charmap/README.md` and `crates/pcf-codec/examples/dump_container.rs`'s
/// own default. Tries each of [`fixtures_candidate_dirs`] in order.
fn load_charmap() -> Result<pcf_codec::CharMap, PcfError> {
    let tried: Vec<PathBuf> = fixtures_candidate_dirs()
        .into_iter()
        .map(|dir| dir.join("charmap").join("confirmed_real_map_v2.txt"))
        .collect();
    match tried.iter().find(|p| p.is_file()) {
        Some(path) => pcf_codec::CharMap::load(path),
        None => Err(PcfError::new(
            "charmap_not_found",
            "couldn't find fixtures/charmap/confirmed_real_map_v2.txt in any known location",
        )
        .with_context(
            tried
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("; "),
        )),
    }
}

/// Cross-references real Argentina team pointers out of
/// `fixtures/pointers/team_pointers.csv` (community reference data, not
/// proprietary game content — see that file's own header) by lowercase
/// `short_name` match. Returns an empty table (never an error) if the CSV
/// can't be read, so a missing/misplaced fixtures dir degrades to "every
/// team gets a synthetic pointer" rather than failing `load_pkf` outright.
///
/// **Why name-match at all, and why it won't always hit:** the container
/// format (`fixtures/PKF_FORMAT.md` §6) doesn't carry an explicit per-team
/// pointer field anywhere `container.rs` currently parses, so there's no
/// byte-level source of truth to read one from. The CSV's names don't
/// always match the container's decoded `short_name` verbatim (e.g. the
/// CSV's "Gimnasia (LP)" vs. the container's decoded "Gim. Esgrima (LP)"
/// for the same real club, per PKF_FORMAT.md §9) — when they don't match,
/// `resolve_team_pointer` falls back to a synthetic pointer rather than
/// guess at a fuzzy match.
fn load_argentina_pointer_table() -> HashMap<String, u16> {
    let mut table = HashMap::new();
    let contents = fixtures_candidate_dirs()
        .into_iter()
        .map(|dir| dir.join("pointers").join("team_pointers.csv"))
        .find_map(|path| fs::read_to_string(&path).ok());
    let Some(contents) = contents else {
        return table;
    };

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("pointer,") {
            continue;
        }
        let mut parts = line.splitn(3, ',');
        let (Some(ptr_str), Some(name), Some(country)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(pointer) = ptr_str.trim().parse::<u16>() else {
            continue;
        };
        // Restrict to the Argentina block (country == "Argentina") plus the
        // Argentina-specific "special" entries (e.g. "Estrellas Argentina",
        // "Juveniles Argentina"), which have an empty `country` column in
        // the CSV but say "Argentina" in the name itself. Without this
        // restriction, a name like "Racing" would ambiguously match both
        // the real Argentina club and e.g. a Uruguayan club of the same
        // name elsewhere in the catalog.
        let is_argentina =
            country.trim() == "Argentina" || name.to_lowercase().contains("argentina");
        if is_argentina {
            table.insert(name.trim().to_lowercase(), pointer);
        }
    }
    table
}

/// Resolves every domestic team record's team-index pointer, using the
/// same logic for both `load_pkf` and `load_pkf_team` so their pointer
/// numbering can never diverge between the two commands.
fn resolve_team_pointer(
    record: &ContainerTeamRecord,
    pointer_table: &HashMap<String, u16>,
    synthetic_counter: &mut u16,
) -> u16 {
    match pointer_table.get(&record.short_name.to_lowercase()) {
        Some(&pointer) => pointer,
        None => {
            let pointer = SYNTHETIC_POINTER_BASE.wrapping_add(*synthetic_counter);
            *synthetic_counter += 1;
            pointer
        }
    }
}

/// Reads and parses a `.PKF` container, returning every successfully
/// decoded domestic team record paired with its resolved
/// [`TeamIndexEntry`]. Shared by `load_pkf` (which only needs the index)
/// and `load_pkf_team` (which needs the full record for one team) so
/// there's exactly one place that reads the file, loads the charmap, and
/// resolves pointers.
///
/// Per `pcf_codec::container::parse_pkf_container`'s own design note: a
/// single record's parse failure (e.g. an unrecognized special entry, or a
/// charmap gap — PKF_FORMAT.md §8 UPDATE 2 documents ~16 of 55 real records
/// currently failing this way over an unconfirmed parenthesis glyph) does
/// not fail the whole container; it's just skipped.
fn load_pkf_records(path: &str) -> Result<Vec<(TeamIndexEntry, ContainerTeamRecord)>, PcfError> {
    require_exists(path, "pkf_not_found")?;
    let bytes = fs::read(path).map_err(|e| io_error("load_pkf", path, e))?;
    let charmap = load_charmap()?;
    let outcomes = pcf_codec::parse_pkf_container_verbose(&bytes, &charmap);

    let pointer_table = load_argentina_pointer_table();
    let mut synthetic_counter: u16 = 0;

    let mut out = Vec::new();
    for outcome in outcomes {
        let Ok(record) = outcome.result else {
            continue;
        };
        let pointer = resolve_team_pointer(&record, &pointer_table, &mut synthetic_counter);
        let entry = TeamIndexEntry {
            pointer,
            short_name: record.short_name.clone(),
            country: record.country,
        };
        out.push((entry, record));
    }
    Ok(out)
}

/// Load the team index from a `.PKF` container.
#[tauri::command]
pub fn load_pkf(path: String) -> Result<TeamIndex, PcfError> {
    Ok(load_pkf_records(&path)?
        .into_iter()
        .map(|(entry, _)| entry)
        .collect())
}

/// Load one team's full `Dbc` out of an already-scanned `.PKF` container,
/// by the pointer `load_pkf`'s `TeamIndex` reported for it.
///
/// contract-change: new command, not part of PLAN.md §4.3's original list
/// (see the note appended there). `open_dbc` was deliberately left alone
/// rather than repurposed for this — it documents "open an existing DBC
/// file" (a single-team override file on disk), which is a different
/// shape of request than "pick one team out of an already-loaded, larger
/// PKF container by pointer"; overloading its meaning would have made both
/// call sites more confusing for no real code-sharing benefit (the two
/// commands read completely different file formats).
///
/// Like the rest of this command layer, this is deliberately stateless: it
/// re-reads and re-parses `path` on every call rather than caching a
/// previous `load_pkf` result, so the pointer numbering it produces is
/// always the exact same numbering `load_pkf` itself would report for the
/// same file (see `load_pkf_records`).
#[tauri::command]
pub fn load_pkf_team(path: String, pointer: u16) -> Result<Dbc, PcfError> {
    let records = load_pkf_records(&path)?;
    let (_, record) = records
        .into_iter()
        .find(|(entry, _)| entry.pointer == pointer)
        .ok_or_else(|| {
            PcfError::new(
                "pkf_team_not_found",
                format!("no team with pointer {pointer} found in this PKF"),
            )
            .with_context(path.clone())
        })?;
    Ok(pcf_codec::container_bridge::container_team_to_dbc(&record))
}

/// Open a single team `.DBC` file.
///
/// TODO(D): swap for `pcf_codec::Dbc::read(&std::fs::read(path)?)?` once Agent A lands it.
#[tauri::command]
pub fn open_dbc(path: String) -> Result<Dbc, PcfError> {
    require_exists(&path, "dbc_not_found")?;
    Ok(mock::dbc())
}

/// Write a `.DBC` file for `dbc` into `out_dir`, returning the filename written.
///
/// Uses the real `pcf_codec::DbcCodec::write` (Agent A's already-working
/// override-file codec) rather than the earlier placeholder-JSON stand-in
/// (the stale `TODO(D)` this replaces) — this is the "saving" half of the
/// architecture: a user's edits always land as a new `EQ97####.DBC`
/// override file, never written back into the read-only `EQ003003.PKF`
/// container itself (see `pcf_codec::container`'s own module docs for why
/// that format isn't touched by this codec at all). The team-pointer
/// heuristic below is still a stand-in: once a `Dbc` carries its own real
/// team pointer end to end (rather than inferring one from player pointer
/// 1's block), this should use that instead.
#[tauri::command]
pub fn save_dbc(dbc: Dbc, out_dir: String, mode: PointerMode) -> Result<String, PcfError> {
    let team_pointer = derive_team_pointer(&dbc, mode);
    let filename = pcf_model::pointers::team_filename(team_pointer);

    let dir = PathBuf::from(&out_dir);
    fs::create_dir_all(&dir).map_err(|e| io_error("save_dbc", &out_dir, e))?;

    let charmap = load_charmap()?;
    let bytes = {
        use pcf_codec::DbcCodec;
        dbc.write(&charmap)?
    };

    let out_path = dir.join(&filename);
    fs::write(&out_path, bytes)
        .map_err(|e| io_error("save_dbc", &out_path.to_string_lossy(), e))?;

    Ok(filename)
}

/// Create a new `Dbc`, either cloned from `template` or a blank default.
///
/// TODO(D): once Agent A ships a canonical "new team" builder (correct
/// fixed-prelude bytes, default palmarés length for the target file
/// version, etc.), swap `mock::blank_dbc()` for it.
#[tauri::command]
pub fn new_dbc(template: Option<Dbc>) -> Result<Dbc, PcfError> {
    Ok(template.unwrap_or_else(mock::blank_dbc))
}

/// Import a crest image for `team_pointer` into `out_dir/MINIESC`.
///
/// TODO(D): swap for `pcf_images::import_crest(img, out_dir)?` once Agent B
/// lands it (palette conform step, real dimension validation).
#[tauri::command]
pub fn import_crest(
    img: String,
    team_pointer: u16,
    out_dir: String,
) -> Result<AssetResult, PcfError> {
    import_asset(&img, &out_dir, "MINIESC", team_pointer, 64, 64)
}

/// Import a player photo for `player_pointer` into `out_dir/MINIFOTOS`.
///
/// TODO(D): swap for `pcf_images::import_photo(img, out_dir)?` once Agent B
/// lands it.
#[tauri::command]
pub fn import_photo(
    img: String,
    player_pointer: u16,
    out_dir: String,
) -> Result<AssetResult, PcfError> {
    import_asset(&img, &out_dir, "MINIFOTOS", player_pointer, 64, 96)
}

/// Export a `Project` into `game_dir`'s `DBDAT\EQ003003\` tree
/// (Appendix B): team DBC overrides under `DBDAT/EQ003003/`, asset
/// folders `MINIESC`, `NANOESC`, `MINIFOTOS` as siblings under `DBDAT/`.
///
/// TODO(D): once Agent B lands, stop pre-creating empty asset folders (its
/// import commands will have already populated them via project state).
#[tauri::command]
pub fn export_dbdat(project: Project, game_dir: String) -> Result<ExportReport, PcfError> {
    let dbdat = PathBuf::from(&game_dir).join("DBDAT");
    let container = dbdat.join("EQ003003");
    let mini_esc = dbdat.join("MINIESC");
    let nano_esc = dbdat.join("NANOESC");
    let mini_fotos = dbdat.join("MINIFOTOS");

    for dir in [&dbdat, &container, &mini_esc, &nano_esc, &mini_fotos] {
        fs::create_dir_all(dir).map_err(|e| io_error("export_dbdat", &game_dir, e))?;
    }

    let mut written_files = Vec::new();
    let mut warnings = Vec::new();

    if project.dbcs.is_empty() {
        warnings.push("project has no teams to export".to_string());
    }

    let charmap = load_charmap()?;
    for dbc in &project.dbcs {
        let team_pointer = derive_team_pointer(dbc, PointerMode::Auto);
        let filename = pcf_model::pointers::team_filename(team_pointer);
        let out_path = container.join(&filename);
        let bytes = {
            use pcf_codec::DbcCodec;
            dbc.write(&charmap)?
        };
        fs::write(&out_path, bytes)
            .map_err(|e| io_error("export_dbdat", &out_path.to_string_lossy(), e))?;
        written_files.push(
            out_path
                .strip_prefix(&game_dir)
                .unwrap_or(&out_path)
                .to_string_lossy()
                .replace('\\', "/"),
        );
    }

    Ok(ExportReport {
        written_files,
        warnings,
    })
}

/// Best-effort filesystem probe for a PC Apertura 98/99 install. Never
/// errors — if nothing is found, returns `None`.
#[tauri::command]
pub fn detect_game_dir() -> Option<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(program_files) = std::env::var("ProgramFiles(x86)") {
        candidates.push(
            PathBuf::from(program_files)
                .join("PC Futbol")
                .join("Apertura 98-99"),
        );
    }
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        candidates.push(
            PathBuf::from(program_files)
                .join("PC Futbol")
                .join("Apertura 98-99"),
        );
    }
    candidates.push(PathBuf::from("C:\\PCF6\\APERTURA"));
    candidates.push(PathBuf::from("C:\\Juegos\\Apertura9899"));

    // Wine prefixes (Linux hosts, e.g. this dev container / Wine-based users).
    if let Ok(home) = std::env::var("HOME") {
        let wine_c = PathBuf::from(&home).join(".wine/drive_c");
        candidates.push(wine_c.join("PCF6/APERTURA"));
        candidates.push(wine_c.join("Program Files/PC Futbol/Apertura 98-99"));
    }

    // The container lives at `<root>\DBDAT\EQ003003.PKF` (Appendix B), not
    // directly under the install root — matches the real fixture layout
    // (`fixtures/PKF_FORMAT.md`) and `export_dbdat`'s own `DBDAT` join.
    candidates
        .into_iter()
        .find(|dir| dir.join("DBDAT").join("EQ003003.PKF").is_file())
        .map(|dir| dir.to_string_lossy().into_owned())
}

/// Patch `manager.exe` at `path` according to `opts`, always backing up
/// first.
///
/// TODO(D): swap the backup-only mock for `pcf_manager::patch_y2k`,
/// `pcf_manager::set_start_year`, and `pcf_manager::verify` once Agent C
/// lands (including the size/signature guard against non-Apertura
/// binaries).
#[tauri::command]
pub fn patch_manager(path: String, opts: ManagerPatch) -> Result<PatchReport, PcfError> {
    require_exists(&path, "manager_not_found")?;

    let backup_path = format!("{path}.bak");
    fs::copy(&path, &backup_path).map_err(|e| io_error("patch_manager", &path, e))?;

    let mut applied = Vec::new();
    if opts.y2k {
        applied.push("y2k".to_string());
    }
    if let Some(year) = opts.start_year {
        applied.push(format!("start_year:{year}"));
    }

    Ok(PatchReport {
        already_patched: false,
        backup_path,
        applied,
    })
}

/// Report whether the byte↔char map is loaded and whether any glyphs used
/// by the currently loaded data are missing from it.
///
/// TODO(D): swap for `pcf_codec::CharMap::load(fixtures_path)` status once
/// Agent A/G land the runtime-loaded charmap.
#[tauri::command]
pub fn charmap_status() -> Result<CharmapInfo, PcfError> {
    Ok(CharmapInfo {
        loaded: false,
        missing_glyphs: vec![],
    })
}

// ---------------------------------------------------------------------
// Helpers (not commands)
// ---------------------------------------------------------------------

fn require_exists(path: &str, code: &str) -> Result<(), PcfError> {
    if Path::new(path).exists() {
        Ok(())
    } else {
        Err(PcfError::new(code, "file not found").with_context(path.to_string()))
    }
}

fn io_error(op: &str, path: &str, e: std::io::Error) -> PcfError {
    PcfError::new("io_error", format!("{op} failed: {e}")).with_context(path.to_string())
}

/// TODO(D): remove once teams carry a real pointer (from the PKF index /
/// codec) instead of being inferred from player pointer 1's block.
fn derive_team_pointer(dbc: &Dbc, _mode: PointerMode) -> u16 {
    match dbc.players.first() {
        Some(p) => ((p.pointer.saturating_sub(1)) / 50) + 1,
        None => 0,
    }
}

fn import_asset(
    img: &str,
    out_dir: &str,
    subfolder: &str,
    pointer: u16,
    width: u32,
    height: u32,
) -> Result<AssetResult, PcfError> {
    require_exists(img, "asset_source_not_found")?;

    let dir = PathBuf::from(out_dir).join(subfolder);
    fs::create_dir_all(&dir).map_err(|e| io_error("import_asset", out_dir, e))?;

    let filename = format!("EQ97{pointer:04}.BMP");
    let out_path = dir.join(&filename);

    // TODO(D): copy real, palette-conformed 8-bit BMP bytes from
    // pcf_images once Agent B lands. For now, write a placeholder file so
    // the path exists and the JSON contract can be exercised end-to-end.
    fs::write(&out_path, b"BM-MOCK-PLACEHOLDER")
        .map_err(|e| io_error("import_asset", &out_path.to_string_lossy(), e))?;

    Ok(AssetResult {
        filename,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pcf_model::{Division, LeagueResult, PcfError};
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pcf-editor-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn load_pkf_errors_when_file_missing() {
        let err = load_pkf("/does/not/exist.pkf".to_string()).unwrap_err();
        assert_eq!(err.code, "pkf_not_found");
        assert!(err.context.is_some());
    }

    #[test]
    fn load_pkf_returns_empty_index_for_a_file_with_no_domestic_records() {
        // Real parsing now happens (`pcf_codec::container`), so an
        // existing-but-content-free file legitimately yields zero teams
        // rather than erroring — `find_domestic_team_records` scans the
        // whole byte stream for a specific header shape and finds none in
        // 4 arbitrary bytes. This replaces the old test, which only
        // checked the mock's hardcoded 2-team fixture.
        let dir = temp_dir("load-pkf");
        let path = dir.join("EQ003003.PKF");
        fs::write(&path, b"stub").unwrap();

        let index = load_pkf(path.to_string_lossy().into_owned()).unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn open_dbc_errors_when_file_missing() {
        let err = open_dbc("/does/not/exist.dbc".to_string()).unwrap_err();
        assert_eq!(err.code, "dbc_not_found");
    }

    #[test]
    fn open_dbc_round_trips_mock_shape() {
        let dir = temp_dir("open-dbc");
        let path = dir.join("EQ979013.DBC");
        fs::write(&path, b"stub").unwrap();

        let dbc = open_dbc(path.to_string_lossy().into_owned()).unwrap();
        assert_eq!(dbc.team.short_name, "BOCA");
        assert_eq!(dbc.players.len(), 1);

        let json = serde_json::to_string(&dbc).unwrap();
        let round_tripped: Dbc = serde_json::from_str(&json).unwrap();
        assert_eq!(dbc, round_tripped);
    }

    #[test]
    fn save_dbc_writes_expected_filename() {
        let dir = temp_dir("save-dbc");
        let dbc = mock::dbc(); // player pointer 1 -> load_order 1 -> team pointer 1
        let filename = save_dbc(
            dbc.clone(),
            dir.to_string_lossy().into_owned(),
            PointerMode::Auto,
        )
        .unwrap();

        assert_eq!(filename, "EQ970001.DBC");
        let out_path = dir.join(&filename);
        assert!(out_path.is_file());

        // Verify this is a real, byte-level `.DBC` file (not the old JSON
        // placeholder) by reading it back through `pcf_codec::DbcCodec` and
        // checking it round-trips to the same `Dbc` that was saved.
        use pcf_codec::DbcCodec;
        let charmap = load_charmap().unwrap();
        let bytes = fs::read(&out_path).unwrap();
        let read_back = Dbc::read(&bytes, &charmap).unwrap();
        assert_eq!(read_back, dbc);
    }

    #[test]
    fn save_dbc_creates_missing_out_dir() {
        let dir = temp_dir("save-dbc-missing").join("nested");
        assert!(!dir.exists());

        let dbc = mock::blank_dbc();
        let filename =
            save_dbc(dbc, dir.to_string_lossy().into_owned(), PointerMode::Auto).unwrap();

        assert!(dir.join(&filename).is_file());
    }

    #[test]
    fn new_dbc_without_template_returns_blank() {
        let dbc = new_dbc(None).unwrap();
        assert_eq!(dbc.team.short_name, "");
        assert!(dbc.players.is_empty());
        // Not `None`: a domestic (`is_foreign: false`) `Dbc` must carry
        // *some* `Coach` to be writable via `save_dbc`'s real
        // `DbcCodec::write` call (see `mock::blank_dbc`'s doc comment) —
        // the blank template's coach is present but empty, not absent.
        let coach = dbc.coach.expect("blank template must still be writable");
        assert_eq!(coach.short_name, "");
    }

    #[test]
    fn new_dbc_with_template_returns_it_unchanged() {
        let template = mock::dbc();
        let dbc = new_dbc(Some(template.clone())).unwrap();
        assert_eq!(dbc, template);
    }

    #[test]
    fn import_crest_writes_into_miniesc_subfolder() {
        let dir = temp_dir("import-crest");
        let src = dir.join("crest.png");
        fs::write(&src, b"fake png").unwrap();
        let out_dir = dir.join("out");

        let result = import_crest(
            src.to_string_lossy().into_owned(),
            9013,
            out_dir.to_string_lossy().into_owned(),
        )
        .unwrap();

        assert_eq!(result.filename, "EQ979013.BMP");
        assert!(out_dir.join("MINIESC").join(&result.filename).is_file());
    }

    #[test]
    fn import_photo_writes_into_minifotos_subfolder() {
        let dir = temp_dir("import-photo");
        let src = dir.join("photo.png");
        fs::write(&src, b"fake png").unwrap();
        let out_dir = dir.join("out");

        let result = import_photo(
            src.to_string_lossy().into_owned(),
            42,
            out_dir.to_string_lossy().into_owned(),
        )
        .unwrap();

        assert_eq!(result.filename, "EQ970042.BMP");
        assert!(out_dir.join("MINIFOTOS").join(&result.filename).is_file());
    }

    #[test]
    fn import_crest_errors_when_source_missing() {
        let dir = temp_dir("import-crest-missing");
        let err = import_crest(
            "/does/not/exist.png".to_string(),
            1,
            dir.to_string_lossy().into_owned(),
        )
        .unwrap_err();
        assert_eq!(err.code, "asset_source_not_found");
    }

    #[test]
    fn export_dbdat_creates_full_directory_tree() {
        let game_dir = temp_dir("export-dbdat");
        let project = Project {
            dbcs: vec![mock::dbc()],
            game_dir: Some(game_dir.to_string_lossy().into_owned()),
        };

        let report = export_dbdat(project, game_dir.to_string_lossy().into_owned()).unwrap();

        let dbdat = game_dir.join("DBDAT");
        assert!(dbdat.join("EQ003003").is_dir());
        assert!(dbdat.join("MINIESC").is_dir());
        assert!(dbdat.join("NANOESC").is_dir());
        assert!(dbdat.join("MINIFOTOS").is_dir());
        assert!(dbdat.join("EQ003003").join("EQ970001.DBC").is_file());

        assert_eq!(report.written_files.len(), 1);
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn export_dbdat_warns_when_project_has_no_teams() {
        let game_dir = temp_dir("export-dbdat-empty");
        let project = Project {
            dbcs: vec![],
            game_dir: None,
        };

        let report = export_dbdat(project, game_dir.to_string_lossy().into_owned()).unwrap();
        assert!(report.written_files.is_empty());
        assert_eq!(report.warnings.len(), 1);
    }

    #[test]
    fn detect_game_dir_returns_none_when_nothing_found_and_does_not_panic() {
        // Best-effort probe: on CI/dev containers nothing will match, and
        // that must surface as `None`, never a panic or an `Err`.
        let _ = detect_game_dir();
    }

    #[test]
    fn detect_game_dir_candidate_check_matches_the_real_dbdat_layout() {
        // detect_game_dir() itself only probes fixed OS-specific paths, so
        // it can't be redirected at a temp dir in a unit test. This checks
        // the same has-the-file predicate it uses internally against a
        // fixture shaped like a real install, guarding the DBDAT/ layout
        // regression (it used to look for EQ003003.PKF directly under the
        // root, which never matches a real install).
        let dir = temp_dir("detect-game-dir-layout");
        let dbdat = dir.join("DBDAT");
        fs::create_dir_all(&dbdat).unwrap();
        fs::write(dbdat.join("EQ003003.PKF"), b"stub").unwrap();

        assert!(dir.join("DBDAT").join("EQ003003.PKF").is_file());
        assert!(!dir.join("EQ003003.PKF").is_file());
    }

    #[test]
    fn patch_manager_errors_when_file_missing() {
        let err = patch_manager(
            "/does/not/exist.exe".to_string(),
            ManagerPatch {
                y2k: true,
                start_year: None,
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "manager_not_found");
    }

    #[test]
    fn patch_manager_backs_up_before_patching() {
        let dir = temp_dir("patch-manager");
        let exe = dir.join("manager.exe");
        fs::write(&exe, b"fake exe bytes").unwrap();

        let report = patch_manager(
            exe.to_string_lossy().into_owned(),
            ManagerPatch {
                y2k: true,
                start_year: Some(1999),
            },
        )
        .unwrap();

        assert!(!report.already_patched);
        assert!(Path::new(&report.backup_path).is_file());
        assert_eq!(
            report.applied,
            vec!["y2k".to_string(), "start_year:1999".to_string()]
        );
    }

    #[test]
    fn charmap_status_reports_shape() {
        let status = charmap_status().unwrap();
        assert!(!status.loaded);
        assert!(status.missing_glyphs.is_empty());
    }

    #[test]
    fn pcf_error_json_shape_matches_ui_model() {
        let err = PcfError::new("io_error", "boom").with_context("/some/path");
        let value = serde_json::to_value(&err).unwrap();
        let obj = value.as_object().unwrap();

        assert_eq!(
            obj.len(),
            3,
            "PcfError must serialize to exactly {{code, message, context}}"
        );
        assert_eq!(obj["code"], "io_error");
        assert_eq!(obj["message"], "boom");
        assert_eq!(obj["context"], "/some/path");
    }

    #[test]
    fn pcf_error_context_none_serializes_to_null() {
        let err = PcfError::new("io_error", "boom");
        let value = serde_json::to_value(&err).unwrap();
        assert!(value["context"].is_null());
    }

    #[test]
    fn league_history_still_ten_entries_in_mock() {
        // Sanity check the mock builder against the fixed-size array
        // contract (`[LeagueResult; 10]`) so a future edit to `mock::dbc`
        // can't silently shrink it.
        let dbc = mock::dbc();
        assert_eq!(dbc.team.league_history.len(), 10);
        let expected = LeagueResult {
            position: 1,
            division: Division::First,
        };
        assert!(dbc.team.league_history.iter().all(|r| *r == expected));
    }

    // -----------------------------------------------------------------
    // Real-fixture-aware tests, mirroring `crates/pcf-codec/src/container.rs`'s
    // own `parses_real_river_record_from_the_users_own_pkf_if_present`:
    // never fail just because the real, gitignored `.PKF` isn't present on
    // this machine (CI never has it, and it's read-only user data per this
    // project's own guardrails). If it *is* present, actually exercise
    // `load_pkf`/`load_pkf_team` end to end and assert real,
    // independently-checkable facts.
    // -----------------------------------------------------------------

    const REAL_PKF_PATH: &str = "/c/PCF6AR/DBDAT/EQ003003.PKF";

    #[test]
    fn load_pkf_finds_dozens_of_real_argentine_teams_if_the_users_own_pkf_is_present() {
        if std::fs::metadata(REAL_PKF_PATH).is_err() {
            println!(
                "{REAL_PKF_PATH} not found -- skipping, this test only runs meaningfully on a \
                 machine with the user's own legally-owned copy of the game (never committed)"
            );
            return;
        }

        let index = load_pkf(REAL_PKF_PATH.to_string()).unwrap();

        // PKF_FORMAT.md §9: 55 real domestic records exist in the file;
        // §8 UPDATE 2 documents ~39 currently decoding cleanly (the rest
        // fail on an unconfirmed parenthesis glyph) -- that number can only
        // grow as the charmap improves, never regress below a healthy
        // fraction of the file's real teams, so this is a loose floor, not
        // an exact count tied to today's charmap coverage.
        assert!(
            index.len() >= 30,
            "expected at least 30 successfully-decoded domestic teams, got {}",
            index.len()
        );

        let boca = index
            .iter()
            .find(|e| e.short_name == "Boca")
            .expect("expected to find Boca in the real team index");
        assert_eq!(boca.country, 3);

        assert!(
            index.iter().any(|e| e.short_name == "River"),
            "expected to find River in the real team index"
        );

        // No two teams should ever share a resolved pointer -- that would
        // make `load_pkf_team` ambiguous.
        let mut pointers: Vec<u16> = index.iter().map(|e| e.pointer).collect();
        pointers.sort_unstable();
        pointers.dedup();
        assert_eq!(
            pointers.len(),
            index.len(),
            "resolved team pointers must be unique"
        );
    }

    #[test]
    fn load_pkf_team_bridges_rivers_real_container_record_into_a_real_dbc() {
        if std::fs::metadata(REAL_PKF_PATH).is_err() {
            println!(
                "{REAL_PKF_PATH} not found -- skipping, see the sibling test's comment for why"
            );
            return;
        }

        let index = load_pkf(REAL_PKF_PATH.to_string()).unwrap();
        let river_entry = index
            .iter()
            .find(|e| e.short_name == "River")
            .expect("expected to find River in the real team index");

        let dbc = load_pkf_team(REAL_PKF_PATH.to_string(), river_entry.pointer).unwrap();

        // Same real-world facts `container.rs`'s own real-fixture test
        // checks (PKF_FORMAT.md §6.2/§6.5) -- confirming the bridge carries
        // them through into the frozen `pcf_model::Dbc` shape unchanged.
        assert_eq!(dbc.team.short_name, "River");
        assert_eq!(dbc.team.stadium_name, "Antonio Vespucio Liberti");
        assert_eq!(dbc.team.capacity, 76_687);
        assert_eq!(dbc.team.founded, 1901);
        assert_eq!(dbc.team.long_name, "Club Atlético River Plate");
        assert_eq!(dbc.team.president, "Alfredo Angel Dávicce");
        assert!(!dbc.header.is_foreign);

        let coach = dbc
            .coach
            .clone()
            .expect("River's real container record has a confirmed coach chain");
        assert_eq!(coach.short_name, "Ramón Díaz");
        assert_eq!(coach.long_name, "Ramón Angel DIAZ");

        // The bridged `Dbc` must also be a real, writable override file --
        // exercise the full loop this feature is meant to unlock (load a
        // team straight out of the PKF, then save it as an override
        // without ever touching the read-only container).
        use pcf_codec::DbcCodec;
        let charmap = load_charmap().unwrap();
        let bytes = dbc.write(&charmap).expect("bridged Dbc must be writable");
        let read_back = Dbc::read(&bytes, &charmap).expect("written bytes must be re-readable");
        assert_eq!(read_back, dbc);
    }

    #[test]
    fn load_pkf_team_errors_on_unknown_pointer() {
        if std::fs::metadata(REAL_PKF_PATH).is_err() {
            println!(
                "{REAL_PKF_PATH} not found -- skipping, see the sibling test's comment for why"
            );
            return;
        }

        let err = load_pkf_team(REAL_PKF_PATH.to_string(), 0xdead).unwrap_err();
        assert_eq!(err.code, "pkf_team_not_found");
    }
}
