//! Cargo subcommand for the independent Rust plugin workflow.
#![forbid(unsafe_code)]

use std::{env, ffi::OsString, fs, path::PathBuf, process::Command};

const HELP: &str = "\
Create a Kenshi mod DLL:
    cargo rekenshi new plugin <name> [options]

Options:
    --kind callbacks|graphics   Starter type (default: callbacks)
    --path <directory>          Output folder (default: mods-src/<name>)
    --sdk-path <directory>      Path to crates/plugin-sdk in a host checkout
    --no-build                 Generate and format; skip the release build
    -h, --help                 Show this help
    -V, --version              Show helper version

Install:
    cargo install --path tools/cargo-rekenshi --locked
";

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Callbacks,
    Graphics,
}
#[derive(Debug)]
struct Options {
    name: String,
    kind: Kind,
    path: PathBuf,
    sdk: PathBuf,
    build: bool,
}
enum Action {
    Help,
    Version,
    Create(Options),
}
fn checkout() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Action, String> {
    let mut args = arguments.into_iter().peekable();
    // Cargo external subcommands receive their own name as the first argument;
    // the repository's cargo-run alias and direct binary invocation do not.
    if args.peek().is_some_and(|a| a == "rekenshi") {
        args.next();
    }
    match args.next().as_deref() {
        None => return Ok(Action::Help),
        Some(a) if a == "--help" || a == "-h" => return Ok(Action::Help),
        Some(a) if a == "--version" || a == "-V" => return Ok(Action::Version),
        Some(a) if a == "new" => {}
        Some(a) => return Err(format!("unknown command: {}", a.to_string_lossy())),
    }
    match args.next().as_deref() {
        Some(a) if a == "plugin" => {}
        Some(a) if a == "--help" || a == "-h" => return Ok(Action::Help),
        _ => return Err("expected: new plugin <name>".into()),
    }
    let name = match args.next() {
        Some(a) if a == "--help" || a == "-h" => return Ok(Action::Help),
        Some(a) => a.into_string().map_err(|_| "plugin name must be ASCII")?,
        None => return Err("missing plugin name".into()),
    };
    validate_name(&name)?;
    let mut options = Options {
        path: PathBuf::from("mods-src").join(&name),
        name,
        kind: Kind::Callbacks,
        sdk: checkout().join("crates/plugin-sdk"),
        build: true,
    };
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--kind") => {
                options.kind = match args.next().as_deref() {
                    Some(a) if a == "callbacks" => Kind::Callbacks,
                    Some(a) if a == "graphics" => Kind::Graphics,
                    _ => return Err("--kind requires callbacks or graphics".into()),
                };
            }
            Some("--path") => {
                options.path = args.next().ok_or("--path requires a directory")?.into();
            }
            Some("--sdk-path") => {
                options.sdk = args.next().ok_or("--sdk-path requires a directory")?.into();
            }
            Some("--no-build") => options.build = false,
            Some("--help" | "-h") => return Ok(Action::Help),
            _ => return Err(format!("unknown option: {}", arg.to_string_lossy())),
        }
    }
    Ok(Action::Create(options))
}
fn validate_name(name: &str) -> Result<(), String> {
    if !(2..=49).contains(&name.len())
        || !name.as_bytes()[0].is_ascii_lowercase()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(
            "name must be 2–49 lowercase ASCII letters/digits/hyphens, starting with a letter"
                .into(),
        );
    }
    // Rust crate identifiers are derived from package names. Cargo rejects
    // these words; catch them before touching the output folder.
    if [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
        "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final",
        "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen", "test",
        "std", "core", "alloc",
    ]
    .contains(&name)
    {
        return Err(format!("reserved crate name: {name}; try {name}-mod"));
    }
    Ok(())
}
fn scaffold(options: &Options) -> Result<PathBuf, String> {
    validate_name(&options.name)?;
    let sdk = options.sdk.canonicalize().map_err(|e| {
        format!(
            "SDK not found at {} ({e}); use --sdk-path <checkout>/crates/plugin-sdk",
            options.sdk.display()
        )
    })?;
    let sdk_manifest = fs::read_to_string(sdk.join("Cargo.toml"))
        .map_err(|e| format!("read SDK manifest: {e}"))?;
    if !sdk_manifest.lines().any(|l| {
        l.trim()
            .strip_prefix("name")
            .and_then(|rest| rest.trim().strip_prefix('='))
            .is_some_and(|value| value.trim() == "\"rekenshi-plugin-sdk\"")
    }) {
        return Err("--sdk-path must point to the rekenshi-plugin-sdk crate".into());
    }
    if let Some(parent) = options.path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("create parent directory: {e}"))?;
    }
    // Exclusive directory creation: never overwrite an existing authored mod.
    fs::create_dir(&options.path).map_err(|e| format!("create {}: {e}", options.path.display()))?;
    fs::create_dir(options.path.join("src")).map_err(|e| e.to_string())?;
    let dll = format!("{}.dll", options.name.replace('-', "_"));
    let implementation = match options.kind {
        Kind::Callbacks => include_str!("../templates/callbacks.rs"),
        Kind::Graphics => include_str!("../templates/graphics.rs"),
    };
    for (file, source) in [
        ("Cargo.toml", include_str!("../templates/Cargo.toml")),
        ("src/lib.rs", include_str!("../templates/lib.rs")),
        ("src/implementation.rs", implementation),
        ("README.md", include_str!("../templates/README.md")),
        (".gitignore", "/target/\n"),
    ] {
        fs::write(
            options.path.join(file),
            source
                .replace("__ID__", &options.name)
                .replace("__DLL__", &dll),
        )
        .map_err(|e| format!("write {file}: {e}"))?;
    }
    if options.kind == Kind::Graphics {
        fs::write(
            options.path.join("src/effect.hlsl"),
            include_str!("../templates/effect.hlsl"),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(sdk)
}
fn cargo(args: &[OsString]) -> Result<(), String> {
    let executable = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(executable)
        .args(args)
        .status()
        .map_err(|e| format!("could not launch cargo: {e}"))?;
    if !status.success() {
        return Err(format!(
            "cargo {} failed ({status}); generated files are retained for repair",
            args[0].to_string_lossy()
        ));
    }
    Ok(())
}
fn create(options: Options) -> Result<(), String> {
    let sdk = scaffold(&options)?;
    let manifest = options.path.join("Cargo.toml").into_os_string();
    cargo(&[
        "add".into(),
        "--manifest-path".into(),
        manifest.clone(),
        "rekenshi-plugin-sdk".into(),
        "--path".into(),
        sdk.into_os_string(),
    ])?;
    cargo(&["fmt".into(), "--manifest-path".into(), manifest.clone()])?;
    if options.build {
        cargo(&[
            "build".into(),
            "--release".into(),
            "--manifest-path".into(),
            manifest,
            "--target-dir".into(),
            options.path.join("target").into_os_string(),
        ])?;
    }
    // Absolute, readable paths without Windows canonicalize's extended prefix.
    let root = std::path::absolute(&options.path).map_err(|e| e.to_string())?;
    println!("Created {}", root.display());
    let dll = root
        .join("target/release")
        .join(format!("{}.dll", options.name.replace('-', "_")));
    if options.build {
        println!("Built {}", dll.display());
        println!(
            "Distribute this DLL; players copy it into Kenshi/RE_Kenshi_Rust/plugins and restart."
        );
    } else {
        println!(
            "Next: cargo build --release --manifest-path {}",
            root.join("Cargo.toml").display()
        );
    }
    Ok(())
}
fn run() -> Result<(), String> {
    match parse(env::args_os().skip(1))? {
        Action::Help => print!("{HELP}"),
        Action::Version => println!("cargo-rekenshi {}", env!("CARGO_PKG_VERSION")),
        Action::Create(options) => create(options)?,
    }
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}\nUse cargo rekenshi --help for usage.");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn action(args: &[&str]) -> Result<Action, String> {
        parse(args.iter().map(OsString::from))
    }
    #[test]
    fn cargo_and_direct_invocations_accept_the_same_options() {
        for prefix in [vec![], vec!["rekenshi"]] {
            let args = [
                prefix.as_slice(),
                &[
                    "new",
                    "plugin",
                    "my-look",
                    "--kind",
                    "graphics",
                    "--no-build",
                    "--path",
                    "custom",
                ],
            ]
            .concat();
            let Action::Create(options) = action(&args).unwrap() else {
                panic!("expected create")
            };
            assert_eq!(options.kind, Kind::Graphics);
            assert_eq!(options.path, PathBuf::from("custom"));
            assert!(!options.build);
        }
        assert!(action(&["new", "plugin", "my-mod", "--kind", "broken"]).is_err());
        assert!(action(&["new", "plugin", "my-mod", "--path"]).is_err());
        assert!(action(&["new", "plugin", "my-mod", "--wat"]).is_err());
        assert!(matches!(
            action(&["new", "plugin", "--help"]),
            Ok(Action::Help)
        ));
    }
    #[test]
    fn unsafe_and_reserved_names_are_rejected() {
        for name in [
            "../outside",
            "C:/temp",
            "MyMod",
            "my_mod",
            "a",
            "self",
            "std",
            "gen",
            "mod;echo",
        ] {
            assert!(validate_name(name).is_err(), "{name}");
        }
        assert!(validate_name("my-new-mod2").is_ok());
    }
    #[test]
    fn missing_sdk_and_existing_projects_are_never_overwritten() {
        let root = env::temp_dir().join(format!(
            "rekenshi-helper-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut options = Options {
            name: "my-look".into(),
            kind: Kind::Graphics,
            path: root.join("my-look"),
            sdk: root.join("missing"),
            build: false,
        };
        assert!(scaffold(&options).is_err());
        assert!(!root.exists());
        options.sdk = checkout().join("crates/plugin-sdk");
        scaffold(&options).unwrap();
        let authored = options.path.join("src/implementation.rs");
        fs::write(&authored, "user's implementation").unwrap();
        assert!(scaffold(&options).is_err());
        assert_eq!(
            fs::read_to_string(authored).unwrap(),
            "user's implementation"
        );
        let canonical = root.canonicalize().unwrap();
        assert!(canonical.starts_with(env::temp_dir().canonicalize().unwrap()));
        fs::remove_dir_all(canonical).unwrap();
    }
}
