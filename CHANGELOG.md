# Changelog

## Unreleased

- Added built-in starter templates for C, C++, Go, Java, Node.js, React, Ruby, Rust, TypeScript, and plain HTML/CSS/JavaScript projects.
- Improved binary detection using MIME signatures and control-byte heuristics, added UTF-16 text decoding, and exposed the MIME detection method in output metadata.
- Reworked `ccp-mangen` to generate complete main and subcommand man pages, added stdout rendering, and expanded the manual with examples and `.tree` syntax.
- Added pre-export warnings for common private keys, API keys, tokens, JWTs, and credential assignments without printing matched secret values.
- Added TOML configuration through local `./.ccprc` and higher-priority global `~/.ccprc` files, with explicit CLI arguments taking final precedence.
- Added `-t` / `--tokens` to snapshot and reverse modes to count `o200k_base` BPE tokens and exit.
- Replaced the four-characters-per-token heuristic with complete `o200k_base` tokenization.
- Added MIME detection and size metadata for binary and oversized files.
- Added branch, commit hash, and remote URL metadata to Markdown snapshots of Git repositories.
- Added `--head <LINES>` and `--tail <LINES>` to limit file contents by line count.
- Changed `-r` into a reverse-direction modifier so `-cr --max-chars <CHARS>` copies content from the end of each file.
- Kept raw output available through `--raw`.

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
