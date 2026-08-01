use std::{
    fs,
    path::{Path, PathBuf},
};

// ─────────────────────────────────────────────────────────────────────────────
// Rule 1 — module boundaries
// ─────────────────────────────────────────────────────────────────────────────

/// The only legal way in to a module is its `api.rs`. Everything else is
/// private by contract, whether or not `pub` happens to allow it.
///
/// This also covers `infrastructure/http/`: the composition root may call
/// several modules, but only through their public surface.
#[test]
fn rule_01_no_module_reaches_into_another_modules_layers() {
    let mut violations = Vec::new();

    for file in rust_files(&src_dir()) {
        let owner = owning_module(&file);
        for (target_module, segment) in cross_module_paths(&code_of(&file)) {
            if Some(&target_module) == owner.as_ref() {
                continue; // a module may reference its own layers
            }
            if LAYERS.contains(&segment.as_str()) {
                violations.push(format!(
                    "{}: reaches into crate::modules::{target_module}::{segment} \
                     — call a Command/Query in {target_module}'s api.rs instead",
                    rel(&file)
                ));
            }
        }
    }

    report(
        "no module imports another module's domain/application/infrastructure",
        violations,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rules 2 & 3 — the domain layer stays pure
// ─────────────────────────────────────────────────────────────────────────────

/// Dependencies point inward. The domain describes rules; it never learns how
/// they are persisted.
#[test]
fn rule_02_domain_does_not_depend_on_infrastructure() {
    let mut violations = Vec::new();

    for file in layer_files("domain") {
        for (line_no, line) in numbered_lines(&code_of(&file)) {
            if line.contains("use ") && line.contains("infrastructure") {
                violations.push(format!(
                    "{}:{line_no}: {} — domain must not know about infrastructure; \
                     pass what it needs in as an argument",
                    rel(&file),
                    line.trim()
                ));
            }
        }
    }

    report("domain/ does not depend on infrastructure/", violations);
}

/// No I/O and no `async` in the domain. A domain rule you cannot call from a
/// plain unit test without a runtime, a pool, or a network is not a domain rule.
#[test]
fn rule_03_domain_is_pure() {
    const FORBIDDEN: &[(&str, &str)] = &[
        (
            "async fn",
            "domain logic must be callable without a runtime",
        ),
        ("sqlx", "database access belongs in infrastructure/"),
        ("reqwest", "HTTP calls belong in infrastructure/"),
        ("tokio::", "domain logic must not touch the runtime"),
        ("std::fs", "file I/O belongs in infrastructure/"),
        ("axum", "the domain must not know that HTTP exists"),
    ];

    let mut violations = Vec::new();

    for file in layer_files("domain") {
        for (line_no, line) in numbered_lines(&code_of(&file)) {
            for (needle, why) in FORBIDDEN {
                if line.contains(needle) {
                    violations.push(format!("{}:{line_no}: `{needle}` — {why}", rel(&file)));
                }
            }
        }
    }

    report("domain/ is pure — no I/O, no async", violations);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rules 4 & 5 — the module's public surface
// ─────────────────────────────────────────────────────────────────────────────

/// Handlers are `async` even when today's body does no awaiting. Turning a sync
/// public handler async later is a breaking change for every caller; starting
/// async costs nothing.
#[test]
fn rule_04_every_public_api_handler_is_async() {
    let mut violations = Vec::new();

    for file in module_files_named("api.rs") {
        for (line_no, line) in numbered_lines(&code_of(&file)) {
            if line.trim_start().starts_with("pub fn ") {
                violations.push(format!(
                    "{}:{line_no}: {} — api.rs handlers must be `pub async fn`",
                    rel(&file),
                    line.trim()
                ));
            }
        }
    }

    report("every pub fn in api.rs is async", violations);
}

/// `mod.rs` is the boundary. It re-exports `api` and declares the layers
/// privately — the moment it re-exports a layer, the boundary is gone and rule 1
/// has nothing left to protect.
#[test]
fn rule_05_module_mod_rs_reexports_only_api() {
    let mut violations = Vec::new();

    for module in module_dirs() {
        let mod_rs = module.join("mod.rs");
        let Some(code) = try_code_of(&mod_rs) else {
            violations.push(format!("{}: missing mod.rs", rel(&module)));
            continue;
        };

        if !code.contains("pub use self::api") && !code.contains("pub use api") {
            violations.push(format!(
                "{}: does not re-export api — the module has no public surface",
                rel(&mod_rs)
            ));
        }

        for (line_no, line) in numbered_lines(&code) {
            let trimmed = line.trim();
            for layer in LAYERS {
                let leaks_declaration = trimmed.starts_with(&format!("pub mod {layer}"));
                let leaks_reexport = trimmed.starts_with("pub use") && trimmed.contains(layer);

                if leaks_declaration || leaks_reexport {
                    violations.push(format!(
                        "{}:{line_no}: {trimmed} — `{layer}` must stay private to the module",
                        rel(&mod_rs)
                    ));
                }
            }
        }
    }

    report("mod.rs re-exports only api", violations);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 6 — structural completeness
// ─────────────────────────────────────────────────────────────────────────────

/// All four layers, always — even when one is currently a single `mod.rs` with
/// nothing in it (ADR-003). A "simple module" that skips the domain layer is how
/// business rules end up in a SQL query.
#[test]
fn rule_06_every_module_has_all_four_layers() {
    const REQUIRED_DIRS: [&str; 3] = ["domain", "application", "infrastructure"];
    const REQUIRED_FILES: [&str; 2] = ["mod.rs", "api.rs"];

    let mut violations = Vec::new();

    for module in module_dirs() {
        for file in REQUIRED_FILES {
            if !module.join(file).is_file() {
                violations.push(format!("{}: missing {file}", rel(&module)));
            }
        }
        for dir in REQUIRED_DIRS {
            if !module.join(dir).is_dir() {
                violations.push(format!(
                    "{}: missing {dir}/ (ADR-003: no layer is optional)",
                    rel(&module)
                ));
            }
        }
    }

    report("every module has all four layers", violations);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 7 — event naming
// ─────────────────────────────────────────────────────────────────────────────

/// Events describe what already happened. `IndexRepoEvent` is a command wearing
/// a costume — a subscriber cannot refuse it, so naming it imperatively invites
/// callers to treat the bus as a remote procedure call.
///
/// Add to `IRREGULAR_PAST_PARTICIPLES` rather than weakening the check.
#[test]
fn rule_07_event_names_are_past_tense() {
    const IRREGULAR_PAST_PARTICIPLES: &[&str] = &[
        "Begun", "Bound", "Broken", "Built", "Chosen", "Cut", "Dealt", "Drawn", "Found", "Frozen",
        "Given", "Held", "Hidden", "Kept", "Left", "Lost", "Made", "Put", "Read", "Run", "Sent",
        "Set", "Shown", "Split", "Spent", "Taken", "Thrown", "Torn", "Written",
    ];

    let mut violations = Vec::new();

    for file in module_files_named("events.rs") {
        if !file.components().any(|c| c.as_os_str() == "domain") {
            continue; // only domain/events.rs declares events
        }

        for (line_no, name) in declared_type_names(&code_of(&file)) {
            let Some(stem) = name.strip_suffix("Event") else {
                violations.push(format!(
                    "{}:{line_no}: `{name}` — event types must end in `Event`",
                    rel(&file)
                ));
                continue;
            };

            let past_tense = stem.ends_with("ed")
                || IRREGULAR_PAST_PARTICIPLES.iter().any(|p| stem.ends_with(p));

            if !past_tense {
                violations.push(format!(
                    "{}:{line_no}: `{name}` — not past tense; an event records what \
                     happened (RepoIndexedEvent), not what should happen (IndexRepoEvent)",
                    rel(&file)
                ));
            }
        }
    }

    report("event names are past tense and end in Event", violations);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rules 8 & 9 — the shared kernel stays a kernel
// ─────────────────────────────────────────────────────────────────────────────

/// The shared kernel is upstream of everything. One import from a module and it
/// becomes a cycle — and the "extract a module into a service" path in
/// ARCHITECTURE.md §10 stops working.
#[test]
fn rule_08_shared_kernel_imports_nothing_from_modules() {
    let mut violations = Vec::new();

    for file in rust_files(&src_dir().join("shared_kernel")) {
        for (line_no, line) in numbered_lines(&code_of(&file)) {
            if line.contains("crate::modules") {
                violations.push(format!(
                    "{}:{line_no}: {} — shared_kernel is upstream of every module",
                    rel(&file),
                    line.trim()
                ));
            }
        }
    }

    report("shared_kernel/ imports nothing from modules/", violations);
}

/// No frameworks in the shared kernel. This is the rule that keeps `AppError`
/// free of status codes and lets `infrastructure/http/error.rs` own that mapping
/// alone.
///
/// It is also why `AppContext` lives in `infrastructure/context.rs`: it holds a
/// sqlx pool. ARCHITECTURE.md §3 still shows it here — the diagram is the thing
/// that is out of date, not this test.
#[test]
fn rule_09_shared_kernel_imports_no_framework() {
    const FRAMEWORKS: [&str; 3] = ["axum", "sqlx", "reqwest"];

    let mut violations = Vec::new();

    for file in rust_files(&src_dir().join("shared_kernel")) {
        for (line_no, line) in numbered_lines(&code_of(&file)) {
            for framework in FRAMEWORKS {
                if line.contains(framework) {
                    violations.push(format!(
                        "{}:{line_no}: `{framework}` — shared_kernel must survive being \
                         lifted into another service unchanged",
                        rel(&file)
                    ));
                }
            }
        }
    }

    report("shared_kernel/ imports no framework", violations);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule 10 — one door to the environment
// ─────────────────────────────────────────────────────────────────────────────

/// Config is read once, at startup, in one file. A `std::env::var` buried in a
/// handler is a setting nobody can find, that fails on request #10000 instead of
/// at boot, and that no test can override.
#[test]
fn rule_10_env_vars_are_read_only_in_config() {
    let config_rs = src_dir().join("infrastructure").join("config.rs");
    let mut violations = Vec::new();

    for file in rust_files(&src_dir()) {
        if file == config_rs {
            continue;
        }
        for (line_no, line) in numbered_lines(&code_of(&file)) {
            if line.contains("env::var") {
                violations.push(format!(
                    "{}:{line_no}: {} — read this in infrastructure/config.rs and pass it \
                     through AppContext",
                    rel(&file),
                    line.trim()
                ));
            }
        }
    }

    report("env::var appears only in config.rs", violations);
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

const LAYERS: [&str; 3] = ["domain", "application", "infrastructure"];

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn src_dir() -> PathBuf {
    manifest_dir().join("src")
}

fn modules_dir() -> PathBuf {
    src_dir().join("modules")
}

/// Path relative to the crate root, for readable failure messages.
fn rel(path: &Path) -> String {
    path.strip_prefix(manifest_dir())
        .unwrap_or(path)
        .display()
        .to_string()
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_rust_files(root, &mut found);
    found.sort();
    found
}

fn collect_rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return; // directory does not exist yet — nothing to check
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// Every `src/modules/<name>/` directory.
fn module_dirs() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(modules_dir()) else {
        return Vec::new(); // no modules yet — the rules still hold vacuously
    };

    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// Which module owns this file, if any. `None` for shared_kernel and the
/// composition root.
fn owning_module(path: &Path) -> Option<String> {
    let rest = path.strip_prefix(modules_dir()).ok()?;
    rest.components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
}

/// Every file inside a module's given layer directory.
fn layer_files(layer: &str) -> Vec<PathBuf> {
    rust_files(&modules_dir())
        .into_iter()
        .filter(|p| p.components().any(|c| c.as_os_str() == layer))
        .collect()
}

fn module_files_named(name: &str) -> Vec<PathBuf> {
    rust_files(&modules_dir())
        .into_iter()
        .filter(|p| p.file_name().is_some_and(|f| f == name))
        .collect()
}

fn code_of(path: &Path) -> String {
    try_code_of(path).unwrap_or_default()
}

fn try_code_of(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|src| strip_comments(&src))
}

fn numbered_lines(code: &str) -> impl Iterator<Item = (usize, &str)> {
    code.lines().enumerate().map(|(i, line)| (i + 1, line))
}

/// Strips `//` and `/* */` comments while preserving line numbering, so a
/// comment explaining a rule never trips it.
///
/// String literals are respected; char literals are not special-cased, since
/// distinguishing `'x'` from a lifetime `'a` needs a real lexer and the
/// difference has never mattered here.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut block_depth = 0_usize;

    while let Some(c) = chars.next() {
        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push('\n');
            }
            continue;
        }

        if block_depth > 0 {
            match c {
                '*' if chars.peek() == Some(&'/') => {
                    chars.next();
                    block_depth -= 1;
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    block_depth += 1;
                }
                '\n' => out.push('\n'),
                _ => {}
            }
            continue;
        }

        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(escaped) = chars.next() {
                    out.push(escaped);
                }
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line_comment = true;
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                block_depth = 1;
            }
            _ => out.push(c),
        }
    }

    out
}

/// Finds `crate::modules::<name>::<segment>` references, including the
/// brace-group form `crate::modules::<name>::{a, b}`.
fn cross_module_paths(code: &str) -> Vec<(String, String)> {
    const NEEDLE: &str = "crate::modules::";

    let mut found = Vec::new();
    let mut rest = code;

    while let Some(idx) = rest.find(NEEDLE) {
        let after = &rest[idx + NEEDLE.len()..];
        rest = after;

        let (module, tail) = take_ident(after);
        if module.is_empty() {
            continue;
        }

        let Some(tail) = tail.trim_start().strip_prefix("::") else {
            continue; // `crate::modules::auth` alone — the module root, allowed
        };
        let tail = tail.trim_start();

        match tail.strip_prefix('{') {
            Some(group) => {
                let end = group.find('}').unwrap_or(group.len());
                for item in group[..end].split(',') {
                    let (segment, _) = take_ident(item.trim_start());
                    if !segment.is_empty() {
                        found.push((module.clone(), segment));
                    }
                }
            }
            None => {
                let (segment, _) = take_ident(tail);
                if !segment.is_empty() {
                    found.push((module.clone(), segment));
                }
            }
        }
    }

    found
}

/// `(line number, type name)` for every `struct` / `enum` declared in the code.
fn declared_type_names(code: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();

    for (line_no, line) in numbered_lines(code) {
        for keyword in ["struct ", "enum "] {
            let Some(idx) = line.find(keyword) else {
                continue;
            };

            // Skip mid-expression matches — a declaration's keyword is preceded
            // only by visibility modifiers.
            let prefix = line[..idx].trim();
            if !(prefix.is_empty() || prefix.starts_with("pub")) {
                continue;
            }

            let (name, _) = take_ident(&line[idx + keyword.len()..]);
            if !name.is_empty() {
                found.push((line_no, name));
            }
        }
    }

    found
}

fn take_ident(s: &str) -> (String, &str) {
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    (s[..end].to_owned(), &s[end..])
}

fn report(rule: &str, violations: Vec<String>) {
    if violations.is_empty() {
        return;
    }

    panic!(
        "\n\n  ARCHITECTURE VIOLATION — {rule}\n  {} occurrence(s):\n\n    {}\n\n  \
         See ARCHITECTURE.md §8 and the relevant ADR.\n",
        violations.len(),
        violations.join("\n    ")
    );
}
