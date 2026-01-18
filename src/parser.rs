use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::block::{Block, List, ListItem, Span};

/// Strip YAML frontmatter from the beginning of markdown content
fn strip_frontmatter(markdown: &str) -> &str {
    if !markdown.starts_with("---") {
        return markdown;
    }
    // Find the closing ---
    if let Some(end) = markdown[3..].find("\n---") {
        // Skip past the closing --- and any trailing newline
        let after_frontmatter = &markdown[3 + end + 4..];
        after_frontmatter.trim_start_matches('\n')
    } else {
        markdown
    }
}

/// Parse markdown text into a list of blocks
pub fn parse(markdown: &str) -> Vec<Block> {
    let markdown = strip_frontmatter(markdown);
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(markdown, options);
    let mut blocks = Vec::new();
    let mut state = ParseState::default();

    for event in parser {
        process_event(event, &mut state, &mut blocks);
    }

    blocks
}

#[derive(Default)]
struct ParseState {
    // Current inline content being built
    spans: Vec<Span>,
    // Stack for nested formatting (bold, italic)
    format_stack: Vec<FormatKind>,
    // Nested span buffers for formatting
    span_stack: Vec<Vec<Span>>,

    // Current heading level (if in a heading)
    heading_level: Option<u8>,

    // Code block state
    in_code_block: bool,
    code_language: Option<String>,
    code_content: String,

    // Link state
    link_url: Option<String>,

    // List state
    list_stack: Vec<ListBuilder>,

    // Table state
    in_table: bool,
    table_headers: Vec<Vec<Span>>,
    table_rows: Vec<Vec<Vec<Span>>>,
    current_row: Vec<Vec<Span>>,
    in_table_head: bool,
}

#[derive(Clone, Copy)]
enum FormatKind {
    Bold,
    Italic,
}

struct ListBuilder {
    ordered: bool,
    items: Vec<ListItem>,
    current_item_spans: Vec<Span>,
    current_item_checked: Option<bool>,
}

fn process_event(event: Event, state: &mut ParseState, blocks: &mut Vec<Block>) {
    match event {
        // Headings
        Event::Start(Tag::Heading { level, .. }) => {
            state.heading_level = Some(heading_level_to_u8(level));
        }
        Event::End(TagEnd::Heading(_)) => {
            if let Some(level) = state.heading_level.take() {
                let content = std::mem::take(&mut state.spans);
                blocks.push(Block::Heading { level, content });
            }
        }

        // Paragraphs
        Event::Start(Tag::Paragraph) => {}
        Event::End(TagEnd::Paragraph) => {
            let content = std::mem::take(&mut state.spans);
            if !content.is_empty() {
                // Check for manual page break marker
                if content.len() == 1 {
                    if let Span::Text(text) = &content[0] {
                        if text.trim() == "---pagebreak---" {
                            blocks.push(Block::PageBreak);
                            return;
                        }
                    }
                }
                // If we're in a list item, add to that instead
                if let Some(list) = state.list_stack.last_mut() {
                    list.current_item_spans.extend(content);
                } else if state.in_table {
                    // Ignore paragraphs in tables, handled by cell
                } else {
                    blocks.push(Block::Paragraph { content });
                }
            }
        }

        // Text content
        Event::Text(text) => {
            if state.in_code_block {
                state.code_content.push_str(&text);
            } else {
                state.spans.push(Span::Text(text.into_string()));
            }
        }

        // Inline code
        Event::Code(code) => {
            state.spans.push(Span::Code(code.into_string()));
        }

        // Bold
        Event::Start(Tag::Strong) => {
            state.format_stack.push(FormatKind::Bold);
            state.span_stack.push(std::mem::take(&mut state.spans));
        }
        Event::End(TagEnd::Strong) => {
            state.format_stack.pop();
            let bold_content = std::mem::take(&mut state.spans);
            if let Some(mut parent) = state.span_stack.pop() {
                parent.push(Span::Bold(bold_content));
                state.spans = parent;
            }
        }

        // Italic
        Event::Start(Tag::Emphasis) => {
            state.format_stack.push(FormatKind::Italic);
            state.span_stack.push(std::mem::take(&mut state.spans));
        }
        Event::End(TagEnd::Emphasis) => {
            state.format_stack.pop();
            let italic_content = std::mem::take(&mut state.spans);
            if let Some(mut parent) = state.span_stack.pop() {
                parent.push(Span::Italic(italic_content));
                state.spans = parent;
            }
        }

        // Links
        Event::Start(Tag::Link { dest_url, .. }) => {
            state.link_url = Some(dest_url.into_string());
            state.span_stack.push(std::mem::take(&mut state.spans));
        }
        Event::End(TagEnd::Link) => {
            let link_content = std::mem::take(&mut state.spans);
            if let Some(mut parent) = state.span_stack.pop() {
                if let Some(url) = state.link_url.take() {
                    parent.push(Span::Link {
                        url,
                        content: link_content,
                    });
                }
                state.spans = parent;
            }
        }

        // Code blocks
        Event::Start(Tag::CodeBlock(kind)) => {
            state.in_code_block = true;
            state.code_language = match kind {
                pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                    let lang = lang.into_string();
                    if lang.is_empty() { None } else { Some(lang) }
                }
                pulldown_cmark::CodeBlockKind::Indented => None,
            };
            state.code_content.clear();
        }
        Event::End(TagEnd::CodeBlock) => {
            state.in_code_block = false;
            let content = std::mem::take(&mut state.code_content);
            let language = state.code_language.take();
            blocks.push(Block::CodeBlock { language, content });
        }

        // Lists
        Event::Start(Tag::List(first_item)) => {
            state.list_stack.push(ListBuilder {
                ordered: first_item.is_some(),
                items: Vec::new(),
                current_item_spans: Vec::new(),
                current_item_checked: None,
            });
        }
        Event::End(TagEnd::List(_)) => {
            if let Some(list_builder) = state.list_stack.pop() {
                let list = List {
                    ordered: list_builder.ordered,
                    items: list_builder.items,
                };
                // If there's a parent list, this is nested
                if let Some(parent) = state.list_stack.last_mut() {
                    if let Some(last_item) = parent.items.last_mut() {
                        last_item.nested = Some(Box::new(list));
                    }
                } else {
                    blocks.push(Block::List(list));
                }
            }
        }

        Event::Start(Tag::Item) => {
            if let Some(list) = state.list_stack.last_mut() {
                list.current_item_spans.clear();
                list.current_item_checked = None;
            }
        }
        Event::End(TagEnd::Item) => {
            // Collect any remaining spans
            let remaining = std::mem::take(&mut state.spans);

            if let Some(list) = state.list_stack.last_mut() {
                list.current_item_spans.extend(remaining);
                let content = std::mem::take(&mut list.current_item_spans);
                let checked = list.current_item_checked.take();
                list.items.push(ListItem {
                    content,
                    nested: None,
                    checked,
                });
            }
        }

        // Task list checkboxes
        Event::TaskListMarker(checked) => {
            if let Some(list) = state.list_stack.last_mut() {
                list.current_item_checked = Some(checked);
            }
        }

        // Tables
        Event::Start(Tag::Table(_)) => {
            state.in_table = true;
            state.table_headers.clear();
            state.table_rows.clear();
        }
        Event::End(TagEnd::Table) => {
            state.in_table = false;
            let headers = std::mem::take(&mut state.table_headers);
            let rows = std::mem::take(&mut state.table_rows);
            blocks.push(Block::Table { headers, rows });
        }

        Event::Start(Tag::TableHead) => {
            state.in_table_head = true;
            state.current_row.clear();
        }
        Event::End(TagEnd::TableHead) => {
            state.in_table_head = false;
            state.table_headers = std::mem::take(&mut state.current_row);
        }

        Event::Start(Tag::TableRow) => {
            state.current_row.clear();
        }
        Event::End(TagEnd::TableRow) => {
            if !state.in_table_head {
                let row = std::mem::take(&mut state.current_row);
                state.table_rows.push(row);
            }
        }

        Event::Start(Tag::TableCell) => {
            state.spans.clear();
        }
        Event::End(TagEnd::TableCell) => {
            let cell_content = std::mem::take(&mut state.spans);
            state.current_row.push(cell_content);
        }

        // Horizontal rule
        Event::Rule => {
            blocks.push(Block::Rule);
        }

        // Soft/hard breaks
        Event::SoftBreak => {
            state.spans.push(Span::Text(" ".to_string()));
        }
        Event::HardBreak => {
            state.spans.push(Span::LineBreak);
        }

        // Ignore other events
        _ => {}
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
