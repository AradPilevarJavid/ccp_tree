# Changelog

## [1.0.0] - 2026-08-17

### Added
- **Project statistics** for Markdown and structure output: files, directories, lines, size, and accurate `o200k_base` token count.
- **Git metadata** in Markdown snapshots: branch, commit hash, and remote URL when inside a Git repository.
- **Secret scanning** before exporting file contents. Warns about private keys, AWS access keys, GitHub/GitLab tokens, OpenAI/Google/Stripe/Slack keys, JWTs, authorization headers, and credential assignments without printing secret values.
  - Added `--no-secret-scan` to disable warnings.
- **MIME-aware binary handling** using content signatures (`infer`) and extension fallback.
  - Binary and oversized files now include MIME type, detection method, size, and detected extension in output.
  - Added UTF-16 text decoding with BOM support.
- **Configuration file support** via local `./.ccprc` and global `~/.ccprc`.
  - Configurable: templates dir, clipboard, exclusions, max size/chars, head/tail, from-end, tokens, output modes, verbosity, force, and more.
  - Added `--no-clipboard` flag to override clipboard enabled in config.
- **Jupyter notebook (.ipynb) rendering** to clean Markdown cells with code, outputs, errors, and rich results.
- **New built-in templates**: C, C++, Go, Java, Node.js, React, Ruby, Rust, TypeScript, and Web.
- **Man page generation** via new `ccp-mangen` binary.
- **Arch Linux AUR package support** with `aur/PKGBUILD`.
- **Content limiting options**:
  - `--max-chars <CHARS>` — limit characters per file.
  - `--head <LINES>` — include only first N lines.
  - `--tail <LINES>` — include only last N lines.
- **Token count flag** `-t` / `--tokens` for snapshot and reverse modes.

### Changed
- `-r` short flag now means `--from-end` (apply character limits from the end of a file). Raw output is available via `--raw`.
- Markdown snapshots now start with a statistics header followed by the project structure and file contents.
- `--structure` output now includes statistics.
- Binary and oversized file markers now contain MIME metadata instead of a generic message.
- Reverse `.tree` definitions now include MIME metadata for binary/oversized files.
- Codebase modularized into separate modules for CLI, config, file handling, Git, parser, renderers, scaffolding, secret scanning, statistics, templates, and tree operations.
- Added `Cargo.lock` for reproducible builds.

### Fixed
- Raw output no longer conflicts with `-s` when using `--raw` explicitly; validation added for incompatible output modes.




## [0.1.5]

### Changed
- Updated project roadmap(The roadmap is actually fabulous) and documentation.

## [0.1.4]

### Added
- Official Arch Linux AUR package support.
- PKGBUILD and .SRCINFO for installing `ccp_tree` from the AUR.

### Changed
- Added Cargo.lock to the repository for reproducible builds.
- Improved packaging workflow and release process.


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
