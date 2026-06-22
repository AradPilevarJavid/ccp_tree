# sign it with gpg

# 🧩 ccp – AI-Friendly Project Snapshot & Scaffold Tool

`ccp` is a single binary that snapshots project structure and file contents into AI-friendly Markdown, generates project files from `.tree` blueprints, and reverses directories back into `.tree` definitions.

---

## ✨ Features

- 📸 **Markdown snapshots** – Full directory tree + file contents in one pasteable document.
- 🏗️ **Physical scaffolding** – Generate files and folders from indented `.tree` definitions.
- 🔄 **Reverse mode** – Turn any directory into a reusable `.tree` blueprint.
- ✂️ **Inline & template input** – Create projects from command line strings or pre‑saved templates.
- 🎯 **Smart ignore handling** – Respects `.gitignore`, `.ignore`, `.mktreeignore`, and `--exclude` globs.
- 🧹 **Junk‑folder filtering** – Skips `target`, `node_modules`, `dist`, `.next`, `__pycache__`, etc. by default.
- 🗃️ **Binary‑safe** – Non‑UTF‑8 files are marked `<binary file>` instead of breaking.
- 🎨 **Dry‑run previews** – Color‑coded tree outputs before any file is written.
- 📋 **Clipboard support** – Directly copy the snapshot (requires default `clipboard` feature).

---

## 📦 Installation

```bash
cargo build --release
```

The binary is `target/release/ccp`.  
Add it to your `PATH` or use `cargo install --path .`.

---

## 🚀 Quick Start

### 1. Snapshot a Project (Markdown)

```bash
# Current directory as Markdown
ccp

# Save to file
ccp /path/to/project -o project.md

# Copy to clipboard (default feature)
ccp --clipboard

# Preview only (colored tree)
ccp --dry-run

# Include hidden & junk folders
ccp -a --include-hidden
```

The output looks like:

````markdown
# Project Structure
```
src/
  main.rs
  lib.rs
README.md
```

# File Contents

## src/main.rs
```rust
fn main() {
    println!("Hello");
}
```
````

### 2. Generate Files from a `.tree` Blueprint

```bash
# From a file
ccp generate ./my-app --input blueprint.tree

# From inline text (use \n for newlines)
ccp generate ./my-app --inline "src/\n  main.rs:|\n    fn main() {}\nREADME.md: Hello"

# From a template (searches templates/ folder)
ccp generate ./my-app --template react

# Dry-run to see what will be created
ccp generate ./my-app --input blueprint.tree --dry-run

# Overwrite without prompts
ccp generate ./my-app --input blueprint.tree --force
```

`create` is an alias for `generate`.

#### 📝 `.tree` Format

```text
src/
  main.rs:|
    fn main() {
        println!("Hello");
    }
README.md: Hello from ccp
assets/
  logo.png: <binary file>
```

- Directories end with `/`
- Inline content: `filename: content`
- Multi‑line content: `filename:|` followed by indented lines
- Use **exactly 2 spaces** per indentation level

### 3. Reverse a Directory to `.tree`

```bash
# Standard reverse
ccp reverse ./project -o project.tree

# Same as top-level --reverse flag
ccp ./project --reverse -o project.tree

# Skip file contents (only structure)
ccp reverse ./project --no-content

# Dry-run + save definition
ccp reverse ./project --dry-run -o project.tree
```

---

## 🎯 Common Options

These apply to snapshot, reverse, and sometimes generate commands.

| Flag | Description |
|------|-------------|
| `-a, --all` | Include generated/cache folders normally skipped. |
| `--include-hidden` | Include dotfiles and hidden directories. |
| `--no-ignore` | Ignore `.gitignore` and `.ignore` rules. |
| `-e, --exclude <PATTERN>` | Add custom glob exclusions (repeatable). |
| `--max-size <BYTES>` | Skip files larger than this (default 1 MB). |
| `--dry-run` | Preview only – no files written or output generated (except colored tree). |
| `--no-content` | Omit file contents in reverse mode. |
| `-v, --verbose` | Print detailed progress. |
| `-q, --quiet` | Suppress non‑essential messages. |
| `-V, --version` | Print version. |

---

## 📁 Ignored Folders (by default)

```
target, node_modules, dist, build, .next, .nuxt, .svelte-kit, .turbo,
.cache, coverage, __pycache__, .pytest_cache, .mypy_cache, .ruff_cache,
.tox, .venv, venv, .gradle, cmake-build-debug, cmake-build-release
```
Use `-a` / `--all` to include them.


## 📋 Clipboard Feature

When built with the default `clipboard` feature, `--clipboard` sends the snapshot directly to the system clipboard, ready to paste into a chat or document.



## 📄 License

MIT – see `LICENSE` file.

**Commit message** (use this for the initial commit):

```bash
git commit -m "Initial commit: ccp - AI-friendly project snapshot and scaffold tool"
```

You can drop this directly into your `README.md`. Let me know if you’d like any section expanded or adjusted further.
