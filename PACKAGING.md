# Packaging — portable single-exe (PLAN.md §1, §6 Agent G, §7 M-REL)

Target: a single portable executable, no installer, ~5–15 MB. This note is
a sketch of how we get there from `cargo tauri build`, plus what's already
set vs. still to confirm once Agent D's `src-tauri` and Agent E/F's `ui/`
are further along. Nothing here has been exercised end-to-end yet — there's
no built UI to bundle and no manual verification has been run.

## Why Tauri gets us most of the way for free

Unlike Electron, Tauri doesn't bundle a browser runtime into the binary — on
Windows it uses the OS-provided WebView2 (evergreen runtime, already on
essentially every up-to-date Windows 10/11 box; if truly absent, Tauri can
optionally bootstrap it, which we should avoid to keep the "no runtime
dependency to install" guarantee). That's most of the size difference
between a ~5–15 MB Tauri exe and a ~100+ MB Electron app already accounted
for before any trimming.

## What "no installer" means in Tauri terms

`src-tauri/tauri.conf.json` currently has:

```json
"bundle": { "active": false }
```

(Agent D's setting, as of this writing — re-read the file before changing
it, per the ownership rule.) With bundling inactive, `cargo tauri build`
still produces the raw platform binary at
`src-tauri/target/release/pcf-editor(.exe)` — it just skips wrapping it in
an NSIS/MSI/DMG/AppImage installer. **That raw binary is the portable
artifact.** Zip it (or ship it bare) for release; there's no separate
"portable mode" to configure beyond *not* turning bundling back on for an
installer-producing target. If bundling ever needs to be active for some
other reason (e.g. code signing metadata), keep `bundle.targets` scoped to
whatever produces a bare/portable executable for the platform, not
`nsis`/`msi`/`dmg`.

`identifier` (`com.pcfutbol.editor`) and `productName` (`pcf-editor`) are
already set and look fine to keep as-is.

## Getting from "it builds" to 5–15 MB

1. **Release profile (done — workspace `Cargo.toml`):**
   ```toml
   [profile.release]
   opt-level = "s"
   lto = true
   codegen-units = 1
   strip = true
   ```
   `opt-level = "s"` favors size over speed, `lto = true` +
   `codegen-units = 1` let the linker dedupe/inline across the whole
   dependency graph, `strip = true` drops debug symbols from the final
   binary. This is workspace-wide, so it applies to `src-tauri`'s binary
   too without D needing to duplicate it.

2. **Trim Tauri's own feature surface (sketch — Agent D's `src-tauri/Cargo.toml`):**
   Only enable Tauri features actually used. Concretely, avoid pulling in
   (unless a screen needs them): system tray, global shortcuts, the
   updater, devtools in release builds. Fewer enabled features means fewer
   compiled-in dependencies (icon decoders, HTTP clients, etc.) that show
   up in the final binary size. `--no-default-features` plus an explicit
   feature list on whatever Tauri crate(s) `src-tauri` depends on is the
   mechanism; the exact list is D's call once the feature set is known —
   not something to hardcode here speculatively.

3. **Keep the frontend light (already the stack choice, PLAN.md §2):**
   Svelte + Vite instead of a heavier framework keeps the compiled
   `ui/dist` (embedded into the binary via Tauri's asset embedding) small.
   Run the frontend build in production mode (`vite build`, minified,
   tree-shaken) — not the dev server output — before `cargo tauri build`
   picks up `frontendDist`. Avoid shipping unminified fonts/spritesheets;
   the retro skin's assets (PLAN.md §6 Agent E) are the most likely place
   for bundle bloat to sneak in, since they're the one part of this app
   that isn't just Rust logic.

4. **Strip further post-build if still over budget:** `strip` on the
   produced Linux binary (for the Wine-users target — PLAN.md §6 Agent G
   deliverables), and consider `upx` as a last resort if the plain-stripped
   binary still doesn't hit the range — not applied by default here since
   UPX can trip some antivirus heuristics; evaluate once we have a real
   binary to measure.

## Linux target (Wine users)

PLAN.md's release deliverable includes "single-file portable build for
Windows (and Linux for Wine users)". `cargo tauri build --target
x86_64-unknown-linux-gnu` (or building natively inside the dev container,
which already has the GTK/WebKit2GTK dev libs per `Dockerfile.dev`) produces
the Linux side; the same `bundle.active = false` + release-profile settings
apply. Not yet exercised — there's no UI to actually launch and screenshot
across platforms yet.

## Verification checklist (for M-REL, not done yet)

- [ ] `cargo tauri build` succeeds inside the dev container.
- [ ] Resulting binary size is inside 5–15 MB (measure, don't assume).
- [ ] Binary launches on a clean Windows machine with no installer step and
      no missing-runtime error.
- [ ] Binary opens a real DBC end-to-end (blocked on Agents A/D landing real
      implementations, and on the user supplying `fixtures/golden` — see
      `fixtures/README.md`).
