use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    time::{Duration, Instant},
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
        Some("architecture-map") => {
            if let Err(err) = architecture_map(args.collect()) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some("michisuji-rb2011-profile") => {
            if let Err(err) = michisuji_rb2011_profile(args.collect()) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some("web-smoke") => {
            if let Err(err) = web_smoke(args.collect()) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some("web-bench") => {
            if let Err(err) = web_bench(args.collect()) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some("verify-architecture") => {
            if let Err(err) = verify_architecture() {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Some("verify-layout") => {
            if let Err(err) = verify_layout() {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!(
                "usage: cargo run -p xtask -- <architecture-map|michisuji-rb2011-profile|web-smoke|web-bench|verify-architecture|verify-layout>"
            );
            std::process::exit(2);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CrateInfo {
    name: String,
    path: PathBuf,
    modules: Vec<String>,
    internal_dependencies: Vec<String>,
}

fn architecture_map(args: Vec<String>) -> Result<(), String> {
    let root = env::current_dir().map_err(|err| format!("failed to read cwd: {err}"))?;
    let crates = collect_crates(&root).map_err(|err| format!("architecture scan failed: {err}"))?;
    if args.iter().any(|arg| arg == "--dot") {
        print_architecture_dot(&crates);
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--json") {
        print_architecture_json(&crates);
        return Ok(());
    }

    println!("Omoikane architecture map");
    println!();
    println!("crates:");
    for krate in crates.values() {
        println!("  - {} ({})", krate.name, krate.path.display());
        println!("    modules: {}", display_list(&krate.modules));
        println!(
            "    internal deps: {}",
            display_list(&krate.internal_dependencies)
        );
    }

    println!();
    println!("dependency edges:");
    let mut has_edges = false;
    for krate in crates.values() {
        for dependency in &krate.internal_dependencies {
            has_edges = true;
            println!("  {} -> {dependency}", krate.name);
        }
    }
    if !has_edges {
        println!("  none");
    }

    Ok(())
}

fn print_architecture_dot(crates: &BTreeMap<String, CrateInfo>) {
    println!("digraph omoikane {{");
    println!("  rankdir=LR;");
    for krate in crates.values() {
        println!("  \"{}\";", krate.name);
    }
    for krate in crates.values() {
        for dependency in &krate.internal_dependencies {
            println!("  \"{}\" -> \"{}\";", krate.name, dependency);
        }
    }
    println!("}}");
}

fn print_architecture_json(crates: &BTreeMap<String, CrateInfo>) {
    let mut out = String::from("{\"crates\":[");
    for (index, krate) in crates.values().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field(&mut out, "name", &krate.name, true);
        push_json_field(&mut out, "path", &krate.path.display().to_string(), false);
        push_json_array(&mut out, "modules", &krate.modules, false);
        push_json_array(
            &mut out,
            "internal_dependencies",
            &krate.internal_dependencies,
            false,
        );
        out.push('}');
    }
    out.push_str("]}");
    println!("{out}");
}

fn verify_architecture() -> Result<(), String> {
    let root = env::current_dir().map_err(|err| format!("failed to read cwd: {err}"))?;
    let crates = collect_crates(&root).map_err(|err| format!("architecture scan failed: {err}"))?;
    let violations = architecture_violations(&crates);

    if violations.is_empty() {
        println!("architecture ok: crate dependency boundaries are clean");
        Ok(())
    } else {
        Err(format!(
            "architecture violations:\n{}",
            violations
                .into_iter()
                .map(|violation| format!("  - {violation}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

fn collect_crates(root: &Path) -> io::Result<BTreeMap<String, CrateInfo>> {
    let mut crate_paths = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.join("Cargo.toml").is_file() {
            crate_paths.push(path);
        }
    }
    crate_paths.sort();

    let crate_names = crate_paths
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    let mut crates = BTreeMap::new();
    for path in crate_paths {
        let Some(name) = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        let manifest = fs::read_to_string(path.join("Cargo.toml"))?;
        let modules = collect_root_modules(&path)?;
        let internal_dependencies = collect_internal_dependencies(&manifest, &crate_names, &name);
        crates.insert(
            name.clone(),
            CrateInfo {
                name,
                path: path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                modules,
                internal_dependencies,
            },
        );
    }

    Ok(crates)
}

fn collect_root_modules(crate_path: &Path) -> io::Result<Vec<String>> {
    let root_module = crate_path.join("src/lib.rs");
    let root_module = if root_module.is_file() {
        root_module
    } else {
        crate_path.join("src/main.rs")
    };

    if !root_module.is_file() {
        return Ok(Vec::new());
    }

    let source = fs::read_to_string(root_module)?;
    let mut modules = source
        .lines()
        .filter_map(parse_module_decl)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    modules.sort();
    Ok(modules)
}

fn collect_internal_dependencies(
    manifest: &str,
    crate_names: &BTreeSet<String>,
    own_name: &str,
) -> Vec<String> {
    crate_names
        .iter()
        .filter(|name| name.as_str() != own_name)
        .filter(|name| {
            manifest
                .lines()
                .any(|line| parse_dependency_name(line).as_deref() == Some(name.as_str()))
        })
        .cloned()
        .collect()
}

fn parse_module_decl(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.ends_with(';') {
        return None;
    }
    let declaration = line
        .strip_prefix("pub mod ")
        .or_else(|| line.strip_prefix("mod "))?;
    declaration
        .trim_end_matches(';')
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

fn parse_dependency_name(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with('[') || line.starts_with('#') {
        return None;
    }
    let (name, value) = line.split_once('=')?;
    value.contains("path").then(|| name.trim().to_owned())
}

fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

fn push_json_field(out: &mut String, name: &str, value: &str, first: bool) {
    if !first {
        out.push(',');
    }
    push_json_string(out, name);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_array(out: &mut String, name: &str, values: &[String], first: bool) {
    if !first {
        out.push(',');
    }
    push_json_string(out, name);
    out.push_str(":[");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, value);
    }
    out.push(']');
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn architecture_violations(crates: &BTreeMap<String, CrateInfo>) -> Vec<String> {
    let mut violations = Vec::new();

    for rule in architecture_rules() {
        if has_internal_dependency(crates, rule.source, rule.target) {
            violations.push(format!(
                "{} must not depend on {} ({})",
                rule.source, rule.target, rule.reason
            ));
        }
    }

    if let Some(hikari) = crates.get("hikari") {
        let unexpected_deps = hikari
            .internal_dependencies
            .iter()
            .filter(|dependency| dependency.as_str() != "keisan")
            .cloned()
            .collect::<Vec<_>>();
        if !unexpected_deps.is_empty() {
            violations.push(format!(
                "hikari has unexpected internal deps before extract integration: {} (only keisan is allowed)",
                display_list(&unexpected_deps),
            ));
        }
    }

    for cycle in dependency_cycles(crates) {
        violations.push(format!("dependency cycle detected: {}", cycle.join(" -> ")));
    }

    violations.sort();
    violations.dedup();
    violations
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArchitectureRule {
    source: &'static str,
    target: &'static str,
    reason: &'static str,
}

fn architecture_rules() -> &'static [ArchitectureRule] {
    &[
        ArchitectureRule {
            source: "keisan",
            target: "hikari",
            reason: "math must stay renderer-agnostic",
        },
        ArchitectureRule {
            source: "jikan",
            target: "hikari",
            reason: "time and loop primitives must stay renderer-agnostic",
        },
        ArchitectureRule {
            source: "butsuri",
            target: "hikari",
            reason: "physics must not know presentation",
        },
        ArchitectureRule {
            source: "sekai",
            target: "hikari",
            reason: "shared ECS must not know presentation",
        },
        ArchitectureRule {
            source: "daikoku",
            target: "hikari",
            reason: "authoritative server must stay headless",
        },
        ArchitectureRule {
            source: "daikoku",
            target: "shinobi",
            reason: "server must not depend on client code",
        },
        ArchitectureRule {
            source: "daikoku",
            target: "omoikane_web",
            reason: "authoritative server must not depend on web edge adapters",
        },
        ArchitectureRule {
            source: "daikoku",
            target: "omoikane_control",
            reason: "authoritative server must not depend on launch/control adapters",
        },
        ArchitectureRule {
            source: "omoikane_control",
            target: "omoikane_web",
            reason: "network control manifests must stay below web adapters",
        },
        ArchitectureRule {
            source: "sekai",
            target: "daikoku",
            reason: "shared state must not depend on server systems",
        },
        ArchitectureRule {
            source: "sekai",
            target: "shinobi",
            reason: "shared state must not depend on client systems",
        },
        ArchitectureRule {
            source: "butsuri",
            target: "sekai",
            reason: "physics primitives must stay below ECS",
        },
        ArchitectureRule {
            source: "jikan",
            target: "butsuri",
            reason: "time primitives must stay below physics",
        },
    ]
}

fn has_internal_dependency(
    crates: &BTreeMap<String, CrateInfo>,
    source: &str,
    target: &str,
) -> bool {
    crates
        .get(source)
        .is_some_and(|krate| krate.internal_dependencies.iter().any(|dep| dep == target))
}

fn dependency_cycles(crates: &BTreeMap<String, CrateInfo>) -> Vec<Vec<String>> {
    let mut cycles = BTreeSet::new();
    for name in crates.keys() {
        let mut stack = Vec::new();
        visit_dependency_cycles(name, name, crates, &mut stack, &mut cycles);
    }
    cycles.into_iter().collect()
}

fn visit_dependency_cycles(
    start: &str,
    current: &str,
    crates: &BTreeMap<String, CrateInfo>,
    stack: &mut Vec<String>,
    cycles: &mut BTreeSet<Vec<String>>,
) {
    if stack.iter().any(|item| item == current) {
        return;
    }

    stack.push(current.to_string());
    let Some(krate) = crates.get(current) else {
        stack.pop();
        return;
    };

    for dependency in &krate.internal_dependencies {
        if dependency == start {
            let mut cycle = stack.clone();
            cycle.push(start.to_string());
            cycles.insert(canonical_cycle(cycle));
        } else {
            visit_dependency_cycles(start, dependency, crates, stack, cycles);
        }
    }

    stack.pop();
}

fn canonical_cycle(mut cycle: Vec<String>) -> Vec<String> {
    let Some(last) = cycle.pop() else {
        return cycle;
    };
    if cycle.is_empty() {
        return vec![last];
    }

    let min_index = cycle
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| left.cmp(right))
        .map(|(index, _)| index)
        .unwrap_or(0);
    cycle.rotate_left(min_index);
    cycle.push(cycle[0].clone());
    cycle
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EndpointTarget {
    host: String,
    port: u16,
}

impl Default for EndpointTarget {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

fn michisuji_rb2011_profile(args: Vec<String>) -> Result<(), String> {
    let mut server = "127.0.0.1".to_string();
    let mut port = 8080;
    let mut out_path = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" => server = next_xtask_value(&mut args, "--server")?,
            "--port" => port = parse_xtask_u16(&next_xtask_value(&mut args, "--port")?, "--port")?,
            "--out" => out_path = Some(PathBuf::from(next_xtask_value(&mut args, "--out")?)),
            unknown => {
                return Err(format!(
                    "unknown michisuji-rb2011-profile argument: {unknown}"
                ));
            }
        }
    }

    let script = omoikane_web::MichisujiRackProfile::rb2011(server, port)
        .render_script()
        .map_err(|err| format!("failed to render Michisuji RB2011 profile: {err:?}"))?;
    if let Some(path) = out_path {
        fs::write(&path, script)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("michisuji profile written: {}", path.display());
    } else {
        print!("{script}");
    }
    Ok(())
}

fn web_smoke(args: Vec<String>) -> Result<(), String> {
    let target = parse_endpoint_target(args)?;
    let checks = [
        ("/health", "\"ok\":true"),
        ("/status", "\"state\":\"running\""),
        ("/metrics", "omoikane_http_requests_total"),
        ("/database/status", "\"status\""),
        ("/grakane/dashboard.json", "Omoikane Grakane Runtime"),
        ("/network/publication", "\"subservers\""),
        ("/servers", "\"id\":\"site\""),
        (
            "/network/dns/routeros.rsc",
            "Omoikane public DNS publication plan",
        ),
        (
            "/network/dns/junos.set",
            "Omoikane public DNS publication plan",
        ),
        ("/vps/reality-blueprint", "VLESS Reality"),
        ("/automation/mamori", "omoikane-rack-acceptance"),
        ("/automation/kaminari/tools", "kaminari.interface_terse"),
        (
            "/automation/michisuji/rb2011.rsc",
            "Omoikane Michisuji rack profile",
        ),
    ];

    for (path, expected) in checks {
        let (status, body, elapsed) = http_get(&target, path)?;
        if status != 200 {
            return Err(format!("{path} returned HTTP {status}"));
        }
        if !body.contains(expected) {
            return Err(format!(
                "{path} did not contain expected fragment: {expected}"
            ));
        }
        println!("{path} ok in {} ms", elapsed.as_millis());
    }
    Ok(())
}

fn web_bench(args: Vec<String>) -> Result<(), String> {
    let mut requests = 64_u32;
    let mut path = "/status".to_string();
    let mut target_args = Vec::new();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--requests" => {
                requests =
                    parse_xtask_u32(&next_xtask_value(&mut args, "--requests")?, "--requests")?
            }
            "--path" => path = next_xtask_value(&mut args, "--path")?,
            "--host" | "--port" => {
                let value = next_xtask_value(&mut args, &arg)?;
                target_args.push(arg);
                target_args.push(value);
            }
            unknown => return Err(format!("unknown web-bench argument: {unknown}")),
        }
    }
    if requests == 0 {
        return Err("--requests must be greater than zero".to_string());
    }
    let target = parse_endpoint_target(target_args)?;

    let mut total = Duration::ZERO;
    let mut min = Duration::MAX;
    let mut max = Duration::ZERO;
    for _ in 0..requests {
        let (status, _body, elapsed) = http_get(&target, &path)?;
        if status != 200 {
            return Err(format!("{path} returned HTTP {status} during bench"));
        }
        total += elapsed;
        min = min.min(elapsed);
        max = max.max(elapsed);
    }

    let avg = total / requests;
    println!(
        "web bench {}:{}{} requests={} avg_ms={} min_ms={} max_ms={}",
        target.host,
        target.port,
        path,
        requests,
        avg.as_micros() as f64 / 1000.0,
        min.as_micros() as f64 / 1000.0,
        max.as_micros() as f64 / 1000.0
    );
    Ok(())
}

fn parse_endpoint_target(args: Vec<String>) -> Result<EndpointTarget, String> {
    let mut target = EndpointTarget::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--host" => target.host = next_xtask_value(&mut args, "--host")?,
            "--port" => {
                target.port = parse_xtask_u16(&next_xtask_value(&mut args, "--port")?, "--port")?
            }
            unknown => return Err(format!("unknown endpoint argument: {unknown}")),
        }
    }
    Ok(target)
}

fn http_get(target: &EndpointTarget, path: &str) -> Result<(u16, String, Duration), String> {
    let started = Instant::now();
    let mut stream = TcpStream::connect((&*target.host, target.port)).map_err(|err| {
        format!(
            "failed to connect to {}:{}: {err}",
            target.host, target.port
        )
    })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|err| format!("failed to set read timeout: {err}"))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        target.host
    )
    .map_err(|err| format!("failed to write request for {path}: {err}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|err| format!("failed to read response for {path}: {err}"))?;
    let elapsed = started.elapsed();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .ok_or_else(|| format!("invalid HTTP response for {path}"))?;
    Ok((status, body.to_string(), elapsed))
}

fn next_xtask_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {name}"))
}

fn parse_xtask_u16(value: &str, name: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|err| format!("invalid {name}: {err}"))
}

fn parse_xtask_u32(value: &str, name: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|err| format!("invalid {name}: {err}"))
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
    matches!(file_name, ".git" | "target") || file_name.starts_with("target-")
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

#[cfg(test)]
mod tests {
    use super::{
        CrateInfo, architecture_violations, collect_internal_dependencies, dependency_cycles,
        display_list, parse_dependency_name, parse_module_decl, should_skip,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    #[test]
    fn parse_module_decl_reads_root_modules() {
        assert_eq!(
            parse_module_decl("pub mod map_manager;").as_deref(),
            Some("map_manager")
        );
        assert_eq!(
            parse_module_decl("mod base_server;").as_deref(),
            Some("base_server")
        );
        assert_eq!(parse_module_decl("pub use map_manager::MapManager;"), None);
        assert_eq!(parse_module_decl("mod tests {"), None);
    }

    #[test]
    fn parse_dependency_name_reads_path_dependencies_only() {
        assert_eq!(
            parse_dependency_name("sekai = { path = \"../sekai\" }").as_deref(),
            Some("sekai")
        );
        assert_eq!(parse_dependency_name("serde = \"1\""), None);
        assert_eq!(parse_dependency_name("[dependencies]"), None);
    }

    #[test]
    fn collect_internal_dependencies_keeps_workspace_edges_sorted() {
        let manifest = r#"
[dependencies]
sekai = { path = "../sekai" }
keisan = { path = "../keisan" }
serde = "1"
"#;
        let crate_names = ["keisan", "sekai", "shinobi"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();

        assert_eq!(
            collect_internal_dependencies(manifest, &crate_names, "shinobi"),
            vec!["keisan".to_string(), "sekai".to_string()]
        );
    }

    #[test]
    fn display_list_and_skip_rules_are_stable() {
        assert_eq!(display_list(&[]), "none");
        assert_eq!(display_list(&["a".to_string(), "b".to_string()]), "a, b");
        assert!(should_skip(".git"));
        assert!(should_skip("target-msvc-lock"));
        assert!(!should_skip("sekai"));
    }

    #[test]
    fn architecture_violations_accept_current_layering() {
        let crates = crates(vec![
            ("keisan", vec![]),
            ("jikan", vec!["keisan"]),
            ("butsuri", vec!["jikan", "keisan"]),
            ("sekai", vec!["butsuri", "jikan", "keisan"]),
            ("daikoku", vec!["butsuri", "jikan", "keisan", "sekai"]),
            (
                "shinobi",
                vec!["butsuri", "daikoku", "jikan", "keisan", "sekai"],
            ),
            ("hikari", vec![]),
        ]);

        assert!(architecture_violations(&crates).is_empty());
    }

    #[test]
    fn architecture_violations_reject_renderer_back_edges() {
        let crates = crates(vec![
            ("keisan", vec![]),
            ("hikari", vec![]),
            ("sekai", vec!["hikari"]),
            ("daikoku", vec!["sekai", "shinobi"]),
        ]);

        assert_eq!(
            architecture_violations(&crates),
            vec![
                "daikoku must not depend on shinobi (server must not depend on client code)"
                    .to_string(),
                "sekai must not depend on hikari (shared ECS must not know presentation)"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn architecture_violations_reject_early_hikari_internal_deps() {
        let crates = crates(vec![
            ("keisan", vec![]),
            ("sekai", vec![]),
            ("hikari", vec!["keisan", "sekai"]),
        ]);

        assert_eq!(
            architecture_violations(&crates),
            vec![
                "hikari has unexpected internal deps before extract integration: sekai (only keisan is allowed)"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn architecture_violations_reject_web_edge_back_edges() {
        let crates = crates(vec![
            ("daikoku", vec!["sekai", "omoikane_web"]),
            ("omoikane_web", vec![]),
        ]);

        assert_eq!(
            architecture_violations(&crates),
            vec![
                "daikoku must not depend on omoikane_web (authoritative server must not depend on web edge adapters)"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn architecture_violations_reject_control_back_edges() {
        let crates = crates(vec![
            ("daikoku", vec!["omoikane_control"]),
            ("omoikane_control", vec!["omoikane_web"]),
            ("omoikane_web", vec![]),
        ]);

        assert_eq!(
            architecture_violations(&crates),
            vec![
                "daikoku must not depend on omoikane_control (authoritative server must not depend on launch/control adapters)"
                    .to_string(),
                "omoikane_control must not depend on omoikane_web (network control manifests must stay below web adapters)"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn dependency_cycles_reports_canonical_cycles() {
        let crates = crates(vec![
            ("a", vec!["b"]),
            ("b", vec!["c"]),
            ("c", vec!["a"]),
            ("d", vec![]),
        ]);

        assert_eq!(
            dependency_cycles(&crates),
            vec![vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "a".to_string()
            ]]
        );
    }

    fn crates(entries: Vec<(&str, Vec<&str>)>) -> BTreeMap<String, CrateInfo> {
        entries
            .into_iter()
            .map(|(name, deps)| {
                (
                    name.to_string(),
                    CrateInfo {
                        name: name.to_string(),
                        path: PathBuf::from(name),
                        modules: Vec::new(),
                        internal_dependencies: deps.into_iter().map(str::to_string).collect(),
                    },
                )
            })
            .collect()
    }
}
