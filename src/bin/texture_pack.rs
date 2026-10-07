//! Lossless SIC converter. Sidecars retain the original TGA for fallback.
#![forbid(unsafe_code)]
use rekenshi_sic::{self as sic, SourceStamp};
use std::{io::Write, path::PathBuf};
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(2..=3).contains(&args.len()) {
        return Err("Usage: texture-pack INPUT OUTPUT.sic|--sidecar [LEVEL=3]".into());
    }
    let input = PathBuf::from(&args[0]);
    let sidecar = args[1] == "--sidecar";
    if sidecar
        && !input
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tga"))
    {
        return Err("Automatic sidecars support TGA sources".into());
    }
    let output = if sidecar {
        PathBuf::from(format!("{}.sic", input.display()))
    } else {
        PathBuf::from(&args[1])
    };
    let level = args
        .get(2)
        .map(|value| value.to_string_lossy().parse::<i32>())
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or(3);
    let before = SourceStamp::capture(&input)?;
    let image = sic::load_image(&input)?;
    if sidecar {
        sic::tga(&image)?;
    }
    let bytes = sic::encode(&image, level)?;
    let decoded = sic::decode(&bytes)?;
    if decoded.pixels != image.pixels || before != SourceStamp::capture(&input)? {
        return Err("Source changed during conversion or verification failed".into());
    }
    let source_len = std::fs::metadata(&input).map_err(|e| e.to_string())?.len();
    if sidecar && bytes.len() as u64 >= source_len {
        return Err("SIC is not smaller than the source; no sidecar written".into());
    }
    write_new(&output, &bytes)?;
    if sidecar {
        write_new(
            &PathBuf::from(format!("{}.toml", output.display())),
            toml::to_string(&before)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )?;
    }
    println!(
        "{}: {}x{}, {} -> {} bytes; pixels verified",
        output.display(),
        image.width,
        image.height,
        source_len,
        bytes.len()
    );
    Ok(())
}
fn write_new(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())
}
