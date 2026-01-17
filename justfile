default:
    @just --list

build:
    cargo build --release
    cp target/release/pdf ~/dev/_scripts/
