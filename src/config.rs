use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub links: LinksConfig,
    pub page: PageConfig,
    pub font: FontConfig,
    pub layout: LayoutConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LinksConfig {
    pub color: String,
    pub underline: bool,
}

impl Default for LinksConfig {
    fn default() -> Self {
        Self {
            color: "#1a4f8b".to_string(),
            underline: true,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct PageConfig {
    pub numbers: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct FontConfig {
    pub sans: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct LayoutConfig {
    pub h1_min_space: Option<String>,
    pub h2_min_space: Option<String>,
    pub h3_min_space: Option<String>,
    pub h4_min_space: Option<String>,
    pub h5_min_space: Option<String>,
    pub h6_min_space: Option<String>,
    pub h1_break_if_lines: Option<usize>,
    pub h2_break_if_lines: Option<usize>,
    pub h3_break_if_lines: Option<usize>,
    pub h4_break_if_lines: Option<usize>,
    pub h5_break_if_lines: Option<usize>,
    pub h6_break_if_lines: Option<usize>,
}

impl LayoutConfig {
    /// Get the minimum space requirement for a heading level.
    /// Returns None if no requirement is set.
    pub fn min_space_for_heading(&self, level: u8) -> Option<&str> {
        match level {
            1 => self.h1_min_space.as_deref(),
            2 => self.h2_min_space.as_deref(),
            3 => self.h3_min_space.as_deref(),
            4 => self.h4_min_space.as_deref(),
            5 => self.h5_min_space.as_deref(),
            6 => self.h6_min_space.as_deref(),
            _ => None,
        }
    }

    /// Get the line threshold for forcing a page break for a heading level.
    /// Returns None if no threshold is set.
    pub fn break_if_lines_for_heading(&self, level: u8) -> Option<usize> {
        match level {
            1 => self.h1_break_if_lines,
            2 => self.h2_break_if_lines,
            3 => self.h3_break_if_lines,
            4 => self.h4_break_if_lines,
            5 => self.h5_break_if_lines,
            6 => self.h6_break_if_lines,
            _ => None,
        }
    }
}

impl Config {
    /// Load config from a TOML file, or return defaults if not found.
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
}
