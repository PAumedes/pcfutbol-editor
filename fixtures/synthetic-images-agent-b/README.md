# Synthetic image fixtures (Agent B)

**Everything in this folder is hand-constructed placeholder data, not real
game assets.** No real crest/photo BMP or real Apertura 98/99 `.pal` file
has been supplied yet.

- `synthetic_palette.rgb` — a 768-byte raw RGB dump (256 * 3 bytes, no
  header) of `pcf_images::Palette::synthetic_placeholder()`: an 8x8x4 RGB
  color cube. Loadable via `Palette::from_raw_rgb_bytes`.
- `synthetic_checker_4x4.bmp` / `synthetic_strip_2x3.bmp` — tiny 8-bit
  indexed BMPs written by `pcf_images::write_bmp8` against the same
  synthetic palette, used for byte-identical round-trip tests.

Regenerate with:

```
cargo run -p pcf-images --example gen_synthetic_fixtures
```

TODO(B): once the user supplies a real game palette and real
MINIESC/NANOESC/MINIFOTOS samples, validate against those instead and
replace/extend this folder as needed (do not delete without checking
whether other tests still reference it).
