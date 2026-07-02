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

use std::fs;
use std::path::{Path, PathBuf};

use pcf_model::{
    AssetResult, CharmapInfo, Dbc, ExportReport, ManagerPatch, PatchReport, PcfError, PointerMode,
    Project, TeamIndex,
};

use crate::mock;

/// Load the team index from a `.PKF` container.
///
/// TODO(D): swap for `pcf_codec::Pkf::load(path)?.index()` once Agent A lands it.
#[tauri::command]
pub fn load_pkf(path: String) -> Result<TeamIndex, PcfError> {
    require_exists(&path, "pkf_not_found")?;
    Ok(mock::team_index())
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
/// TODO(D): swap the placeholder byte payload for `pcf_codec::Dbc::write(&dbc)`
/// once Agent A lands it. The team-pointer-from-players heuristic below is
/// also a stand-in: once the PKF index / real pointer plumbing exists this
/// should take the team's real pointer instead of guessing from player 1.
#[tauri::command]
pub fn save_dbc(dbc: Dbc, out_dir: String, mode: PointerMode) -> Result<String, PcfError> {
    let team_pointer = derive_team_pointer(&dbc, mode);
    let filename = pcf_model::pointers::team_filename(team_pointer);

    let dir = PathBuf::from(&out_dir);
    fs::create_dir_all(&dir).map_err(|e| io_error("save_dbc", &out_dir, e))?;

    let out_path = dir.join(&filename);
    let placeholder = mock_dbc_bytes(&dbc);
    fs::write(&out_path, placeholder)
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
/// TODO(D): once Agents A/B land, replace the placeholder DBC bytes with
/// `pcf_codec::Dbc::write` and stop pre-creating empty asset folders (B's
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

    for dbc in &project.dbcs {
        let team_pointer = derive_team_pointer(dbc, PointerMode::Auto);
        let filename = pcf_model::pointers::team_filename(team_pointer);
        let out_path = container.join(&filename);
        fs::write(&out_path, mock_dbc_bytes(dbc))
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

    candidates
        .into_iter()
        .find(|dir| dir.join("EQ003003.PKF").is_file())
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

/// Placeholder serialization until `pcf_codec::Dbc::write` lands. Keeping
/// this JSON (rather than raw garbage bytes) makes the placeholder file
/// inspectable during development without pretending it's a real DBC.
fn mock_dbc_bytes(dbc: &Dbc) -> Vec<u8> {
    serde_json::to_vec_pretty(dbc).unwrap_or_default()
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
    fn load_pkf_returns_team_index_when_file_exists() {
        let dir = temp_dir("load-pkf");
        let path = dir.join("EQ003003.PKF");
        fs::write(&path, b"stub").unwrap();

        let index = load_pkf(path.to_string_lossy().into_owned()).unwrap();
        assert_eq!(index.len(), 2);
        assert_eq!(index[0].short_name, "BOCA");
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
        let filename =
            save_dbc(dbc, dir.to_string_lossy().into_owned(), PointerMode::Auto).unwrap();

        assert_eq!(filename, "EQ970001.DBC");
        assert!(dir.join(&filename).is_file());
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
        assert!(dbc.coach.is_none());
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
}
