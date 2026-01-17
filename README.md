# pdf

A Rust CLI tool and library for converting Markdown files to PDF. Uses Typst as an embedded library with bundled fonts. No external dependencies required.

---

## Features

- Headings (H1-H6)
- Paragraphs with bold, italic, and inline code
- Code blocks with syntax highlighting
- Ordered and unordered lists (with nesting)
- Tables
- Horizontal rules
- Smart page breaks (keeps headings with content, avoids widows/orphans)
- Beautiful typography with embedded Libertinus Serif font

---

## CLI Usage

```bash
# Convert markdown to PDF (output: input.pdf)
pdf input.md

# Specify output file
pdf input.md -o output.pdf
```

---

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
pdf = { path = "path/to/pdf" }
```

Basic usage:

```rust
use std::fs;

fn main() {
    let markdown = "# Hello\n\nThis is **bold** and *italic*.";
    let pdf_bytes = pdf::markdown_to_pdf(markdown).expect("failed to create PDF");
    fs::write("output.pdf", pdf_bytes).expect("failed to write file");
}
```

You can also access the intermediate Typst markup:

```rust
fn main() {
    let markdown = "# Title\n\nParagraph text.";
    let typst_markup = pdf::markdown_to_typst(markdown);
    println!("{}", typst_markup);
}
```

---

## Building

```bash
cargo build --release
```
