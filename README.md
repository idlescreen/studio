# studio

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

Offline saver → video rendering: the `render` engine (headless simulation →
AV1/H.264/PNG/raw via ffmpeg) plus `idle-studio`, the Director TUI for
queueing and tuning export jobs. Companion to the
[IdleScreen](https://idlescreen.github.io) (`v4.7.0`) monorepo — high-performance
unified single-binary Wayland screensaver and idle management suite for Linux (First-Class SuperFedora
integration, 52 built-in 16-bit arcade & mathematical scenes, 10-bit HDR deep color, 4K 2x Hi-Res `wp_viewporter`
GPU scaling, system theme adaptation, interactive `--tour` preview & TUI dashboard, and multi-init support across `systemd`, `OpenRC`,
`runit`, `s6`, `dinit`, and XDG autostart).

## Build & Run

Build from the studio repository (with optional `target-cpu=native` SIMD tuning):

```sh
git clone https://github.com/idlescreen/studio
cd studio
RUSTFLAGS="-C target-cpu=native" cargo build --release --workspace
```

## Commands

```sh
idle-studio                                   # Director TUI: queue + tune export jobs
render -e ascii --duration 30s -o out.mkv     # headless ASCII banner/effect → AV1/Matroska
render -e beams --duration 30s -o out.mkv     # headless mathematical saver → AV1/Matroska
render -e ripple --format png -o frames/      # one PNG per frame
```

## License

Apache-2.0 · © 2026 IdleScreen
