use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

const BANNED_DIRS: &[&str] = &[
    "Avalonia.Base",
    "BuildFiles",
    "Lidgren.Network",
    "ManagedHttpListener",
    "MSBuild",
    "NetSerializer",
    "OpenToolkit.GraphicsLibraryFramework",
    "Robust.Analyzers",
    "Robust.Benchmarks",
    "Robust.Client",
    "Robust.Client.Injectors",
    "Robust.Client.NameGenerator",
    "Robust.Client.WebView",
    "Robust.Docfx",
    "Robust.Generators",
    "Robust.Generators.UnitTesting",
    "Robust.LoaderApi",
    "Robust.Physics",
    "Robust.Server",
    "Robust.Shared",
    "Robust.Shared.Maths",
    "Robust.Shared.Scripting",
    "Robust.UnitTesting",
    "Tools",
    "XamlX",
    "cefglue",
    "rust",
];

const BANNED_FILENAMES: &[&str] = &[
    ".gitmodules",
    "RobustToolbox.sln",
    "RobustToolbox.sln.DotSettings",
    "nuget.config",
];

const BANNED_EXTENSIONS: &[&str] = &[
    "cs",
    "csproj",
    "csx",
    "dll.config",
    "fs",
    "fsproj",
    "props",
    "sln",
    "targets",
    "vb",
    "vbproj",
    "xaml",
];

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("verify-layout") => {
            if let Err(err) = verify_layout() {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("usage: cargo run -p xtask -- verify-layout");
            std::process::exit(2);
        }
    }
}

fn verify_layout() -> Result<(), String> {
    let root = env::current_dir().map_err(|err| format!("failed to read cwd: {err}"))?;
    let mut violations = Vec::new();
    visit(&root, &root, &mut violations).map_err(|err| format!("layout scan failed: {err}"))?;

    if violations.is_empty() {
        println!("layout ok: no legacy .NET or nested rust workspace files found");
        Ok(())
    } else {
        violations.sort();
        Err(format!(
            "layout violations:\n{}",
            violations
                .into_iter()
                .map(|path| format!("  - {}", path.display()))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

fn visit(root: &Path, dir: &Path, violations: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();

        if should_skip(&file_name) {
            continue;
        }

        let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();

        if path.is_dir() {
            if BANNED_DIRS.contains(&file_name.as_ref()) {
                violations.push(relative);
                continue;
            }

            visit(root, &path, violations)?;
            continue;
        }

        if BANNED_FILENAMES.contains(&file_name.as_ref()) || has_banned_extension(&path) {
            violations.push(relative);
        }
    }

    Ok(())
}

fn should_skip(file_name: &str) -> bool {
    matches!(file_name, ".git" | "target")
}

fn has_banned_extension(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    BANNED_EXTENSIONS.iter().any(|extension| {
        name.eq_ignore_ascii_case(&format!(".{extension}"))
            || name
                .to_ascii_lowercase()
                .ends_with(&format!(".{extension}"))
    })
}
