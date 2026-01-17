mod block;
mod parser;
mod typst;

pub use block::{Block, List, ListItem, Span};

use typst_as_lib::typst_kit_options::TypstKitFontOptions;
use typst_as_lib::TypstEngine;
use typst_pdf::PdfOptions;

/// Parse markdown text into a vector of blocks.
pub fn parse(markdown: &str) -> Vec<Block> {
    parser::parse(markdown)
}

/// Convert markdown to Typst markup.
pub fn markdown_to_typst(markdown: &str) -> String {
    let blocks = parse(markdown);
    typst::blocks_to_typst(&blocks)
}

/// Convert markdown to PDF bytes.
pub fn markdown_to_pdf(markdown: &str) -> Result<Vec<u8>, String> {
    use typst_library::layout::PagedDocument;

    let typst_content = markdown_to_typst(markdown);

    let font_options = TypstKitFontOptions::new()
        .include_embedded_fonts(true)
        .include_system_fonts(false);

    let engine = TypstEngine::builder()
        .main_file(typst_content)
        .search_fonts_with(font_options)
        .build();

    let doc: PagedDocument = engine
        .compile()
        .output
        .map_err(|e| format!("Typst compilation failed: {:?}", e))?;

    typst_pdf::pdf(&doc, &PdfOptions::default())
        .map_err(|e| format!("PDF generation failed: {:?}", e))
}
