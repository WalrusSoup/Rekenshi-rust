//! Installer migration, without loading any game/native DLL.
#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
#[path = "../sample_config.rs"]
mod sample_config;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    #[serde(default)]
    discover: Option<bool>,
    #[serde(default)]
    plugins: Vec<Plugin>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plugin {
    file: String,
    enabled: bool,
}
const BUILTINS: [&str; 3] = [
    "rekenshi_shader_cache.dll",
    "rekenshi_graphics.dll",
    "rekenshi_color_grade.dll",
];
fn read(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    if text.len() > 65536 {
        return Err("configuration exceeds 64 KiB".into());
    }
    Ok(text)
}
fn migrate(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let plugins_path = root.join("plugins.toml");
    let original = read(&plugins_path)?;
    let mut plugins: Config = toml::from_str(&original)?;
    if plugins.schema_version != 1 {
        return Err("unsupported plugin configuration".into());
    }
    let enabled = |name: &str| {
        plugins
            .plugins
            .iter()
            .find(|p| {
                p.file.eq_ignore_ascii_case(name)
                    || (name == BUILTINS[0]
                        && p.file.eq_ignore_ascii_case("rekenshi_performance.dll"))
            })
            .map(|p| p.enabled)
    };
    let shader_path = root.join("shader-cache.toml");
    let performance = optional_table(&if shader_path.exists() {
        shader_path
    } else {
        root.join("performance.toml")
    })?;
    let mut values = BTreeMap::new();
    values.insert(
        "performance.shader_cache",
        toml::Value::Boolean(
            enabled(BUILTINS[0]).unwrap_or(true) && bool_value(&performance, "shader_cache", true),
        ),
    );
    #[derive(Serialize)]
    struct Settings<'a> {
        values: BTreeMap<&'a str, toml::Value>,
    }
    #[derive(Serialize)]
    struct Pack<'a> {
        schema_version: u32,
        settings: Settings<'a>,
    }
    let pack_path = root.join("modpack.toml");
    if !pack_path.exists() {
        write(
            &pack_path,
            &toml::to_string_pretty(&Pack {
                schema_version: 1,
                settings: Settings { values },
            })?,
        )?;
    }
    let previous_plugins = plugins.plugins.clone();
    let missing_discovery = plugins.discover.is_none();
    plugins.discover.get_or_insert(true);
    plugins.plugins.retain(|p| {
        !BUILTINS
            .iter()
            .any(|name| p.file.eq_ignore_ascii_case(name))
            && !p.file.eq_ignore_ascii_case("rekenshi_performance.dll")
    });
    // Discovery must not reactivate the retired compatibility DLLs left by
    // older installers. Preserve files, but disable their duplicate adapters.
    for file in BUILTINS.into_iter().chain(["rekenshi_performance.dll"]) {
        if root.join("plugins").join(file).is_file() {
            plugins.plugins.push(Plugin {
                file: file.into(),
                enabled: false,
            });
        }
    }
    if plugins.plugins != previous_plugins || missing_discovery {
        let backup = root.join("plugins.toml.before-modpack");
        if !backup.exists() {
            std::fs::write(&backup, &original)?;
        }
        write(&plugins_path, &toml::to_string_pretty(&plugins)?)?;
    }
    sample_config::migrate_settings(root)?;
    Ok(())
}
fn optional_table(path: &Path) -> Result<toml::Table, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(toml::Table::new());
    }
    Ok(toml::from_str(&read(path)?)?)
}
fn bool_value(table: &toml::Table, key: &str, default: bool) -> bool {
    table
        .get(key)
        .and_then(toml::Value::as_bool)
        .unwrap_or(default)
}
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    let pending = path.with_extension("toml.pending");
    std::fs::write(&pending, text)?;
    std::fs::rename(pending, path)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("usage: pack-config <RE_Kenshi_Rust folder>")?;
    migrate(Path::new(&root))?;
    println!(
        "Internal plugin settings migrated to modpack.toml; external plugin choices preserved."
    );
    Ok(())
}
