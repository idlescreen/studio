[![secured by studio2201](https://img.shields.io/badge/secured%20by-studio2201-2f6f5e?logo=shield)](https://studio2201.com)

# studio

[![snip](https://img.shields.io/github/actions/workflow/status/idlescreen/studio/snip.yml?label=snip&logo=shield)](https://github.com/idlescreen/studio/actions/workflows/snip.yml)
[![vigil](https://img.shields.io/github/actions/workflow/status/idlescreen/studio/vigil.yml?label=vigil&logo=shield)](https://github.com/idlescreen/studio/actions/workflows/vigil.yml)
[![aegis](https://img.shields.io/github/actions/workflow/status/idlescreen/studio/aegis.yml?label=aegis&logo=shield)](https://github.com/idlescreen/studio/actions/workflows/aegis.yml)
[![proven](https://img.shields.io/github/actions/workflow/status/idlescreen/studio/proven.yml?label=proven&logo=shield)](https://github.com/idlescreen/studio/actions/workflows/proven.yml)
[![boneyard](https://img.shields.io/github/actions/workflow/status/idlescreen/studio/boneyard.yml?label=boneyard&logo=shield)](https://github.com/idlescreen/studio/actions/workflows/boneyard.yml)

Offline saver → video rendering: the `render` engine (headless sim →
AV1/H.264/PNG/raw via ffmpeg) plus `idle-studio`, the Director TUI for
queueing and tuning export jobs. Part of
[IdleScreen](https://idlescreen.github.io) — modular Wayland screensavers
for Linux.

## Install

```sh
idlescreen install studio
```

## Commands

```sh
idlescreen studio                             # Director TUI: queue + tune jobs
render -e beams --duration 30s -o out.mkv     # headless render → AV1/Matroska
render -e ripple --format png -o frames/      # one PNG per frame
```

## License

Apache-2.0 · © 2026 IdleScreen

---

<div align="center">

[![Necrometer](necrometer.svg)](https://necrometer.dev/?u=idlescreen)

</div>
