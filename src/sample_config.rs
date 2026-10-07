//! Legacy effect settings migration shared by the host and offline installer.
#![forbid(unsafe_code)]
use std::path::Path;

/// Move the frozen, former built-in effect keys to their owning DLL files.
/// The retired samples.toml selector does not participate in discovery.
pub(crate) fn migrate_settings(root: &Path) -> Result<(), String> {
    let path = root.join("modpack.toml");
    if !path.is_file() {
        return Ok(());
    }
    let original = read(&path)?;
    let mut pack: toml::Table = toml::from_str(&original).map_err(|e| e.to_string())?;
    if pack.get("schema_version").and_then(toml::Value::as_integer) != Some(1) {
        return Err("unsupported modpack schema".into());
    }
    let Some(values) = pack
        .get_mut("settings")
        .and_then(toml::Value::as_table_mut)
        .and_then(|s| s.get_mut("values"))
        .and_then(toml::Value::as_table_mut)
    else {
        return Ok(());
    };
    let mut changed = false;
    // Frozen legacy keys: new DLL mods persist their own declarations in
    // settings/<id>.toml and need no changes to this host migration.
    for (id, keys) in [
        (
            "color-grade",
            &[
                "graphics.color_grade",
                "graphics.contrast",
                "graphics.saturation",
                "graphics.brightness",
            ][..],
        ),
        (
            "rain-refinement",
            &[
                "weather.rain_refinement",
                "weather.rain_visibility",
                "weather.rain_size",
            ][..],
        ),
        (
            "kenshi2-style",
            &[
                "samples.kenshi2_style",
                "samples.kenshi2_style.ink",
                "samples.kenshi2_style.ink_width",
                "samples.kenshi2_style.contrast",
                "samples.kenshi2_style.saturation",
                "samples.kenshi2_style.exposure",
                "samples.kenshi2_style.palette",
                "samples.kenshi2_style.palette_levels",
                "samples.kenshi2_style.sharpen",
                "samples.kenshi2_style.warmth",
            ][..],
        ),
    ] {
        let mut migrated = toml::Table::new();
        for key in keys {
            if let Some(value) = values.remove(*key) {
                migrated.insert((*key).into(), value);
                changed = true;
            }
        }
        if migrated.is_empty() {
            continue;
        }
        let folder = root.join("settings");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let target = folder.join(format!("{id}.toml"));
        let mut owned: toml::Table = if target.exists() {
            toml::from_str(&read(&target)?).map_err(|e| e.to_string())?
        } else {
            toml::from_str("schema_version=1\n[settings.values]").map_err(|e| e.to_string())?
        };
        if owned
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            != Some(1)
        {
            return Err("unsupported DLL settings schema".into());
        }
        let owned_values = owned
            .get_mut("settings")
            .and_then(toml::Value::as_table_mut)
            .and_then(|s| s.get_mut("values"))
            .and_then(toml::Value::as_table_mut)
            .ok_or("invalid DLL settings")?;
        for (key, value) in migrated {
            owned_values.entry(key).or_insert(value);
        }
        let pending = target.with_extension("toml.pending");
        std::fs::write(
            &pending,
            toml::to_string_pretty(&owned).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(pending, target).map_err(|e| e.to_string())?;
    }
    if !changed {
        return Ok(());
    }
    let backup = root.join("modpack.toml.before-dll-plugins");
    // Preserve the first pre-migration snapshot without overwriting it.
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(backup)
    {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(original.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    let text = toml::to_string_pretty(&pack).map_err(|e| e.to_string())?;
    let pending = path.with_extension("toml.pending");
    std::fs::write(&pending, text).map_err(|e| e.to_string())?;
    std::fs::rename(pending, path).map_err(|e| e.to_string())
}
fn read(path: &Path) -> Result<String, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 65536 {
        return Err("configuration exceeds 64 KiB".into());
    }
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moves_effect_settings_and_preserves_original_once() {
        let root = std::env::temp_dir().join(format!(
            "rekenshi-samples-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let original = "schema_version=1\n[settings.values]\n\"performance.shader_cache\"=false\n\"graphics.color_grade\"=true\n\"weather.rain_visibility\"=0.7\n\"custom.setting\"=true\n";
        std::fs::write(root.join("modpack.toml"), original).unwrap();
        std::fs::write(
            root.join("samples.toml"),
            "schema_version=1\nplugins=['rain-refinement']",
        )
        .unwrap();
        migrate_settings(&root).unwrap();
        let migrated = read(&root.join("modpack.toml")).unwrap();
        assert!(!migrated.contains("graphics.color_grade"));
        assert!(!migrated.contains("weather.rain_visibility"));
        assert!(
            read(&root.join("settings/rain-refinement.toml"))
                .unwrap()
                .contains("0.7")
        );
        assert!(migrated.contains("\"performance.shader_cache\" = false"));
        assert!(migrated.contains("custom.setting"));
        migrate_settings(&root).unwrap();
        assert_eq!(read(&root.join("modpack.toml")).unwrap(), migrated);
        assert_eq!(
            read(&root.join("modpack.toml.before-dll-plugins")).unwrap(),
            original
        );
        std::fs::write(
            root.join("samples.toml"),
            "schema_version=1\nplugins=['oops']",
        )
        .unwrap();
        // A retired selector, even with unknown IDs, cannot block new DLL mods.
        migrate_settings(&root).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
