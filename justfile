# `just` with no args shows this list
default:
    @just --list

# Everything CI checks, in CI's order — run before pushing
check: fmt-check clippy test

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-targets -- -D warnings

test:
    cargo test --all-targets

# Release build matters: debug FFT + decode can underrun the audio buffer
play *ARGS:
    cargo run --release --bin rtrax -- {{ ARGS }}

# The native GUI frontend (egui), e.g. `just gui song.xm`
gui *ARGS:
    cargo run --release --bin rtrax-gui -- {{ ARGS }}

# Headless playback smoke test (no TUI), e.g. `just play-headless song.xm`
play-headless FILE:
    cargo run --release -p rtrax-core --example play -- {{ FILE }}

build:
    cargo build --release
