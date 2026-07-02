// Additional mock fixtures for the IPC commands that don't already have a
// fixture in ./dbc.ts. Used by ../ipc.ts as the offline fallback whenever
// window.__TAURI__ isn't present, so the whole app runs standalone in a
// browser with zero backend (Agent D's real Tauri commands land later).

import type {
  AssetResult,
  CharmapInfo,
  ExportReport,
  ManagerPatch,
  PatchReport,
} from "../model";

export const mockAssetResult: AssetResult = {
  filename: "9013.bmp",
  width: 64,
  height: 64,
};

export const mockExportReport: ExportReport = {
  writtenFiles: ["DBDAT\\EQ003003\\EQ979013.DBC"],
  warnings: [],
};

export const mockGameDir = "C:\\Games\\PC Apertura 98-99";

export function mockPatchReport(opts: ManagerPatch): PatchReport {
  return {
    alreadyPatched: false,
    backupPath: "manager.exe.bak",
    applied: [
      ...(opts.y2k ? ["y2k"] : []),
      ...(opts.startYear !== null ? ["start_year"] : []),
    ],
  };
}

export const mockCharmapInfo: CharmapInfo = {
  loaded: true,
  missingGlyphs: [],
};
