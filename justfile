default:
    @just --list

build:
    cargo build -p pdf_cli --release
    cp target/release/pdf ~/dev/_scripts/

install:
    cd crates/pdf_app && bun install

dev:
    cd crates/pdf_app && bun run tauri dev
