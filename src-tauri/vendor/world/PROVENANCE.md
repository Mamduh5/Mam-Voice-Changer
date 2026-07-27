# WORLD source provenance

- Upstream: `https://github.com/mmorise/World`
- Official release: `v1.0.1`
- Pinned commit: `d625e7608ca23a870018f01e7c562ac683d9847f`
- License: official modified BSD license in
  `licenses/WORLD-modified-BSD.txt`
- Imported on: 2026-07-24

Only the source needed for the offline reference experiment was imported:

- `src/cheaptrick.cpp`
- `src/common.cpp`
- `src/d4c.cpp`
- `src/fft.cpp`
- `src/harvest.cpp`
- `src/matlabfunctions.cpp`
- `src/synthesis.cpp`
- `src/world/cheaptrick.h`
- `src/world/common.h`
- `src/world/constantnumbers.h`
- `src/world/d4c.h`
- `src/world/fft.h`
- `src/world/harvest.h`
- `src/world/macrodefinitions.h`
- `src/world/matlabfunctions.h`
- `src/world/synthesis.h`

No upstream algorithm source was modified. The Mam Voice Changer C ABI is an
external wrapper in `src-tauri/native/world_wrapper.cpp`.
