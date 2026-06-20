
# Copy4AI

**Copy a project's structure and file contents to your clipboard (or file/STDOUT) – just like the copy4AI VSCode extension, but as a standalone CLI.**

`copy-project` recursively walks a directory, builds a Unicode tree of its structure, and then dumps the content of every text file in a clean, copy‑paste‑friendly Markdown format. It respects `.gitignore` and `.ignore`, skips binary files, and can automatically copy the output to the system clipboard.

## Features

- **Directory tree** – rendered with box‑drawing characters (directories end with `/`).
- **File contents** – each file is shown as a Markdown heading with a fenced code block.
- **Respects ignore rules** – automatically skips files/directories matched by `.gitignore` and `.ignore`.
- **Custom exclusion patterns** – add glob patterns to exclude additional files.
- **Skips binary files** – non‑UTF‑8 files are replaced with a placeholder.
- **Size limit** – files larger than a configurable threshold are skipped (default 1 MB).
- **Clipboard support** – one flag copies the entire output directly to your system clipboard.
- **File output** – optionally write the result to a file.
- **Hidden file control** – choose whether to include dotfiles.
- **Cross‑platform** – works on Linux, macOS, and Windows.

## Installation

### From source (with Cargo)
```bash
cargo install --git https://github.com/your-username/copy-project
```
(Replace the URL with the actual repository after publishing.)

Or clone and build manually:
```bash
git clone https://github.com/your-username/copy-project
cd copy-project
cargo build --release
```
The binary will be at `target/release/copy-project`.

### Pre‑built binaries
Check the [Releases](https://github.com/your-username/copy-project/releases) page for downloadable executables.

## Usage

```
copy-project [OPTIONS] [ROOT]
```

- `ROOT` – Directory to scan (defaults to current directory `.`).

### Basic examples

```bash
# Dump current project to STDOUT
copy-project

# Copy my project to the clipboard
copy-project /path/to/my_project --clipboard

# Save output to a file
copy-project -o project_dump.md

# Include hidden files, ignore .gitignore
copy-project --include-hidden --no-ignore

# Exclude log and temp files
copy-project -e "*.log" -e "tmp/**"

# Set a custom max file size (100 KB)
copy-project --max-size 102400
```

### Full options

| Flag / Option | Description |
|---------------|-------------|
| `ROOT` | Root directory (default: `.`) |
| `-o`, `--output <FILE>` | Write output to `FILE` instead of STDOUT |
| `--clipboard` | Copy result to system clipboard (requires `clipboard` feature) |
| `--include-hidden` | Include hidden files and directories (starting with `.`) |
| `--no-ignore` | Do **not** respect `.gitignore` / `.ignore` files |
| `-e`, `--exclude <PATTERN>` | Exclude files/directories matching this glob (can be repeated) |
| `--max-size <BYTES>` | Skip files larger than this (default: 1048576) |
| `-h`, `--help` | Print help |
| `-V`, `--version` | Print version |

## Output format

The generated text looks like this:

~~~
# Project Structure

```
├── src/
│   ├── main.rs
│   └── lib.rs
├── Cargo.toml
└── README.md
```

# File Contents

## src/main.rs

```
fn main() {
    println!("Hello, world!");
}
```

## Cargo.toml

```
[package]
name = "my-app"
version = "0.1.0"
edition = "2021"
```
~~~

- Files that cannot be read as UTF‑8 display: `[Binary file not shown]`
- Files exceeding the size limit show: `[File too large, > N bytes]`

This format is perfect for pasting into AI assistants, documentation, or issue reports.

## Building from source

Requirements:
- Rust toolchain (1.70+)
- On Linux, `libxcb` development packages may be needed for clipboard support (optional).

```bash
# Clone the repo
git clone https://github.com/your-username/copy-project
cd copy-project

# Build (with clipboard support, default)
cargo build --release

# Build without clipboard support
cargo build --release --no-default-features
```

## License

This project is licensed under the MIT License – see the [LICENSE](LICENSE) file for details.

---

**Inspired by the copy4AI VSCode extension. Contributions welcome!**
