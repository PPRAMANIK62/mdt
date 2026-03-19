# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**mdt** (published as `mdtui` on crates.io) is a terminal-based markdown viewer/editor built in Rust using Ratatui. It features a file browser, markdown preview with syntax highlighting, vim-style editor, live preview split pane, fuzzy file finder, and file system watching.

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (binary: target/release/mdt)
cargo test                     # Run all tests
cargo clippy -- -D warnings    # Lint (CI enforces warnings-as-errors)
cargo fmt --check              # Check formatting
cargo fmt                      # Auto-format
cargo run -- [path] [--max-file-size <bytes>]  # Run the app
```

CI runs: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`.

## Code Style

- Max line width: 100 characters (`.rustfmt.toml`)
- MSRV: 1.70 (`clippy.toml`)
- Clippy pedantic is enabled with specific exemptions in `clippy.toml`

## Architecture

### Event Loop (`src/main.rs`)

Dirty-flag rendering loop with 50ms poll interval. Three event sources: keyboard/mouse input (crossterm), filesystem watcher (debounced 300ms), and live preview debounce timer (150ms). Uses advisory file locking and panic-safe terminal cleanup.

### Application State (`src/app/`)

The `App` struct in `mod.rs` holds all state: `DocumentState`, `EditorState`, `SearchState`, `TreeViewState`, `FileFinderState`, `LinkPickerState`, `LivePreviewState`, `CursorState`. Modes (`AppMode`) and focus (`Focus`) control which input handler and UI renderer are active. Key enums live in `types.rs`.

### Markdown Rendering Pipeline (`src/markdown/`)

Two-phase rendering:
1. **`render_markdown_blocks`** (in `renderer.rs`): Parses markdown via pulldown-cmark and syntax-highlights code blocks via syntect → produces width-independent `RenderedBlock`s (cached)
2. **`rewrap_blocks`** (in `blocks.rs`): Re-wraps blocks to current viewport width → `Vec<Line>` for display

Syntax highlighting is pre-warmed on a background thread before the app starts.

### UI Rendering (`src/ui/`)

Layout: status bar (1 row) on top, main area split into file tree (25%) and content pane (75%). Live preview splits the content pane horizontally or vertically. Overlays (help, link picker, file ops) render on top via `modal.rs`.

### Input Handling (`src/input/`)

Dispatched by mode: `normal.rs` (navigation/tree), `editor.rs` (vim insert/normal), `command.rs` (`:w`, `:wq`, `:q`, `:e`, `:preview`), `search.rs`, `file_ops.rs`, `mouse.rs`.

### File Operations (`src/file_ops.rs`)

Stateless pure functions. Rejects `..` traversal, canonicalizes paths, auto-appends `.md`. Supports create/delete/rename/move with nested directory creation.

### File Watching (`src/watcher.rs`)

Background thread with debounced events (300ms). Handles vim's write-temp-rename pattern. Auto-reloads files on external modification.

## Website (`/website/`)

Separate Astro + Starlight docs site. Uses Bun, oxlint, oxfmt. Has its own `CLAUDE.md` with specific instructions. Husky pre-commit hooks for lint + format.

### App ↔ Website Sync

The website documents all user-facing app behavior. **When changing the TUI, update the corresponding website docs too.** Here's the mapping:

| TUI change area | Website file to update |
|---|---|
| Keybindings (`src/input/normal.rs`, `editor.rs`, etc.) | `website/src/content/docs/keybindings.mdx` |
| File browser / tree / fuzzy finder | `website/src/content/docs/file-browser.mdx` |
| Editor modes / vim commands (`:w`, `:q`, etc.) | `website/src/content/docs/editor.mdx` |
| Live preview / split pane behavior | `website/src/content/docs/live-preview.mdx` |
| Search / match navigation | `website/src/content/docs/search.mdx` |
| CLI flags / env vars / terminal integration | `website/src/content/docs/configuration.mdx` |
| Installation methods / crate name | `website/src/content/docs/installation.mdx` |
| New features / overview changes | `website/src/content/docs/getting-started.mdx` |

Always check if a TUI change affects any of these docs. When in doubt, grep the website content for the feature name or keybinding being changed.

## Release Process

Releases publish to **crates.io** and **GitHub Releases** simultaneously via a single git tag. They must always stay in sync — never publish one without the other.

### When the user asks to release

Before proceeding, run a full readiness check and present a report:

1. **Changelog preview**: Run `git-cliff --unreleased` to see what will be added to `CHANGELOG.md`. Review the output for completeness and correctness.

2. **CI health**: Check if the latest commit on main passes CI. Run `gh run list --branch main --limit 1` to verify.

3. **Test suite**: Run `cargo test` locally and report the result.

4. **Clippy + fmt**: Run `cargo clippy -- -D warnings` and `cargo fmt --check` to confirm zero warnings/issues.

5. **Version check**: Read the current version from `Cargo.toml` and the last published version from the latest git tag. Suggest the appropriate version bump (patch/minor/major) based on the changes.

6. **Website sync**: Check if any TUI-facing changes were made since the last release that should be reflected in the website docs (use the App ↔ Website Sync table above).

### Report format

Present the report as:

```
## Release Readiness Report

**Current version**: 0.x.y (Cargo.toml)
**Last released version**: v0.x.y (or "none" if first release)
**Suggested next version**: 0.x.y (patch/minor/major because ...)

### Changes since last release
- **Features**: list or "none"
- **Fixes**: list or "none"
- **Other**: list or "none"

### Checks
- [ ] CI passing on main
- [ ] cargo test: X tests passed
- [ ] cargo clippy: clean
- [ ] cargo fmt: clean
- [ ] Website docs in sync (or list what needs updating)

### Recommendation
Release / Don't release (with reason)
```

Wait for explicit user approval before proceeding.

### Executing the release

After approval:

1. **Update the changelog**: Run `git-cliff --tag v{version} -o CHANGELOG.md` to generate the changelog with the new version header. Commit this change: `chore: update changelog for v{version}`.

2. **Release**: The user runs `cargo release <patch|minor|major> --execute` locally. This:
   - Bumps version in `Cargo.toml`
   - Creates a commit: `release: v{version}`
   - Creates a git tag: `v{version}`
   - Publishes to crates.io
   - Pushes commit + tag to origin

3. **CI takes over**: The tag push triggers `.github/workflows/release.yml`, which:
   - Re-runs CI as a gate
   - Publishes to crates.io (idempotent if already published by cargo-release)
   - Cross-compiles binaries for 6 targets (Linux x86_64/musl/aarch64, macOS x86_64/aarch64, Windows x86_64)
   - Creates a GitHub Release with auto-generated notes and attached binaries

### Changelog

The changelog is auto-generated from conventional commits using **git-cliff** (`cliff.toml`). Never edit `CHANGELOG.md` by hand — it is regenerated on each release.

- `git-cliff --unreleased` — preview what will be added next release
- `git-cliff --tag vX.Y.Z -o CHANGELOG.md` — generate changelog with a new version tag
- `git-cliff -o CHANGELOG.md` — regenerate the full changelog

Commit messages must follow [Conventional Commits](https://www.conventionalcommits.org/) format (`feat:`, `fix:`, `docs:`, `perf:`, `refactor:`, `test:`, `chore:`, `ci:`, `style:`). Non-conventional commits are excluded from the changelog.

### Configuration
- `cliff.toml`: git-cliff changelog template and commit parsing rules
- `release.toml`: cargo-release settings (tag format, commit message, allowed branch)
- `CARGO_REGISTRY_TOKEN` GitHub secret: required for CI-side crates.io publish
