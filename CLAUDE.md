# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release    # Build release binary
cargo test               # Run tests
cargo run -- input.md    # Convert markdown to PDF
just build               # Build and copy to ~/dev/_scripts/
```

## Architecture

This is a Rust CLI tool that converts Markdown to PDF using Typst as the rendering engine.

**Pipeline:** Markdown → Block AST → Typst markup → PDF

- `src/main.rs` - CLI entry point using clap, reads markdown file and writes PDF
- `src/lib.rs` - Public API exposing `parse()`, `markdown_to_typst()`, and `markdown_to_pdf()`
- `src/block.rs` - AST types: `Block` (heading, paragraph, code block, list, table, rule) and `Span` (text, bold, italic, code, line break)
- `src/parser.rs` - Converts markdown to Block AST using pulldown-cmark with a state machine
- `src/typst.rs` - Converts Block AST to Typst markup string, handles escaping and page break prevention

**Key design decisions:**
- Fonts are bundled via typst-kit-embed-fonts (no system font dependencies)
- Headings are grouped with following content using `#block(breakable: false)` to prevent orphaned headings
- Small lists (≤5 items) and tables are kept together on one page
