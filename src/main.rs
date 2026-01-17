use std::fs;
use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(name = "pdf")]
#[command(about = "Convert Markdown files to PDF")]
struct Cli {
    /// Input Markdown file
    input: PathBuf,

    /// Output PDF file (defaults to input name with .pdf extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    // Read input file
    let markdown = match fs::read_to_string(&cli.input) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading {}: {}", cli.input.display(), e);
            std::process::exit(1);
        }
    };

    // Convert markdown to PDF
    let pdf_bytes = match pdf::markdown_to_pdf(&markdown) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // Determine output path
    let output = cli
        .output
        .unwrap_or_else(|| cli.input.with_extension("pdf"));

    // Write PDF
    if let Err(e) = fs::write(&output, pdf_bytes) {
        eprintln!("Error writing {}: {}", output.display(), e);
        std::process::exit(1);
    }

    println!("Created {}", output.display());
}
