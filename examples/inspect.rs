fn main() {
    let args: Vec<String> = std::env::args().collect();
    let md = if args.len() > 1 {
        std::fs::read_to_string(&args[1]).expect("Failed to read file")
    } else {
        "# Overview\n\n[Link to overview](#overview)".to_string()
    };

    // Load config from current directory
    let config = pdf::Config::load(std::path::Path::new("config.toml"));
    println!("{}", pdf::markdown_to_typst_with_config(&md, &config));
}
