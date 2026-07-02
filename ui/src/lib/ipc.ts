// Typed wrappers over Tauri's invoke() for every command in PLAN.md §4.3.
//
// Agent D's real Tauri backend is being built in parallel. Until
// window.__TAURI__ exists (i.e. we're not running inside the Tauri
// webview), every function here falls back to the fixtures in ./mocks/ so
// the whole UI runs standalone in a plain browser with zero backend.
//
// Payloads are typed against ./model.ts, the hand-mirrored contract for
// crates/pcf-model. Do not add ad-hoc shapes here — extend model.ts first.

import type {
  AssetResult,
  CharmapInfo,
  Dbc,
  ExportReport,
  ManagerPatch,
  PatchReport,
  PointerMode,
  Project,
  TeamIndex,
} from "./model";
import { mockDbc, mockTeamIndex } from "./mocks/dbc";
import {
  mockAssetResult,
  mockCharmapInfo,
  mockExportReport,
  mockGameDir,
  mockPatchReport,
} from "./mocks/ipc";

/** True when running inside the Tauri webview (real backend available). */
export function hasTauriBackend(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
}

async function invokeTauri<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  // Imported lazily so a plain-browser dev session never needs the
  // @tauri-apps/api package resolved/bundled for anything it actually uses.
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export async function loadPkf(path: string): Promise<TeamIndex> {
  if (hasTauriBackend()) return invokeTauri<TeamIndex>("load_pkf", { path });
  return structuredClone(mockTeamIndex);
}

export async function openDbc(path: string): Promise<Dbc> {
  if (hasTauriBackend()) return invokeTauri<Dbc>("open_dbc", { path });
  return structuredClone(mockDbc);
}

export async function saveDbc(
  dbc: Dbc,
  outDir: string,
  mode: PointerMode,
): Promise<string> {
  if (hasTauriBackend()) {
    return invokeTauri<string>("save_dbc", { dbc, outDir, mode });
  }
  const pointer = dbc.players[0]?.pointer ?? 0;
  return `EQ97${String(pointer).padStart(4, "0")}.DBC`;
}

export async function newDbc(template?: Dbc): Promise<Dbc> {
  if (hasTauriBackend()) return invokeTauri<Dbc>("new_dbc", { template });
  return structuredClone(template ?? mockDbc);
}

export async function importCrest(
  img: string,
  teamPointer: number,
  outDir: string,
): Promise<AssetResult> {
  if (hasTauriBackend()) {
    return invokeTauri<AssetResult>("import_crest", { img, teamPointer, outDir });
  }
  return { ...mockAssetResult, filename: `${teamPointer}.bmp` };
}

export async function importPhoto(
  img: string,
  playerPointer: number,
  outDir: string,
): Promise<AssetResult> {
  if (hasTauriBackend()) {
    return invokeTauri<AssetResult>("import_photo", { img, playerPointer, outDir });
  }
  return { ...mockAssetResult, filename: `${playerPointer}.bmp` };
}

export async function exportDbdat(project: Project, gameDir: string): Promise<ExportReport> {
  if (hasTauriBackend()) {
    return invokeTauri<ExportReport>("export_dbdat", { project, gameDir });
  }
  return structuredClone(mockExportReport);
}

export async function detectGameDir(): Promise<string | null> {
  if (hasTauriBackend()) return invokeTauri<string | null>("detect_game_dir");
  return mockGameDir;
}

export async function patchManager(path: string, opts: ManagerPatch): Promise<PatchReport> {
  if (hasTauriBackend()) {
    return invokeTauri<PatchReport>("patch_manager", { path, opts });
  }
  return mockPatchReport(opts);
}

export async function charmapStatus(): Promise<CharmapInfo> {
  if (hasTauriBackend()) return invokeTauri<CharmapInfo>("charmap_status");
  return structuredClone(mockCharmapInfo);
}
