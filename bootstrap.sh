#!/usr/bin/env bash
set -euo pipefail

if command -v apt-get >/dev/null 2>&1; then
    # A stalled mirror or an interactive debconf prompt used to leave this
    # script hanging with no output, for the installer and in CI alike. The
    # retries ride out a flaky mirror; the HTTP timeout turns a stalled
    # connection into an error; the non-interactive front end stops a
    # configuration question from blocking on input that never comes.
    export DEBIAN_FRONTEND=noninteractive
    APT_OPTS=(-o Acquire::Retries=3 -o Acquire::http::Timeout=30)
    sudo apt-get "${APT_OPTS[@]}" update
    sudo apt-get "${APT_OPTS[@]}" install -y libdbus-1-dev libwayland-dev libxkbcommon-dev libssl-dev pkg-config libudev-dev ffmpeg
fi

if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    fi
fi

rustup show >/dev/null

cargo test --workspace
