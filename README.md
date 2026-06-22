# use gpg



# ccp — Copy Project

`ccp` is a command-line tool that captures a directory tree and its file contents into a portable format, and can recreate the directory structure from that format. Think of it as a project snapshotter and scaffolder in one.

- **Snapshot** a folder into a Markdown document or a concise `.tree` definition.
- **Blueprint** output – a human-readable, copy‑paste‑friendly representation of an entire project.
- **Scaffold** a new directory from a `.tree` definition, perfect for quickly recreating project skeletons, sharing ideas, or feeding AI with full context.

It’s especially handy for pasting a project into a chat window (LLMs, code reviews, bug reports), or for generating repeatable project templates.

## Features

- **Tree-only mode** (`--structure`) – just the folder hierarchy without file contents.
- **Reverse mode** (`--reverse`) – emits a lightweight `.tree` definition that can later be used to recreate the exact directory and file layout.
- **Generate / Create** subcommands – turn a `.tree` definition back into real files and directories.
- **Template system** – store reusable `.tree` snippets in a `templates/` directory.
- **Smart ignores** – respects `.gitignore` / `.ignore` files and skips common clutter (`node_modules`, `target`, `__pycache__`, etc.) by default.
- **Clipboard support** – optional, copies the output directly to the system clipboard.
- **Flexible filtering** – include hidden files, ignore default excludes, add custom exclude patterns, limit file size.
- **Colored tree preview** – for quick visual inspection (`--dry-run`).

## Installation

### Using cargo

```bash
cargo install copy-project
```

This will install the `ccp` binary.

To enable clipboard support:

```bash
cargo install copy-project --features clipboard
```

On Linux the clipboard feature tries `wl-copy` (Wayland) and `xclip` (X11) first; falls back to the `arboard` crate if neither is found.

### From source

```bash
git clone https://github.com/user/copy-project   # replace with actual repo
cd copy-project
cargo build --release
```

The binary will be at `./target/release/ccp`.

## Basic Usage

`ccp` works on the current directory by default.

```bash
ccp              # snapshot current dir → Markdown to stdout
ccp . > output.md
ccp -s           # only the folder tree (no file contents)
ccp --reverse    # output a .tree definition
ccp -o snapshot.md   # write directly to a file
```

## Subcommands

### `ccp reverse` – Generate a .tree definition

Scans a directory and prints its structure in the `.tree` format. Useful for creating templates.

```bash
ccp reverse                     # current dir
ccp reverse /path/to/project    # specific folder
ccp reverse -o template.tree
```

Use `--no-content` to omit file contents from the output; the definition then only carries the hierarchy and file names.

### `ccp generate` (or `ccp create`) – Create files from a .tree definition

Reads a `.tree` definition and creates all directories and files accordingly.

```bash
# From a file
ccp generate --input blueprint.tree

# From a template stored in templates/
ccp generate --template react-component

# Inline definition (\\n for newlines)
ccp generate --inline "src/
  index.ts: export default {}
  README.md: # Hello"
```

By default the target is the current directory. Use the first positional argument to specify a different destination:

```bash
ccp generate ./my-new-project --input blueprint.tree
```

The command asks before overwriting existing files. Use `--force` to overwrite without prompts.

**Dry-run** shows what would happen without touching the file system:

```bash
ccp generate --dry-run --input blueprint.tree
```

## Options

### Global options (apply to `ccp`, `ccp reverse`, etc.)

| Flag / Option           | Description |
|------------------------|-------------|
| `--include-hidden`     | Include files and folders starting with a dot. |
| `--no-ignore`          | Ignore `.gitignore` and `.ignore` files. |
| `-a`, `--all`          | Include default‑excluded directories (like `node_modules`, `target`). |
| `-e`, `--exclude <PAT>`| Exclude additional glob patterns (repeatable). |
| `--max-size <BYTES>`   | Skip files larger than this size (default: 1 MB). |
| `--structure`, `-s`    | Output only the directory tree (Markdown). |
| `--reverse`            | Output in `.tree` definition format. |
| `--no-content`         | Omit file contents in `.tree` output. |
| `--dry-run`            | Preview the tree/operations without writing. |
| `--verbose`, `-v`      | Print extra progress info. |
| `--quiet`, `-q`        | Suppress non‑essential output. |
| `-o`, `--output <FILE>`| Write output to a file instead of stdout. |
| `-c`, `--clipboard`    | Copy output to clipboard (requires the `clipboard` feature). |

### `ccp generate` specific

| Flag / Option              | Description |
|---------------------------|-------------|
| `--input <FILE>`           | Read `.tree` definition from a file. |
| `--template <NAME>`        | Load a template from the templates directory. |
| `--templates-dir <DIR>`    | Set the templates directory (default: `templates/`). |
| `--inline <TEXT>`          | Provide the `.tree` definition directly. |
| `--force`                  | Overwrite existing files without asking. |
| `--dry-run`                | Preview the files to be created. |
| `--verbose`, `-v`          | Show file creation events. |
| `--quiet`, `-q`            | Suppress prompts and informational messages. |

## Examples

**1. Full Markdown snapshot for sharing**

```bash
ccp > project-for-ai.md
```

The output includes a tree and every file’s content inside fenced code blocks.

**2. Only the folder tree**

```bash
ccp -s
```

**3. Create a reusable project template**

```bash
ccp reverse -o python-package.tree
```

**4. Generate a project from the template**

```bash
ccp generate new-package --input python-package.tree
```

**5. Use an inline blueprint**

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

**6. Exclude logs and target everywhere**

```bash
ccp -e "*.log" -e "target/"
```

## The .tree Definition Format

A simple, indentation‑based format that describes files and directories.

- Each entry is a name on its own line, indented with **two spaces** per level.
- Directory names end with a trailing `/`.
- File contents can follow a colon (`:`). One‑liners: `file.txt: hello world`.
- Multi‑line contents use `|` after the colon and are indented further (two extra spaces).
- Empty files are just the name.
- Binary files or files too large are marked as `<binary file>` or `<file too large>`.

Example:

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

## Default Exclusions

By default `ccp` skips many common build artifacts, caches, and large files. You can see the full list in the source code – it includes directories like `target/`, `node_modules/`, `dist/`, `build/`, `__pycache__/`, `.git/`, `Cargo.lock`, `*.log`, etc.

Use `-a` / `--all` to include everything (except patterns added with `-e`).

## License

MIT – see the [LICENSE](LICENSE) file.
