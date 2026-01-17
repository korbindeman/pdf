use std::fs;

fn main() {
    // Validate default config at compile time
    let config_path = "src/default_config.toml";
    println!("cargo:rerun-if-changed={}", config_path);

    let content = fs::read_to_string(config_path).expect("Failed to read default_config.toml");

    // Try to parse it as TOML to catch syntax errors
    if let Err(e) = content.parse::<toml::Table>() {
        panic!("Invalid default_config.toml: {}", e);
    }
}
