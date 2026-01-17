fn main() {
    let md = std::fs::read_to_string("/Users/korbin/pdf/Operations Guide.md").unwrap();
    let blocks = pdf::parse(&md);

    for (i, block) in blocks.iter().enumerate() {
        if let pdf::Block::Heading { level, content } = block {
            let lines = count_section_lines(&blocks, i, *level);
            let text: String = content.iter().map(|s| span_text(s)).collect();
            println!(
                "H{} {:30} -> {} lines",
                level,
                text.chars().take(30).collect::<String>(),
                lines
            );
        }
    }
}

fn count_section_lines(blocks: &[pdf::Block], start: usize, start_level: u8) -> usize {
    let mut lines = 0;
    for block in blocks.iter().skip(start + 1) {
        match block {
            pdf::Block::Heading { level, .. } if *level <= start_level => break,
            pdf::Block::Paragraph { content } => {
                let char_count: usize = content.iter().map(|s| span_char_count(s)).sum();
                lines += (char_count / 80).max(1);
            }
            pdf::Block::CodeBlock { content, .. } => {
                lines += content.lines().count();
            }
            pdf::Block::List(list) => {
                lines += count_list_lines(list);
            }
            pdf::Block::Table { headers, rows } => {
                lines += 1 + headers.len() + rows.len();
            }
            pdf::Block::Rule => {
                lines += 1;
            }
            pdf::Block::Heading { .. } => {
                lines += 2;
            }
            pdf::Block::PageBreak => {}
        }
    }
    lines
}

fn span_char_count(span: &pdf::Span) -> usize {
    match span {
        pdf::Span::Text(t) => t.len(),
        pdf::Span::Bold(inner) | pdf::Span::Italic(inner) => {
            inner.iter().map(span_char_count).sum()
        }
        pdf::Span::Code(t) => t.len(),
        pdf::Span::Link { content, .. } => content.iter().map(span_char_count).sum(),
        pdf::Span::LineBreak => 1,
    }
}

fn count_list_lines(list: &pdf::List) -> usize {
    let mut lines = 0;
    for item in &list.items {
        lines += 1;
        if let Some(ref nested) = item.nested {
            lines += count_list_lines(nested);
        }
    }
    lines
}

fn span_text(span: &pdf::Span) -> String {
    match span {
        pdf::Span::Text(t) => t.clone(),
        pdf::Span::Bold(inner) | pdf::Span::Italic(inner) => inner.iter().map(span_text).collect(),
        pdf::Span::Code(t) => t.clone(),
        pdf::Span::Link { content, .. } => content.iter().map(span_text).collect(),
        pdf::Span::LineBreak => " ".to_string(),
    }
}
