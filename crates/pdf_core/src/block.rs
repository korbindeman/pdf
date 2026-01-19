/// Inline text spans with formatting
#[derive(Debug, Clone)]
pub enum Span {
    Text(String),
    Bold(Vec<Span>),
    Italic(Vec<Span>),
    Code(String),
    Link { url: String, content: Vec<Span> },
    LineBreak,
}

/// A single list item, which can contain nested content
#[derive(Debug, Clone)]
pub struct ListItem {
    pub content: Vec<Span>,
    pub nested: Option<Box<List>>,
    /// For task lists: None = not a task, Some(false) = unchecked, Some(true) = checked
    pub checked: Option<bool>,
}

/// A list (ordered or unordered)
#[derive(Debug, Clone)]
pub struct List {
    pub ordered: bool,
    pub items: Vec<ListItem>,
}

/// Block-level elements parsed from Markdown
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Span>,
    },
    Paragraph {
        content: Vec<Span>,
    },
    CodeBlock {
        #[allow(dead_code)] // Reserved for future syntax highlighting
        language: Option<String>,
        content: String,
    },
    List(List),
    Table {
        headers: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
    },
    Rule,
    PageBreak,
}
