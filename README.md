# studio (`idle-studio` + `render`) — v5.0.1 (100% Native openOODA `.oo`)

<div align="center">

| Security Pillar | Verification Badge |
| :--- | :---: |
| **Platform Standard** | [![secured by studio2201][b-studio]][u-home] |
| **Credential Defense** | [![snip][b-snip]][u-snip] |
| **Supply Chain Surface** | [![vigil][b-vigil]][u-vigil] |
| **Post-Quantum Cryptography** | [![aegis][b-aegis]][u-aegis] |
| **Build Provenance & SLSA** | [![proven][b-proven]][u-proven] |
| **Repository Governance** | [![boneyard][b-boneyard]][u-boneyard] |

[b-studio]: https://img.shields.io/badge/secured%20by-studio2201-2f6f5e?logo=shield
[u-home]: https://studio2201.com
[b-snip]: https://img.shields.io/badge/snip-0%20secrets-2f6f5e?logo=shield
[u-snip]: https://studio2201.com/snip
[b-vigil]: https://img.shields.io/badge/vigil-0%20dependencies-2f6f5e?logo=shield
[u-vigil]: https://studio2201.com/vigil
[b-aegis]: https://img.shields.io/badge/aegis-PQC%20compliant-2f6f5e?logo=shield
[u-aegis]: https://studio2201.com/aegis
[b-proven]: https://img.shields.io/badge/proven-ML--DSA--65%20verified-2f6f5e?logo=shield
[u-proven]: https://studio2201.com/proven
[b-boneyard]: https://img.shields.io/badge/boneyard-maintained-2f6f5e?logo=shield
[u-boneyard]: https://studio2201.com/boneyard

</div>

Offline saver -> video rendering in **100% native openOODA (`.oo`)**: the `render` engine (headless simulation -> AV1/H.264/PNG/raw) plus `idle-studio`, the Director TUI for queueing and tuning export jobs. The engine compiles the daemon's `.oo` rendering core directly (`core/*.oo`, `raster/*.oo`, `ascii/**/*.oo`), so an export runs **the daemon's own 53 scenes** — the exact same deterministic 16-bit scenes the screensaver draws on screen. Companion to the [IdleScreen](https://idlescreen.github.io) (`v5.0.1`) native openOODA monorepo.

## Build & Verify (Native `.oo`)

```sh
git clone https://github.com/idlescreen/studio
cd studio
make all   # builds bin/idle-studio & bin/render, verifies 16-256 line Page Rule, runs tests & determinism check
```

## Commands

```sh
./bin/idle-studio                                         # Director TUI: live preview + queue + tune export jobs
./bin/render -e beams --duration 30s -o out.mkv           # one of the daemon's 53 scenes -> AV1/Matroska
./bin/render -e ripple --text "OPENOODA" --duration 30s -o out.mkv
./bin/idle-studio enqueue -e aurora --duration 10s -o /tmp/aurora.mkv
./bin/idle-studio list
./bin/idle-studio run --all
```

Every scene name the daemon ships is accepted by `-e`; an unknown name is refused fail-closed.

## License

Apache-2.0 · © 2026 IdleScreen
