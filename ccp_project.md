# Project Structure

```
├── CHANGELOG.md
├── Cargo.toml
├── LICENSE
├── README.md
├── build.rs
├── cargo
├── ccp.gif
├── ccp_project.md
├── src/
│   ├── lib.rs
│   └── main.rs
└── templates/
    └── python.tree
```

# File Contents

## CHANGELOG.md

````
# Changelog

## [0.1.3]
added --raw: this option outputs raw file contents without a directory tree. The tool doesn't use many tokens even by default, but if you want to be extremely token‑friendly, use `--raw`.



## [0.1.2]
### Fixed
- **Adaptive Markdown fences** – Previously, all file contents were wrapped in a hardcoded triple‑backtick code block (`` ``` ``). If a source file itself contained a line of consecutive backticks , the outer fence would close prematurely, breaking the output and possibly confusing an AI. Now, the tool scans each file’s content for the longest run of consecutive backtick characters and uses a fence one character longer than that run(for instance if you have 3 backticks inside your file the tool would wrap it inside four backticks). This ensures the block never closes unexpectedly, no matter how many nested backtick fences the file contains. The fix is applied to both the full Markdown snapshot and the `--structure` tree output.
- No performance regression: the scan is a single O(n) pass over the already‑in‑memory content and adds only microseconds per file.The program remains rust-fast :)

  
## [0.1.1]
- Added `--reverse` mode to produce reusable `.tree` definitions.
- Added `ccp generate` / `ccp create` commands to scaffold projects from `.tree` files.
- Built‑in template support (`python`) and custom template directories.
- Clipboard support (optional `-c` flag).
- Colored tree preview via `--dry-run`.

## [0.1.0]
- Initial public release: snapshot a directory into Markdown (full content + tree) or a simple tree view.

````

## Cargo.toml

```
[package]
name = "ccp_tree"
version = "0.1.3"
edition = "2021"
description = "ccp: 📄 Snapshot, 📋 blueprint, 🏗️ scaffold. Instantly capture project structure & files to Markdown/.tree, then regenerate anywhere."
license = "MIT"
repository = "https://github.com/AradPilevarJavid/ccp"
homepage = "https://github.com/AradPilevarJavid/ccp"
documentation = "https://docs.rs/ccp"
readme = "README.md"
keywords = ["snapshot", "blueprint", "scaffold", "markdown", "cli"]
categories = ["command-line-utilities", "development-tools"]
authors = ["Arad Pilevar Javid"]

[dependencies]
anyhow = "1.0"
anstream = "1.0"
clap = { version = "4.5", features = ["derive"] }
ignore = "0.4"
glob = "0.3"
arboard = { version = "3.3", optional = true }

[[bin]]
name = "ccp"
path = "src/main.rs"

[features]
default = ["clipboard"]
clipboard = ["dep:arboard"]

```

## LICENSE

```
MIT License

Copyright (c) 2026 Arad Pilevar Javid

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

```

## README.md

````
# ccp — Copy Project
(crate: `ccp_tree`)

> 📸 Snapshot · 📋 Blueprint · 🏗️ Scaffold  
> Capture a directory into a portable format and recreate it anywhere.

[![Crates.io](https://img.shields.io/crates/v/ccp_tree)](https://crates.io/crates/ccp_tree)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

`ccp` is a command‑line tool I built for daily use. It turns a folder into a human‑readable, copy‑paste‑friendly **snapshot**, and that same snapshot back into a real directory tree. Written in Rust 🦀 with a lot of love ❤️.

- 📄 **Snapshot** a project to Markdown (full content + tree) or a concise `.tree` definition.
- 📋 **Blueprint** – a single file that represents your entire project structure and contents.
- 🏗️ **Scaffold** – recreate the layout with a single command; perfect for bootstrapping, sharing ideas, or feeding LLMs full context.

It’s built for quick pasting into chat windows, code reviews, bug reports, and for generating repeatable project templates. Kinda neat actually 😄.

<p align="center">
  <img src="ccp.gif" alt="ccp demo"/>
</p>


---

## Features ✨

- 📄 **Markdown output** – full project tree + every file inside fenced code blocks.
- 🌲 **Tree‑only mode** (`--structure`) – just the directory hierarchy and structure.
- 🔁 **Reverse mode** – emit a `.tree` definition that can later be rebuilt.
- 🛠️ **Generate / Create** – turn a `.tree` definition (file, inline, or template) into real files.
- 🧩 **Template system** – bundled templates (Python, …) + custom templates directory.
- 🔍 **Smart ignores** – respects `.gitignore` / `.ignore`, plus a comprehensive default exclude list (node_modules, target, __pycache__, …).
- 📋 **Clipboard support** – optional; copies output directly to the clipboard.
- 🧹 **Flexible filtering** – include hidden files, skip default ignores, add custom glob patterns, limit file size.
- 🎨 **Colored tree preview** – visual inspection with `--dry-run`.

---

## Installation 🚀

### From Crates.io

```bash
cargo install ccp_tree
```

This installs the `ccp` binary.  
To enable clipboard support (optional, works out‑of‑the‑box on most systems):

```bash
cargo install ccp --features clipboard
```

> On Linux, the clipboard feature tries `wl-copy` (Wayland) and `xclip` (X11) first, then falls back to the `arboard` crate. No extra configuration needed. 👍

### From source

```bash
git clone https://github.com/AradPilevarJavid/ccp
cd ccp
cargo build --release
```

The binary will be at `./target/release/ccp`.

> 💡 Pro‑tip: try `ccp --help`, `ccp reverse --help` and `ccp generate --help` after installation.

---

## Quick Start ⚡

Work from any directory; by default `ccp` scans the current folder.

```bash
ccp                    # full Markdown snapshot → stdout
ccp > snapshot.md      # save to file
ccp -o output.md       # write directly to a file
ccp -s                 # only the folder tree, no contents
ccp --reverse          # output a .tree definition
```

---

## Usage 🧑‍💻

### The fastest way to capture the structure and recreate in one pipe

```bash
ccp reverse | ccp generate -f -q
```

```
ccp [ROOT] [OPTIONS]
ccp reverse [ROOT] [OPTIONS]
ccp generate [ROOT] [OPTIONS]
ccp create [ROOT] [OPTIONS]        # alias for generate
```

### 1. 📸 Snapshot a folder (default)

```bash
ccp                          # scan current directory
ccp /path/to/project -s      # structure only
ccp --reverse                # .tree definition
ccp --reverse --no-content   # .tree definition without file contents
```

### 2. 🔁 Reverse – create a `.tree` template

```bash
ccp reverse                         # current dir
ccp reverse /path/to/project        # specific folder
ccp reverse -o template.tree        # write to file
ccp reverse --no-content            # omit file contents
```

### 3. 🏗️ Generate / Create – materialise a `.tree` definition

```bash
# From a .tree file
ccp generate --input blueprint.tree

# Into a specific folder
ccp generate ./my-new-project --input blueprint.tree

# From a bundled template
ccp create my-project --template python

# From a custom template
ccp generate --template react-component --templates-dir ./templates

# Inline definition (use \n for newlines)
ccp generate --inline "src/
  index.ts: export default {}
  README.md: # Hello"
```

**Overwrite safety:** the command asks before overwriting existing files.  
Use `--force` to skip prompts, and `--quiet` to suppress them entirely.

**Dry‑run** previews the files that would be created, without touching disk:

```bash
ccp generate --dry-run --input blueprint.tree
```

---

## The `.tree` Definition Format 📝

A lightweight, indentation‑based format that describes files and directories.

### Syntax Rules

- Each line is an entry, indented with **two spaces** per level.
- **Directories** end with a trailing `/`.  
  Example: `src/`
- **Files** are just the name.
  Empty files have no extra content marker.
- **Single‑line contents** follow a colon (`:`):  
  `README.md: # My Project`
- **Multi‑line contents** use a pipe `|` after the colon, then indent content lines by **two extra spaces**:  
  ```
  file.txt:|
    line one
    line two
  ```
- **Binary files** or files exceeding `--max-size` are automatically marked:  
  `image.png: <binary file>`  
  `large.csv: <file too large>`

### Example

```
src/
  main.rs: fn main() { println!("Hello"); }
  lib.rs
  utils/
    helpers.rs: |
      pub fn add(a: i32, b: i32) -> i32 {
          a + b
      }
      pub fn sub(a: i32, b: i32) -> i32 {
          a - b
      }
README.md: # My Project
Cargo.toml: |
  [package]
  name = "my-project"
  version = "0.1.0"
```

This definition can be saved as a `.tree` file and reused with `ccp generate`.

---

## Options ⚙️

### Global / `ccp` (snapshot) & `ccp reverse` options
### if you run ccp --help you will see:

| Flag / Option               | Description |
|-----------------------------|-------------|
| `--include-hidden`          | Include dot‑files and dot‑folders. |
| `--no-ignore`               | Ignore `.gitignore` and `.ignore` files. |
| `-a`, `--all`               | Include default‑excluded directories (target, node_modules, …). |
| `-e`, `--exclude <PAT>`     | Exclude additional glob patterns (repeatable). |
| `--max-size <BYTES>`        | Skip files larger than this size (default: 1 MB). |
| `--structure`, `-s`         | Output only the directory tree (Markdown). |
| `--reverse`                 | Output in `.tree` definition format. |
| `--no-content`              | Omit file contents in `.tree` output. |
| `--dry-run`                 | Preview the tree (colored) without writing. |
| `--verbose`, `-v`           | Print extra progress info. |
| `--quiet`, `-q`             | Suppress non‑essential output. |
| `-o`, `--output <FILE>`     | Write output to a file instead of stdout. |
| `-c`, `--clipboard`         | Copy output to clipboard (requires the `clipboard` feature). |

### `ccp generate` / `ccp create` options
### if you run ccp create --help you will see:

| Flag / Option                | Description |
|------------------------------|-------------|
| `--input <FILE>`             | Read `.tree` definition from a file. |
| `--template <NAME>`          | Load a built‑in or custom template. |
| `--templates-dir <DIR>`      | Directory for custom templates (default: `templates/`). |
| `--inline <TEXT>`            | Provide the `.tree` definition directly (newlines as `\n`). |
| `--force`                    | Overwrite existing files without asking. |
| `--dry-run`                  | Preview files to be created (colored tree). |
| `--verbose`, `-v`            | Show every file/directory creation event. |
| `--quiet`, `-q`              | Suppress prompts and informational messages. |

### `ccp reverse` option
### if you run ccp reverse --help you will see:

| Flag / Option                | Description |
|------------------------------|-------------|
| `-o`, `--output <FILE>`      | Write output to this file instead of stdout. |
| `-c`, `--clipboard`          | Copy the result to the system clipboard (requires the `clipboard` feature). |
| `--include-hidden`           | Include hidden files and directories. |
| `--no-ignore`                | Do not respect `.gitignore` / `.ignore` files. |
| `-a`, `--all`                | Include default‑excluded directories (like `target`, `node_modules`). |
| `-e`, `--exclude <PAT>`      | Exclude additional glob patterns (repeatable). |
| `--max-size <BYTES>`         | Skip files larger than this size (default: 1 MB). |
| `--no-content`               | Omit file contents in the `.tree` output. |
| `--dry-run`                  | Preview the tree (colored) without writing. |
| `--verbose`, `-v`            | Print extra progress info. |
| `--quiet`, `-q`              | Suppress non‑essential output. |

---

## Practical Examples 💡

### 1. Full context for an AI / code review 🤖

```bash
ccp > project-for-ai.md
```

Paste the Markdown into your chat window. The AI sees the exact tree and every file’s content.

### 2. Quick visual scan (similar to ` ccp -s`) 👀

```bash
ccp --dry-run
```

Prints a colored tree without reading file contents – perfect for checking what would be included.

### 3. Create a reusable project template 📁

```bash
ccp reverse -o python-package.tree
```

Share the `.tree` file or commit it to a template repository.

### 4. Bootstrap a new project from a template 🚀

```bash
ccp create my-project --template python
```

### 5. Inline one‑off scaffolding 🧱
#### writing this requires knowing the .tree syntac
```bash
ccp generate --inline "src/
  main.rs: fn main() { println!(\"Hello\"); }
  Cargo.toml: |
    [package]
    name = \"demo\"
    version = \"0.1.0\"
    edition = \"2021\"
"
```

### 6. Exclude logs and artifacts everywhere 🗑️

```bash
ccp -e "*.log" -e "target/"
```

### 7. Snapshot everything (including build dirs) 🌐

```bash
ccp --all > full-snapshot.md
```

### 8. Scan a different folder and output only the tree 🌲

```bash
ccp ../another-project -s -o structure.md
```

---

## Built‑in Templates 📦

`ccp` ships with a few starter templates (bundled at compile time).  
Check the available ones with a non‑existent name:

```bash
ccp generate --template not-a-template  # error message lists all built‑in templates
```

Currently included: `python` (a minimal Python project).  
Add your own by placing `.tree` files into a `templates/` directory next to where you run `ccp`, or specify a path with `--templates-dir`.

The template loader checks:
1. `templates_dir/NAME`
2. `templates_dir/NAME.tree`
3. Built‑in templates (fallback)

So you can override a built‑in template by placing a file with the same name in your custom folder.

---

## Default Exclusions 🧹

To keep snapshots clean and focused, `ccp` skips a large set of common clutter by default.  
The full list is embedded in the source; it includes:

- Build & cache directories: `target/`, `node_modules/`, `dist/`, `build/`, `.next/`, `__pycache__/`, `.cache/`, …
- Version control: `.git/`
- Lock files: `Cargo.lock`, `*.lock`
- IDE / editor: `.idea/`, `.vscode/`, `*.swp`
- OS files: `.DS_Store`, `Thumbs.db`
- Binary archives: `*.zip`, `*.tar.gz`, `*.pdf`, `*.mp4`

Use **`-a` / `--all`** to include everything (except patterns added with `-e`).  
You can also create a `.mktreeignore` file in your project root to add custom ignore patterns (one per line, same syntax as `.gitignore`).

---

## Integration Tips 🔗

- **Piping to clipboard**: `ccp | xclip -selection clipboard` (if not using `-c`).
- **CI / scripts**: use `-q` and `--force` to avoid interactive prompts.
- **Templating engine**: you can pre‑process `.tree` files with environment variables or `sed` before feeding them to `ccp generate`.

---
## Roadmap(TODO)
- [ ] Add more built‑in templates (Rust, React, Go)
- [ ] Handel files that are too lard (more than 1048576 bytes)
- [ ] Better binary‑file detection and handeling (checksums)
- [ ] Have the name of the project directory when printing the structure (minor change)
- [ ] handel files like gifs


## License 📜

MIT – see [LICENSE](LICENSE).  
Copyright (c) 2026 Arad Pilevar Javid.

---

## Contributing 🤝

Feedback, issues, and pull requests are welcome!  
Make sure to run `cargo test` before submitting.  
If you want to add a new built‑in template, drop a `.tree` file into the `templates/` directory – the build script picks it up automatically.

````

## build.rs

```
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let templates_dir = manifest_dir.join("templates");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let output_path = out_dir.join("builtin_templates.rs");

    println!("cargo:rerun-if-changed={}", templates_dir.display());

    let mut templates = Vec::new();
    if templates_dir.is_dir() {
        for entry in fs::read_dir(&templates_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("tree") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            templates.push((stem.to_string(), path));
        }
    }

    templates.sort_by(|left, right| left.0.cmp(&right.0));

    let mut generated = String::from("const BUILTIN_TEMPLATES: &[(&str, &str)] = &[\n");
    for (name, path) in templates {
        generated.push_str(&format!(
            "    ({name:?}, include_str!({path:?})),\n",
            name = name,
            path = path.display().to_string()
        ));
    }
    generated.push_str("];\n");

    fs::write(output_path, generated)
}

```

## cargo

```

```

## ccp.gif

```
[Binary file not shown]
```

## ccp_project.md

```

```

## src/lib.rs

``````
use anyhow::{bail, Context, Result};
use glob::Pattern;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/builtin_templates.rs"));

pub const DEFAULT_EXCLUDES: &[&str] = &[
    "target/",
    "node_modules/",
    "dist/",
    "build/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    ".turbo/",
    ".cache/",
    "coverage/",
    "__pycache__/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".tox/",
    ".venv/",
    "venv/",
    ".gradle/",
    "cmake-build-debug/",
    "cmake-build-release/",
    "*.log",
    "logs/",
    ".log",
    "out/",
    "bin/",
    "obj/",
    "*.egg-info/",
    ".eggs/",
    ".pnp.*",
    ".yarn/",
    "vendor/",
    "Pods/",
    ".idea/",
    ".vscode/",
    "*.swp",
    "*.swo",
    ".DS_Store",
    ".nyc_output/",
    "htmlcov/",
    ".coverage",
    "test-results/",
    "playwright-report/",
    "tmp/",
    "temp/",
    ".tmp/",
    "Thumbs.db",
    "desktop.ini",
    "*.mp4",
    "*.zip",
    "*.tar.gz",
    "*.pdf",
    "public/uploads/",
    "storage/",
    "data/",
    ".env.local",
    ".env.*.local",
    ".git/",
    "Cargo.lock",
];

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    pub children: BTreeMap<String, Entry>,
}

impl Entry {
    pub fn new(name: String, is_dir: bool) -> Self {
        Self {
            name,
            is_dir,
            children: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalkOptions {
    pub include_hidden: bool,
    pub no_ignore: bool,
    pub include_useless: bool,
    pub exclude: Vec<String>,
    pub mktree_ignore: bool,
    pub max_size: u64,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub root: PathBuf,
    pub tree: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone)]
pub enum FileText {
    Text(String),
    Binary,
    TooLarge(u64),
}

#[derive(Debug, Clone)]
pub struct ExcludePattern {
    raw: String,
    pattern: Pattern,
    directory_only: bool,
}

impl ExcludePattern {
    pub fn new(raw: &str) -> Result<Self> {
        let directory_only = raw.ends_with('/');
        let pattern_text = raw.trim_end_matches('/');
        let pattern = Pattern::new(pattern_text)
            .with_context(|| format!("Invalid exclusion pattern: {raw}"))?;
        Ok(Self {
            raw: pattern_text.to_string(),
            pattern,
            directory_only,
        })
    }

    pub fn matches(&self, relative: &Path, is_dir: bool) -> bool {
        if self.directory_only && !is_dir {
            return false;
        }

        let relative_text = relative.to_string_lossy().replace('\\', "/");
        if self.pattern.matches(&relative_text) {
            return true;
        }

        if self.raw.contains('/') {
            return false;
        }

        relative
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .any(|component| self.pattern.matches(component))
    }
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub is_dir: bool,
    pub content: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub force: bool,
    pub dry_run: bool,
    pub verbose: bool,
    pub quiet: bool,
}

#[derive(Debug, Clone)]
pub struct GenerateEvent {
    pub action: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

pub fn load_ignore_patterns(
    root: &Path,
    excludes: &[String],
    use_mktree_ignore: bool,
) -> Result<Vec<ExcludePattern>> {
    let mut patterns = excludes.to_vec();
    if use_mktree_ignore {
        let ignore_path = root.join(".mktreeignore");
        if ignore_path.exists() {
            let content = fs::read_to_string(&ignore_path)
                .with_context(|| format!("Failed to read {}", ignore_path.display()))?;
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                patterns.push(trimmed.to_string());
            }
        }
    }

    patterns
        .iter()
        .map(|pattern| ExcludePattern::new(pattern))
        .collect()
}

pub fn load_default_exclude_patterns() -> Result<Vec<ExcludePattern>> {
    DEFAULT_EXCLUDES
        .iter()
        .map(|pattern| ExcludePattern::new(pattern))
        .collect()
}

pub fn should_exclude(relative: &Path, is_dir: bool, patterns: &[ExcludePattern]) -> bool {
    patterns
        .iter()
        .any(|pattern| pattern.matches(relative, is_dir))
}

pub fn is_useless_dir_name(name: &str) -> bool {
    DEFAULT_EXCLUDES
        .iter()
        .filter(|pattern| pattern.ends_with('/') && !pattern.contains('*'))
        .map(|pattern| pattern.trim_end_matches('/'))
        .any(|pattern| pattern == name)
}

pub fn is_useless_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_useless_dir_name)
}

pub fn insert_entry(root: &mut BTreeMap<String, Entry>, components: &[String], is_dir: bool) {
    if components.is_empty() {
        return;
    }
    let name = &components[0];
    let entry = root
        .entry(name.clone())
        .or_insert_with(|| Entry::new(name.clone(), components.len() > 1 || is_dir));
    if components.len() > 1 {
        entry.is_dir = true;
        insert_entry(&mut entry.children, &components[1..], is_dir);
    } else {
        entry.is_dir = is_dir;
    }
}

pub fn snapshot(root: &Path, options: &WalkOptions) -> Result<Snapshot> {
    let root = root.to_path_buf();
    let root_for_filter = root.clone();
    let include_useless = options.include_useless;
    let mut exclude_patterns = if include_useless {
        Vec::new()
    } else {
        load_default_exclude_patterns()?
    };
    exclude_patterns.extend(load_ignore_patterns(
        &root,
        &options.exclude,
        options.mktree_ignore,
    )?);

    let mut builder = WalkBuilder::new(&root);
    builder
        .hidden(!options.include_hidden)
        .git_ignore(!options.no_ignore)
        .ignore(!options.no_ignore)
        .follow_links(false);

    builder.filter_entry(move |entry| {
        let path = entry.path();
        if path == root_for_filter {
            return true;
        }
        let relative = path.strip_prefix(&root_for_filter).unwrap_or(path);
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        !should_exclude(relative, is_dir, &exclude_patterns)
    });

    let mut tree = BTreeMap::new();
    for result in builder.build() {
        let entry = result?;
        let path = entry.path();
        if path == root {
            continue;
        }
        let relative = path.strip_prefix(&root).unwrap_or(path);
        let components: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        insert_entry(&mut tree, &components, is_dir);
    }

    Ok(Snapshot { root, tree })
}

pub fn fmt_tree(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
    let mut out = String::new();
    let entries_vec: Vec<&Entry> = entries.values().collect();
    let count = entries_vec.len();
    for (index, entry) in entries_vec.iter().enumerate() {
        let last_child = index == count - 1;
        let (connector, child_prefix) = if last_child {
            ("└── ", format!("{}    ", prefix))
        } else {
            ("├── ", format!("{}│   ", prefix))
        };
        let display_name = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        };
        out.push_str(&format!("{}{}{}\n", prefix, connector, display_name));
        if entry.is_dir && !entry.children.is_empty() {
            out.push_str(&fmt_tree(&entry.children, &child_prefix));
        }
    }
    out
}

pub fn fmt_colored_tree(entries: &BTreeMap<String, Entry>, prefix: &str) -> String {
    let mut out = String::new();
    let entries_vec: Vec<&Entry> = entries.values().collect();
    let count = entries_vec.len();
    for (index, entry) in entries_vec.iter().enumerate() {
        let last_child = index == count - 1;
        let (connector, child_prefix) = if last_child {
            ("└── ", format!("{}    ", prefix))
        } else {
            ("├── ", format!("{}│   ", prefix))
        };
        let display_name = if entry.is_dir {
            format!("\x1b[34m{}/\x1b[0m", entry.name)
        } else {
            format!("\x1b[32m{}\x1b[0m", entry.name)
        };
        out.push_str(&format!("{}{}{}\n", prefix, connector, display_name));
        if entry.is_dir && !entry.children.is_empty() {
            out.push_str(&fmt_colored_tree(&entry.children, &child_prefix));
        }
    }
    out
}

pub fn collect_files(entries: &BTreeMap<String, Entry>, current_path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in entries.values() {
        let child_path = current_path.join(&entry.name);
        if entry.is_dir {
            files.extend(collect_files(&entry.children, &child_path));
        } else {
            files.push(child_path);
        }
    }
    files
}

pub fn read_file_text(path: &Path, max_size: u64) -> Result<FileText> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_size {
        return Ok(FileText::TooLarge(max_size));
    }
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(FileText::Text(text)),
        Err(_) => Ok(FileText::Binary),
    }
}

pub fn file_content(path: &Path, max_size: u64) -> Result<String> {
    match read_file_text(path, max_size)? {
        FileText::Text(text) => Ok(text),
        FileText::Binary => Ok("[Binary file not shown]".to_string()),
        FileText::TooLarge(size) => Ok(format!("[File too large, > {} bytes]", size)),
    }
}

fn markdown_fence_for(content: &str) -> String {
    let mut max_run = 0;
    let mut current_run = 0;
    for character in content.chars() {
        if character == '`' {
            current_run += 1;
            max_run = max_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    "`".repeat(std::cmp::max(3, max_run + 1))
}

pub fn render_markdown(snapshot: &Snapshot, max_size: u64) -> String {
    let tree_str = fmt_tree(&snapshot.tree, "");
    let tree_fence = markdown_fence_for(&tree_str);
    let mut output = format!("# Project Structure\n\n{tree_fence}\n{tree_str}{tree_fence}\n");
    output.push_str("\n# File Contents\n");
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    for path in &file_paths {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        let content = match file_content(path, max_size) {
            Ok(content) => content,
            Err(error) => format!("[Error reading file: {}]", error),
        };
        let fence = markdown_fence_for(&content);
        output.push_str(&format!("\n## {}\n\n{fence}\n", relative.display()));
        output.push_str(&content);
        output.push_str(&format!("\n{fence}\n"));
    }
    output
}

pub fn render_raw(snapshot: &Snapshot, max_size: u64) -> String {
    let file_paths = collect_files(&snapshot.tree, &snapshot.root);
    let mut output = String::new();
    for (index, path) in file_paths.iter().enumerate() {
        let relative = path.strip_prefix(&snapshot.root).unwrap_or(path);
        let content = match file_content(path, max_size) {
            Ok(content) => content,
            Err(error) => format!("[Error reading file: {}]", error),
        };
        output.push_str(&format!("==== {} ====\n", relative.display()));
        output.push_str(&content);
        if !content.ends_with('\n') {
            output.push('\n');
        }
        if index + 1 < file_paths.len() {
            output.push('\n');
        }
    }
    output
}

pub fn render_structure(snapshot: &Snapshot) -> String {
    let tree_str = fmt_tree(&snapshot.tree, "");
    let fence = markdown_fence_for(&tree_str);
    format!("# Project Structure\n\n{fence}\n{tree_str}{fence}\n")
}

pub fn render_tree_definition(snapshot: &Snapshot, max_size: u64, no_content: bool) -> String {
    render_tree_definition_entries(&snapshot.tree, &snapshot.root, 0, max_size, no_content)
}

fn render_tree_definition_entries(
    entries: &BTreeMap<String, Entry>,
    current_path: &Path,
    depth: usize,
    max_size: u64,
    no_content: bool,
) -> String {
    let mut out = String::new();
    let indent = "  ".repeat(depth);
    for entry in entries.values() {
        let child_path = current_path.join(&entry.name);
        if entry.is_dir {
            out.push_str(&format!("{}{}/\n", indent, entry.name));
            out.push_str(&render_tree_definition_entries(
                &entry.children,
                &child_path,
                depth + 1,
                max_size,
                no_content,
            ));
            continue;
        }

        if no_content {
            out.push_str(&format!("{}{}\n", indent, entry.name));
            continue;
        }

        match read_file_text(&child_path, max_size) {
            Ok(FileText::Text(text)) if text.is_empty() => {
                out.push_str(&format!("{}{}\n", indent, entry.name))
            }
            Ok(FileText::Text(text)) if is_single_line(&text) => {
                out.push_str(&format!("{}{}: {}\n", indent, entry.name, text));
            }
            Ok(FileText::Text(text)) => {
                out.push_str(&format!("{}{}:|\n", indent, entry.name));
                for line in text.lines() {
                    out.push_str(&format!("{}  {}\n", indent, line));
                }
                if text.ends_with('\n') {
                    out.push_str(&format!("{}  \n", indent));
                }
            }
            Ok(FileText::Binary) => {
                out.push_str(&format!("{}{}: <binary file>\n", indent, entry.name))
            }
            Ok(FileText::TooLarge(_)) => {
                out.push_str(&format!("{}{}: <file too large>\n", indent, entry.name))
            }
            Err(error) => out.push_str(&format!("{}{}: <error: {}>\n", indent, entry.name, error)),
        }
    }
    out
}

fn is_single_line(text: &str) -> bool {
    !text.contains('\n') && !text.contains('\r')
}

pub fn parse_tree_definition(input: &str) -> Result<Vec<TreeNode>> {
    let lines: Vec<&str> = input.lines().collect();
    let mut index = 0;
    parse_nodes(&lines, &mut index, 0)
}

fn parse_nodes(lines: &[&str], index: &mut usize, depth: usize) -> Result<Vec<TreeNode>> {
    let mut nodes = Vec::new();
    while *index < lines.len() {
        let line = lines[*index];
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            *index += 1;
            continue;
        }

        let current_depth = indentation_depth(line)?;
        if current_depth < depth {
            break;
        }
        if current_depth > depth {
            bail!("Unexpected indentation on line {}", *index + 1);
        }

        let trimmed = line.trim_start();
        *index += 1;
        let mut node = parse_node_header(trimmed)?;

        if node.content.as_deref() == Some("__MULTILINE__") {
            let mut content_lines = Vec::new();
            while *index < lines.len() {
                let content_line = lines[*index];
                if content_line.trim().is_empty() {
                    content_lines.push(String::new());
                    *index += 1;
                    continue;
                }
                let content_depth = indentation_depth(content_line)?;
                if content_depth <= current_depth {
                    break;
                }
                let strip_chars = ((current_depth + 1) * 2).min(content_line.len());
                let content = if content_line.len() >= strip_chars {
                    content_line[strip_chars..].to_string()
                } else {
                    String::new()
                };
                content_lines.push(content);
                *index += 1;
            }
            node.content = Some(content_lines.join("\n"));
        }

        if node.is_dir {
            node.children = parse_nodes(lines, index, depth + 1)?;
        }
        nodes.push(node);
    }
    Ok(nodes)
}

fn indentation_depth(line: &str) -> Result<usize> {
    let mut spaces = 0;
    for character in line.chars() {
        match character {
            ' ' => spaces += 1,
            '\t' => bail!("Tabs are not supported for indentation"),
            _ => break,
        }
    }
    if spaces % 2 != 0 {
        bail!("Indentation must use multiples of two spaces");
    }
    Ok(spaces / 2)
}

fn parse_node_header(header: &str) -> Result<TreeNode> {
    let (raw_name, content) = match header.split_once(':') {
        Some((name, "|")) => (name.trim(), Some("__MULTILINE__".to_string())),
        Some((name, value)) => (name.trim(), Some(value.trim_start().to_string())),
        None => (header.trim(), None),
    };

    if raw_name.is_empty() {
        bail!("Tree entry names cannot be empty");
    }
    if raw_name.contains('/') && !raw_name.ends_with('/') {
        bail!("Nested paths are not supported inside a single tree entry: {raw_name}");
    }

    let is_dir = raw_name.ends_with('/');
    let name = raw_name.trim_end_matches('/').to_string();
    Ok(TreeNode {
        name,
        is_dir,
        content,
        children: Vec::new(),
    })
}

pub fn nodes_to_entries(nodes: &[TreeNode]) -> BTreeMap<String, Entry> {
    let mut entries = BTreeMap::new();
    for node in nodes {
        entries.insert(
            node.name.clone(),
            Entry {
                name: node.name.clone(),
                is_dir: node.is_dir,
                children: nodes_to_entries(&node.children),
            },
        );
    }
    entries
}

pub fn create_tree(
    root: &Path,
    nodes: &[TreeNode],
    options: &GenerateOptions,
) -> Result<Vec<GenerateEvent>> {
    let mut events = Vec::new();
    if !options.dry_run {
        fs::create_dir_all(root).with_context(|| format!("Failed to create {}", root.display()))?;
    }
    for node in nodes {
        create_node(root, node, options, &mut events)?;
    }
    Ok(events)
}

fn create_node(
    root: &Path,
    node: &TreeNode,
    options: &GenerateOptions,
    events: &mut Vec<GenerateEvent>,
) -> Result<()> {
    let path = root.join(&node.name);
    if node.is_dir {
        if path.exists() && !path.is_dir() {
            handle_existing(&path, options)?;
            if !options.dry_run {
                remove_existing(&path)?;
            }
        }
        let exists_after_overwrite = path.exists();
        events.push(GenerateEvent {
            action: if exists_after_overwrite {
                "keep"
            } else {
                "create"
            }
            .to_string(),
            path: path.clone(),
            is_dir: true,
        });
        if !options.dry_run {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create {}", path.display()))?;
        }
        for child in &node.children {
            create_node(&path, child, options, events)?;
        }
        return Ok(());
    }

    let existed = path.exists();
    if existed {
        handle_existing(&path, options)?;
        if !options.dry_run {
            remove_existing(&path)?;
        }
    }
    events.push(GenerateEvent {
        action: if existed { "overwrite" } else { "create" }.to_string(),
        path: path.clone(),
        is_dir: false,
    });
    if !options.dry_run {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        fs::write(&path, node.content.as_deref().unwrap_or_default())
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }
    Ok(())
}

fn handle_existing(path: &Path, options: &GenerateOptions) -> Result<()> {
    if options.force || options.dry_run {
        return Ok(());
    }
    if options.quiet || !io::stdin().is_terminal() {
        bail!(
            "{} already exists; use --force to overwrite",
            path.display()
        );
    }
    print!("{} already exists. Overwrite? [y/N] ", path.display());
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
        Ok(())
    } else {
        bail!("Aborted by user")
    }
}

fn remove_existing(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("Failed to remove {}", path.display()))
    } else {
        fs::remove_file(path).with_context(|| format!("Failed to remove {}", path.display()))
    }
}

pub fn load_template(templates_dir: &Path, name: &str) -> Result<String> {
    let candidates = [
        templates_dir.join(name),
        templates_dir.join(format!("{name}.tree")),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return fs::read_to_string(&candidate)
                .with_context(|| format!("Failed to read template {}", candidate.display()));
        }
    }

    let builtin_name = name.strip_suffix(".tree").unwrap_or(name);
    if let Some((_, template)) = BUILTIN_TEMPLATES
        .iter()
        .find(|(template_name, _)| *template_name == builtin_name)
    {
        return Ok((*template).to_string());
    }

    let builtin_templates = if BUILTIN_TEMPLATES.is_empty() {
        "none".to_string()
    } else {
        BUILTIN_TEMPLATES
            .iter()
            .map(|(template_name, _)| *template_name)
            .collect::<Vec<_>>()
            .join(", ")
    };

    bail!(
        "Template '{name}' was not found in {} or built-in templates ({builtin_templates})",
        templates_dir.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_patterns() -> Vec<ExcludePattern> {
        load_default_exclude_patterns().expect("default exclude patterns should be valid")
    }

    #[test]
    fn default_excludes_match_nested_directories() {
        let patterns = default_patterns();

        assert!(should_exclude(
            Path::new("frontend/node_modules/react/index.js"),
            true,
            &patterns
        ));
        assert!(should_exclude(
            Path::new("app/.next/cache"),
            true,
            &patterns
        ));
        assert!(should_exclude(
            Path::new("service/__pycache__"),
            true,
            &patterns
        ));
    }

    #[test]
    fn default_excludes_match_file_globs_and_exact_files() {
        let patterns = default_patterns();

        assert!(should_exclude(Path::new("debug.log"), false, &patterns));
        assert!(should_exclude(
            Path::new("src/.env.production.local"),
            false,
            &patterns
        ));
        assert!(should_exclude(Path::new("Cargo.lock"), false, &patterns));
        assert!(should_exclude(
            Path::new("docs/archive.tar.gz"),
            false,
            &patterns
        ));
    }

    #[test]
    fn directory_only_defaults_do_not_match_files_with_same_name() {
        let patterns = default_patterns();

        assert!(!should_exclude(Path::new("docs/target"), false, &patterns));
    }

    #[test]
    fn structure_render_omits_file_contents_section() {
        let mut tree = BTreeMap::new();
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );
        let snapshot = Snapshot {
            root: PathBuf::from("."),
            tree,
        };

        let output = render_structure(&snapshot);

        assert!(output.contains("# Project Structure"));
        assert!(!output.contains("# File Contents"));
    }

    #[test]
    fn markdown_fence_is_longer_than_content_backtick_runs() {
        assert_eq!(markdown_fence_for("no fences"), "```");
        assert_eq!(markdown_fence_for("```rust\nfn main() {}\n```"), "````");
        assert_eq!(markdown_fence_for("````\ninner\n````"), "`````");
    }

    #[test]
    fn markdown_render_uses_adaptive_fences_for_file_contents() {
        let root =
            std::env::temp_dir().join(format!("ccp-markdown-fence-test-{}", std::process::id()));
        let readme_path = root.join("README.md");

        fs::create_dir_all(&root).expect("test root should be created");
        fs::write(&readme_path, "before\n```rust\nfn main() {}\n```\nafter")
            .expect("test file should be written");

        let mut tree = BTreeMap::new();
        insert_entry(&mut tree, &[String::from("README.md")], false);
        let snapshot = Snapshot { root, tree };

        let output = render_markdown(&snapshot, 1_000);

        assert!(output.contains("## README.md\n\n````\nbefore\n```rust"));
        assert!(output.contains("```\nafter\n````\n"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn raw_render_outputs_only_delimited_file_contents_in_order() {
        let root = std::env::temp_dir().join(format!("ccp-raw-test-{}", std::process::id()));
        let src_dir = root.join("src");
        let readme_path = root.join("README.md");
        let main_path = src_dir.join("main.rs");

        fs::create_dir_all(&src_dir).expect("test src dir should be created");
        fs::write(&readme_path, "readme\n").expect("readme should be written");
        fs::write(&main_path, "fn main() {}").expect("main should be written");

        let mut tree = BTreeMap::new();
        insert_entry(
            &mut tree,
            &[String::from("src"), String::from("main.rs")],
            false,
        );
        insert_entry(&mut tree, &[String::from("README.md")], false);
        let snapshot = Snapshot { root, tree };

        let output = render_raw(&snapshot, 1_000);

        assert_eq!(
            output,
            "==== README.md ====\nreadme\n\n==== src/main.rs ====\nfn main() {}\n"
        );
        assert!(!output.contains("# Project Structure"));
        assert!(!output.contains("```"));

        fs::remove_dir_all(&snapshot.root).expect("test root should be removed");
    }

    #[test]
    fn load_template_falls_back_to_builtin_templates() {
        let template = load_template(Path::new("missing-template-dir"), "python")
            .expect("python should be available as a built-in template");

        assert!(template.contains("main.py"));
        assert!(template.contains("Hello from ccp"));
    }
}

``````

## src/main.rs

```
use anstream::println as aprintln;
use anyhow::{Context, Result};
use ccp_tree::{
    create_tree, fmt_colored_tree, load_template, nodes_to_entries, parse_tree_definition,
    render_markdown, render_raw, render_structure, render_tree_definition, snapshot,
    GenerateOptions, Snapshot, WalkOptions,
};
use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "ccp")]
#[command(version)]
#[command(about = "Snapshot, scaffold, and blueprint projects")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Root directory to scan (defaults to current directory)
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    clipboard: bool,

    /// Include hidden files and directories (those starting with a dot)
    #[arg(long)]
    include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    no_ignore: bool,

    /// Include generated/cache folders that are skipped by default
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Maximum file size in bytes (larger files are skipped)
    #[arg(long, default_value = "1048576")]
    max_size: u64,

    /// Omit file contents from reverse .tree output
    #[arg(long)]
    no_content: bool,

    /// Output only the project structure, without file contents
    #[arg(long, short = 's')]
    structure: bool,

    /// Output a reusable .tree definition instead of Markdown
    #[arg(long)]
    reverse: bool,

    /// Output raw concatenated file contents (no Markdown, no tree)
    #[arg(long)]
    raw: bool,

    /// Preview filesystem operations or scan output only
    #[arg(long)]
    dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    verbose: bool,

    /// Suppress non-essential messages
    #[arg(long, short)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Create files and directories from an indented .tree definition
    Generate(GenerateCommand),
    /// Alias for generate
    Create(GenerateCommand),
    /// Output a reusable .tree definition for a directory
    Reverse(ReverseCommand),
}

#[derive(Parser)]
struct GenerateCommand {
    /// Target directory to create into
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Read structure from a .tree file
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Load a .tree file from the templates directory
    #[arg(long)]
    template: Option<String>,

    /// Directory containing .tree templates
    #[arg(long, default_value = "templates")]
    templates_dir: PathBuf,

    /// Inline .tree structure text
    #[arg(long)]
    inline: Option<String>,

    /// Overwrite existing files without prompting
    #[arg(long)]
    force: bool,

    /// Preview only, without writing files
    #[arg(long)]
    dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    verbose: bool,

    /// Suppress non-essential messages and decline overwrite prompts
    #[arg(long, short)]
    quiet: bool,
}

#[derive(Parser)]
struct ReverseCommand {
    /// Root directory to scan
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Write output to this file instead of stdout
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Copy the result to the system clipboard (requires the 'clipboard' feature)
    #[cfg(feature = "clipboard")]
    #[arg(long, short = 'c')]
    clipboard: bool,

    /// Include hidden files and directories
    #[arg(long)]
    include_hidden: bool,

    /// Do not respect .gitignore / .ignore files
    #[arg(long)]
    no_ignore: bool,

    /// Include generated/cache folders that are skipped by default
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Glob patterns to exclude (can be repeated)
    #[arg(short = 'e', long = "exclude")]
    exclude: Vec<String>,

    /// Maximum file size in bytes
    #[arg(long, default_value = "1048576")]
    max_size: u64,

    /// Omit file contents
    #[arg(long)]
    no_content: bool,

    /// Preview tree in color before emitting .tree output
    #[arg(long)]
    dry_run: bool,

    /// Print more progress details
    #[arg(long, short)]
    verbose: bool,

    /// Suppress non-essential messages
    #[arg(long, short)]
    quiet: bool,
}

#[cfg(feature = "clipboard")]
fn set_clipboard(text: &str) -> Result<()> {
    use std::io::Write;

    // On Linux, try wl-copy (Wayland) and xclip (X11) first.
    // These force the active selection, bypassing KDE's clipboard history.
    if cfg!(target_os = "linux") {
        if let Ok(mut child) = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let status = child.wait()?;
            if status.success() {
                return Ok(());
            }
        }
        if let Ok(mut child) = std::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-i"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(text.as_bytes())?;
            }
            let status = child.wait()?;
            if status.success() {
                return Ok(());
            }
        }
    }
    // Fallback to arboard for non-Linux or if neither tool is installed
    let mut clipboard = arboard::Clipboard::new().context("Failed to access clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to set clipboard")?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Generate(command)) | Some(Command::Create(command)) => run_generate(command),
        Some(Command::Reverse(command)) => run_reverse(command),
        None => run_copy(cli),
    }
}

fn run_copy(cli: Cli) -> Result<()> {
    let options = WalkOptions {
        include_hidden: cli.include_hidden,
        no_ignore: cli.no_ignore,
        include_useless: cli.all,
        exclude: cli.exclude,
        mktree_ignore: true,
        max_size: cli.max_size,
    };
    let scan = snapshot(&cli.root, &options)?;

    if cli.dry_run {
        aprintln!("{}", fmt_colored_tree(&scan.tree, ""));
        return Ok(());
    }

    let output = if cli.raw {
        render_raw(&scan, cli.max_size)
    } else if cli.reverse {
        render_tree_definition(&scan, cli.max_size, cli.no_content)
    } else if cli.structure {
        render_structure(&scan)
    } else {
        render_markdown(&scan, cli.max_size)
    };

    #[cfg(feature = "clipboard")]
    if cli.clipboard {
        set_clipboard(&output)?;
        if !cli.quiet {
            let message = if cli.raw {
                if cli.all {
                    "Full raw project snapshot copied to clipboard."
                } else {
                    "Raw project snapshot copied to clipboard."
                }
            } else if cli.reverse {
                if cli.all {
                    "Tree definition (all files) copied to clipboard."
                } else {
                    "Tree definition copied to clipboard."
                }
            } else if cli.structure {
                if cli.all {
                    "Full project structure copied to clipboard."
                } else {
                    "Project structure copied to clipboard."
                }
            } else {
                if cli.all {
                    "Full project snapshot copied to clipboard."
                } else {
                    "Project snapshot copied to clipboard."
                }
            };
            println!("{}", message);
        }
        return Ok(());
    }
    if cli.reverse && !cli.raw {
        let output_path = cli
            .output
            .unwrap_or_else(|| default_tree_output_path(&cli.root));
        return write_output(Some(output_path), &output);
    }

    write_output(cli.output, &output)
}

fn run_reverse(command: ReverseCommand) -> Result<()> {
    let options = WalkOptions {
        include_hidden: command.include_hidden,
        no_ignore: command.no_ignore,
        include_useless: command.all,
        exclude: command.exclude,
        mktree_ignore: true,
        max_size: command.max_size,
    };

    let scan = snapshot(&command.root, &options)?;
    if command.dry_run && !command.quiet {
        aprintln!("{}", fmt_colored_tree(&scan.tree, ""));
    }
    if command.verbose && !command.quiet {
        eprintln!("Scanned {}", command.root.display());
    }

    let output = render_tree_definition(&scan, command.max_size, command.no_content);

    #[cfg(feature = "clipboard")]
    if command.clipboard {
        set_clipboard(&output)?;
        if !command.quiet {
            let message = if command.all {
                "Tree definition (all files) copied to clipboard."
            } else {
                "Tree definition copied to clipboard."
            };
            println!("{}", message);
        }
        return Ok(());
    }

    let output_path = command
        .output
        .unwrap_or_else(|| default_tree_output_path(&command.root));
    write_output(Some(output_path), &output)
}

fn default_tree_output_path(root: &Path) -> PathBuf {
    let name = root
        .file_name()
        .filter(|name| !name.is_empty() && *name != "." && *name != "..")
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ccp".to_string());

    PathBuf::from(format!("{name}.tree"))
}

fn run_generate(command: GenerateCommand) -> Result<()> {
    let input = load_generate_input(&command)?;
    let nodes = parse_tree_definition(&input)?;
    let options = GenerateOptions {
        force: command.force,
        dry_run: command.dry_run,
        verbose: command.verbose,
        quiet: command.quiet,
    };

    if command.dry_run && !command.quiet {
        let preview = Snapshot {
            root: command.root.clone(),
            tree: nodes_to_entries(&nodes),
        };
        aprintln!("{}", fmt_colored_tree(&preview.tree, ""));
    }

    let events = create_tree(&command.root, &nodes, &options)?;
    if (command.verbose || command.dry_run) && !command.quiet {
        for event in events {
            let suffix = if event.is_dir { "/" } else { "" };
            eprintln!("{} {}{}", event.action, event.path.display(), suffix);
        }
    }
    Ok(())
}

fn load_generate_input(command: &GenerateCommand) -> Result<String> {
    let provided = command.input.is_some() as u8
        + command.template.is_some() as u8
        + command.inline.is_some() as u8;
    if provided > 1 {
        anyhow::bail!("Use only one of --input, --template, or --inline");
    }

    if let Some(inline) = &command.inline {
        return Ok(inline.replace("\\n", "\n"));
    }
    if let Some(template) = &command.template {
        return load_template(&command.templates_dir, template);
    }
    if let Some(input) = &command.input {
        return fs::read_to_string(input)
            .with_context(|| format!("Failed to read {}", input.display()));
    }

    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .context("Failed to read .tree definition from stdin")?;
    Ok(buffer)
}

fn write_output(output_path: Option<PathBuf>, output: &str) -> Result<()> {
    if let Some(path) = output_path {
        fs::write(&path, output).with_context(|| format!("Failed to write {}", path.display()))?;
    } else {
        use std::io::Write;

        io::stdout()
            .write_all(output.as_bytes())
            .context("Failed to write to stdout")?;
    }
    Ok(())
}

```

## templates/python.tree

```
src/
  main.py:|
    def main():
        print("Hello from ccp")
    if __name__ == "__main__":
        main()
LICENSE: MIT License
README.md:|
  # My Python Project
  Built with `ccp`.
requirements.txt:|
  # Add dependencies here
.gitignore:|
  __pycache__/
  *.pyc
  venv/
  .env
  *.egg-info
  dist/
  build/
```
