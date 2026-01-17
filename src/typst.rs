use crate::block::{Block, List, Span};

/// Convert blocks to Typst markup
pub fn blocks_to_typst(blocks: &[Block]) -> String {
    let mut out = String::new();

    // Set up paragraph settings to prevent widows/orphans
    out.push_str("#set par(linebreaks: \"optimized\")\n\n");

    let mut i = 0;
    while i < blocks.len() {
        let block = &blocks[i];

        match block {
            Block::Heading { .. } => {
                // Keep heading with following content using a block that prevents breaks
                out.push_str("#block(breakable: false)[\n");
                emit_heading(block, &mut out);

                // Include the next block if it exists (to keep heading with first content)
                if i + 1 < blocks.len() {
                    i += 1;
                    emit_block(&blocks[i], &mut out);
                }
                out.push_str("]\n\n");
            }
            _ => {
                emit_block(block, &mut out);
            }
        }

        i += 1;
    }

    out
}

fn emit_heading(block: &Block, out: &mut String) {
    if let Block::Heading { level, content } = block {
        for _ in 0..*level {
            out.push('=');
        }
        out.push(' ');
        spans_to_typst(content, out);
        out.push('\n');
        out.push('\n');
    }
}

fn emit_block(block: &Block, out: &mut String) {
    match block {
        Block::Heading { .. } => {
            emit_heading(block, out);
        }
        Block::Paragraph { content } => {
            spans_to_typst(content, out);
            out.push('\n');
            out.push('\n');
        }
        Block::CodeBlock { language, content } => {
            // Keep code blocks together when possible
            out.push_str("#block(breakable: false)[\n```");
            if let Some(lang) = language {
                out.push_str(lang);
            }
            out.push('\n');
            out.push_str(content);
            if !content.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("```\n]\n\n");
        }
        Block::List(list) => {
            // Wrap list to keep together when small, allow breaks when large
            let item_count = count_list_items(list);
            if item_count <= 5 {
                out.push_str("#block(breakable: false)[\n");
                list_to_typst(list, 0, out);
                out.push_str("]\n\n");
            } else {
                list_to_typst(list, 0, out);
                out.push('\n');
            }
        }
        Block::Table { headers, rows } => {
            // Keep tables together when possible
            out.push_str("#block(breakable: false)[\n");
            table_to_typst(headers, rows, out);
            out.push_str("]\n\n");
        }
        Block::Rule => {
            out.push_str("#line(length: 100%)\n\n");
        }
    }
}

fn count_list_items(list: &List) -> usize {
    let mut count = list.items.len();
    for item in &list.items {
        if let Some(ref nested) = item.nested {
            count += count_list_items(nested);
        }
    }
    count
}

fn spans_to_typst(spans: &[Span], out: &mut String) {
    for span in spans {
        span_to_typst(span, out);
    }
}

fn span_to_typst(span: &Span, out: &mut String) {
    match span {
        Span::Text(text) => {
            // Escape special Typst characters
            for ch in text.chars() {
                match ch {
                    '#' | '*' | '_' | '@' | '$' | '\\' | '`' | '<' | '>' | '[' | ']' => {
                        out.push('\\');
                        out.push(ch);
                    }
                    _ => out.push(ch),
                }
            }
        }
        Span::Bold(inner) => {
            out.push('*');
            spans_to_typst(inner, out);
            out.push('*');
        }
        Span::Italic(inner) => {
            out.push('_');
            spans_to_typst(inner, out);
            out.push('_');
        }
        Span::Code(text) => {
            out.push('`');
            // Inside raw/code, backticks need special handling
            out.push_str(&text.replace('`', "\\`"));
            out.push('`');
        }
        Span::LineBreak => {
            out.push_str(" \\\n");
        }
    }
}

fn list_to_typst(list: &List, indent: usize, out: &mut String) {
    let prefix = if list.ordered { "+" } else { "-" };
    let indent_str: String = "  ".repeat(indent);

    for item in &list.items {
        out.push_str(&indent_str);
        out.push_str(prefix);
        out.push(' ');
        spans_to_typst(&item.content, out);
        out.push('\n');

        if let Some(ref nested) = item.nested {
            list_to_typst(nested, indent + 1, out);
        }
    }
}

fn table_to_typst(headers: &[Vec<Span>], rows: &[Vec<Vec<Span>>], out: &mut String) {
    let col_count = headers.len();
    if col_count == 0 {
        return;
    }

    out.push_str("#table(\n");
    out.push_str(&format!("  columns: {},\n", col_count));

    // Header cells (bold)
    for cell in headers {
        out.push_str("  [*");
        spans_to_typst(cell, out);
        out.push_str("*],\n");
    }

    // Data rows
    for row in rows {
        for cell in row {
            out.push_str("  [");
            spans_to_typst(cell, out);
            out.push_str("],\n");
        }
    }

    out.push_str(")\n");
}

#[cfg(test)]
mod tests {
    use crate::markdown_to_typst;

    const PREAMBLE: &str = "#set par(linebreaks: \"optimized\")\n\n";

    #[test]
    fn heading() {
        assert_eq!(
            markdown_to_typst("# Hello"),
            format!("{PREAMBLE}#block(breakable: false)[\n= Hello\n\n]\n\n")
        );
    }

    #[test]
    fn heading_with_following_content() {
        // Heading should be grouped with following paragraph
        let result = markdown_to_typst("# Title\n\nSome text.");
        assert!(result.contains("#block(breakable: false)[\n= Title\n\nSome text.\n\n]\n\n"));
    }

    #[test]
    fn paragraph() {
        assert_eq!(
            markdown_to_typst("Hello world"),
            format!("{PREAMBLE}Hello world\n\n")
        );
    }

    #[test]
    fn bold_and_italic() {
        assert_eq!(
            markdown_to_typst("**bold**"),
            format!("{PREAMBLE}*bold*\n\n")
        );
        assert_eq!(
            markdown_to_typst("*italic*"),
            format!("{PREAMBLE}_italic_\n\n")
        );
        assert_eq!(
            markdown_to_typst("***both***"),
            format!("{PREAMBLE}_*both*_\n\n")
        );
    }

    #[test]
    fn inline_code() {
        assert_eq!(markdown_to_typst("`code`"), format!("{PREAMBLE}`code`\n\n"));
    }

    #[test]
    fn code_block() {
        assert_eq!(
            markdown_to_typst("```rust\nlet x = 1;\n```"),
            format!("{PREAMBLE}#block(breakable: false)[\n```rust\nlet x = 1;\n```\n]\n\n")
        );
    }

    #[test]
    fn unordered_list() {
        assert_eq!(
            markdown_to_typst("- one\n- two"),
            format!("{PREAMBLE}#block(breakable: false)[\n- one\n- two\n]\n\n")
        );
    }

    #[test]
    fn ordered_list() {
        assert_eq!(
            markdown_to_typst("1. one\n2. two"),
            format!("{PREAMBLE}#block(breakable: false)[\n+ one\n+ two\n]\n\n")
        );
    }

    #[test]
    fn hard_break() {
        assert_eq!(
            markdown_to_typst("line one  \nline two"),
            format!("{PREAMBLE}line one \\\nline two\n\n")
        );
    }

    #[test]
    fn escapes_special_chars() {
        assert_eq!(markdown_to_typst("a * b"), format!("{PREAMBLE}a \\* b\n\n"));
        assert_eq!(markdown_to_typst("a # b"), format!("{PREAMBLE}a \\# b\n\n"));
        assert_eq!(markdown_to_typst("a_b"), format!("{PREAMBLE}a\\_b\n\n"));
    }

    #[test]
    fn table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let expected = format!(
            "{PREAMBLE}#block(breakable: false)[\n#table(\n  columns: 2,\n  [*A*],\n  [*B*],\n  [1],\n  [2],\n)\n]\n\n"
        );
        assert_eq!(markdown_to_typst(md), expected);
    }

    #[test]
    fn horizontal_rule() {
        assert_eq!(
            markdown_to_typst("---"),
            format!("{PREAMBLE}#line(length: 100%)\n\n")
        );
    }
}
