use crate::block::{Block, List, Span};
use crate::config::Config;

/// Convert blocks to Typst markup
pub fn blocks_to_typst(blocks: &[Block], config: &Config) -> String {
    let mut out = String::new();

    // Set up paragraph settings to prevent widows/orphans
    out.push_str("#set par(linebreaks: \"optimized\")\n");

    // Font family
    if config.font.sans {
        out.push_str("#set text(font: \"Open Sans\")\n");
    }

    // Page numbers
    if config.page.numbers {
        out.push_str("#set page(numbering: \"1\")\n");
    }

    // Style links
    if config.links.underline {
        out.push_str(&format!(
            "#show link: it => underline(text(fill: rgb(\"{}\"), it))\n",
            config.links.color
        ));
    } else {
        out.push_str(&format!(
            "#show link: it => text(fill: rgb(\"{}\"), it)\n",
            config.links.color
        ));
    }

    out.push('\n');

    // Track if previous long section needs a break after it, and at what level
    let mut pending_end_break_level: Option<u8> = None;

    let mut i = 0;
    while i < blocks.len() {
        let block = &blocks[i];

        match block {
            Block::Heading { level, .. } => {
                // Check if this section is long enough to warrant a page break
                let section_lines = count_section_lines(blocks, i);
                let force_break = config
                    .layout
                    .break_if_lines_for_heading(*level)
                    .map(|threshold| section_lines >= threshold)
                    .unwrap_or(false);

                // Only process end breaks for headings at the same level or higher
                let should_check_end_break = pending_end_break_level
                    .map(|pending_level| *level <= pending_level)
                    .unwrap_or(false);

                if force_break {
                    // This section wants a break before it, which satisfies any pending end break
                    pending_end_break_level = None;
                    strip_trailing_rule(&mut out);
                    out.push_str("#pagebreak(weak: true)\n");
                } else if should_check_end_break {
                    // Insert pending end break from previous long section
                    strip_trailing_rule(&mut out);
                    out.push_str("#pagebreak(weak: true)\n");
                    pending_end_break_level = None;
                } else if let Some(min_space) = config.layout.min_space_for_heading(*level) {
                    // If min_space is configured, insert a non-breaking block to reserve space
                    // This causes Typst to move the heading to the next page if not enough room
                    out.push_str(&format!(
                        "#block(breakable: false, height: {})\n",
                        min_space
                    ));
                    out.push_str(&format!("#v(-{}, weak: true)\n", min_space));
                }

                // If this section is long, mark that we need a break after it
                if force_break {
                    pending_end_break_level = Some(*level);
                }

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

/// Remove trailing horizontal rule if present (redundant before page breaks)
fn strip_trailing_rule(out: &mut String) {
    let rule_str = "#line(length: 100%)\n\n";
    if out.ends_with(rule_str) {
        out.truncate(out.len() - rule_str.len());
    }
}

/// Count approximate lines in a section (from heading to next heading of same or higher level)
fn count_section_lines(blocks: &[Block], start: usize) -> usize {
    let start_level = match &blocks[start] {
        Block::Heading { level, .. } => *level,
        _ => return 0,
    };

    let mut lines = 0;

    for block in blocks.iter().skip(start + 1) {
        match block {
            Block::Heading { level, .. } if *level <= start_level => break,
            Block::Paragraph { content } => {
                // Estimate lines based on content length (~80 chars per line)
                let char_count: usize = content.iter().map(|s| span_char_count(s)).sum();
                lines += (char_count / 80).max(1);
            }
            Block::CodeBlock { content, .. } => {
                lines += content.lines().count();
            }
            Block::List(list) => {
                lines += count_list_lines(list);
            }
            Block::Table { headers, rows } => {
                lines += 1 + headers.len() + rows.len();
            }
            Block::Rule => {
                lines += 1;
            }
            Block::Heading { .. } => {
                lines += 2; // Heading + spacing
            }
            Block::PageBreak => {}
        }
    }

    lines
}

fn span_char_count(span: &Span) -> usize {
    match span {
        Span::Text(t) => t.len(),
        Span::Bold(inner) | Span::Italic(inner) => inner.iter().map(span_char_count).sum(),
        Span::Code(t) => t.len(),
        Span::Link { content, .. } => content.iter().map(span_char_count).sum(),
        Span::LineBreak => 1,
    }
}

fn count_list_lines(list: &List) -> usize {
    let mut lines = 0;
    for item in &list.items {
        lines += 1;
        if let Some(ref nested) = item.nested {
            lines += count_list_lines(nested);
        }
    }
    lines
}

fn emit_heading(block: &Block, out: &mut String) {
    if let Block::Heading { level, content } = block {
        for _ in 0..*level {
            out.push('=');
        }
        out.push(' ');
        spans_to_typst(content, out);
        // Add a label for internal linking based on heading text
        let label = heading_to_label(content);
        if !label.is_empty() {
            out.push(' ');
            out.push('<');
            out.push_str(&label);
            out.push('>');
        }
        out.push('\n');
        out.push('\n');
    }
}

/// Convert heading content to a URL-style label (lowercase, hyphens for spaces)
fn heading_to_label(spans: &[Span]) -> String {
    let mut text = String::new();
    collect_span_text(spans, &mut text);

    // Convert to lowercase, replace spaces with hyphens, keep only alphanumeric and hyphens
    text.chars()
        .map(|c| {
            if c.is_whitespace() {
                '-'
            } else {
                c.to_ascii_lowercase()
            }
        })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

/// Recursively collect plain text from spans
fn collect_span_text(spans: &[Span], out: &mut String) {
    for span in spans {
        match span {
            Span::Text(t) => out.push_str(t),
            Span::Bold(inner) | Span::Italic(inner) => collect_span_text(inner, out),
            Span::Code(t) => out.push_str(t),
            Span::Link { content, .. } => collect_span_text(content, out),
            Span::LineBreak => out.push(' '),
        }
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
        Block::PageBreak => {
            strip_trailing_rule(out);
            out.push_str("#pagebreak()\n\n");
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
        Span::Link { url, content } => {
            if let Some(anchor) = url.strip_prefix('#') {
                // Internal link to a heading
                out.push_str("#link(<");
                out.push_str(anchor);
                out.push_str(">)[");
                spans_to_typst(content, out);
                out.push(']');
            } else {
                // External link
                out.push_str("#link(\"");
                out.push_str(&url.replace('\\', "\\\\").replace('"', "\\\""));
                out.push_str("\")[");
                spans_to_typst(content, out);
                out.push(']');
            }
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
            format!("{PREAMBLE}#block(breakable: false)[\n= Hello <hello>\n\n]\n\n")
        );
    }

    #[test]
    fn heading_with_following_content() {
        // Heading should be grouped with following paragraph
        let result = markdown_to_typst("# Title\n\nSome text.");
        assert!(
            result.contains("#block(breakable: false)[\n= Title <title>\n\nSome text.\n\n]\n\n")
        );
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
