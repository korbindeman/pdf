mod block;
mod config;
mod parser;
mod typst;

pub use block::{Block, List, ListItem, Span};
pub use config::Config;

use typst_as_lib::typst_kit_options::TypstKitFontOptions;
use typst_as_lib::TypstEngine;
use typst_pdf::PdfOptions;

// Bundled Open Sans font for sans-serif
static OPEN_SANS_REGULAR: &[u8] = include_bytes!("../fonts/OpenSans-Regular.ttf");
static OPEN_SANS_BOLD: &[u8] = include_bytes!("../fonts/OpenSans-Bold.ttf");
static OPEN_SANS_ITALIC: &[u8] = include_bytes!("../fonts/OpenSans-Italic.ttf");
static OPEN_SANS_BOLD_ITALIC: &[u8] = include_bytes!("../fonts/OpenSans-BoldItalic.ttf");

/// Parse markdown text into a vector of blocks.
pub fn parse(markdown: &str) -> Vec<Block> {
    parser::parse(markdown)
}

/// Convert markdown to Typst markup using default config.
pub fn markdown_to_typst(markdown: &str) -> String {
    markdown_to_typst_with_config(markdown, &Config::compiled_default())
}

/// Convert markdown to Typst markup with custom config.
pub fn markdown_to_typst_with_config(markdown: &str, config: &Config) -> String {
    let blocks = parse(markdown);
    typst::blocks_to_typst(&blocks, config)
}

/// Convert markdown to PDF bytes using default config.
pub fn markdown_to_pdf(markdown: &str) -> Result<Vec<u8>, String> {
    markdown_to_pdf_with_config(markdown, &Config::compiled_default())
}

/// Convert markdown to PDF bytes with custom config.
pub fn markdown_to_pdf_with_config(markdown: &str, config: &Config) -> Result<Vec<u8>, String> {
    use typst_library::layout::PagedDocument;

    let typst_content = markdown_to_typst_with_config(markdown, config);

    let font_options = TypstKitFontOptions::new()
        .include_embedded_fonts(true)
        .include_system_fonts(false);

    let engine = TypstEngine::builder()
        .main_file(typst_content)
        .fonts([
            OPEN_SANS_REGULAR,
            OPEN_SANS_BOLD,
            OPEN_SANS_ITALIC,
            OPEN_SANS_BOLD_ITALIC,
        ])
        .search_fonts_with(font_options)
        .build();

    let doc: PagedDocument = engine
        .compile()
        .output
        .map_err(|e| format!("Typst compilation failed: {:?}", e))?;

    typst_pdf::pdf(&doc, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))
}
