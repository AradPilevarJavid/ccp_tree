# Changelog

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
