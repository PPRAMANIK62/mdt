# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-03-19

### Bug Fixes

- Bug fixes and README.md([4202735](https://github.com/PPRAMANIK62/mdt/commit/42027358edf1e725a500ef1839f21963f596ada6))
- Replace unwrap with idiomatic pattern matching in handle_enter([d7bffe8](https://github.com/PPRAMANIK62/mdt/commit/d7bffe81666b680ac6b9112891a1dd3ed1e07610))
- Address all 17 audit findings across codebase([cb620bd](https://github.com/PPRAMANIK62/mdt/commit/cb620bdfa1336c944b2d49445b3dead3a827236f))
- Tui layout([309f956](https://github.com/PPRAMANIK62/mdt/commit/309f956fe5a6ef45d5202947fe1883a7325b1312))
- **preview**: Remove Paragraph.wrap, completing renderer-level wrapping([bb67206](https://github.com/PPRAMANIK62/mdt/commit/bb67206300374d57249b655697b29fde7c81024e))
- Line wrapping in tables([dec18fd](https://github.com/PPRAMANIK62/mdt/commit/dec18fdd019bf47cb44f3f34ba9123e99133e4f3))
- **main**: Add panic hook to restore terminal on panic([6941324](https://github.com/PPRAMANIK62/mdt/commit/694132425f794fd4f1751f253dbd887f01b78de1))
- **markdown**: Fix redundant closure clippy warning([db4032c](https://github.com/PPRAMANIK62/mdt/commit/db4032cb3290f2862ed5b646c3a37a43953b4705))
- **markdown**: Cache NO_COLOR with OnceLock and fix correctness per standard([f80e916](https://github.com/PPRAMANIK62/mdt/commit/f80e91667f48220ef42c285f97445cb129fce5be))
- Fixing layout and colors([ead30bc](https://github.com/PPRAMANIK62/mdt/commit/ead30bcba2522aa8b7d2d5ad8ea8e004a26ab4bc))
- Eliminate TOCTOU races in file ops and handle binary files([2a932dc](https://github.com/PPRAMANIK62/mdt/commit/2a932dc24fe0efb7b0ec2537d7959ded9fcc0539))
- Address bad practices and performance issues across codebase (#2)([dc20438](https://github.com/PPRAMANIK62/mdt/commit/dc2043842a2b1528b220b7de02a814469a103ebe))
- Prevent focus on hidden file tree and remove startup blocking join([705aef5](https://github.com/PPRAMANIK62/mdt/commit/705aef54f95fa5feae890f3bcabb22b57170f89d))
- Use derive(Default) for SplitOrientation and LivePreviewState([e6deedc](https://github.com/PPRAMANIK62/mdt/commit/e6deedcd791ff017704e02c97bbab6ef563cd71f))
- Parallel scroll on editor and preview([d8b97f6](https://github.com/PPRAMANIK62/mdt/commit/d8b97f6f4a4fefd1ae1d81535e0d295bac8ac026))
- Fmt([597f43e](https://github.com/PPRAMANIK62/mdt/commit/597f43e2e55d9b2263089462a2882bddf324e0b0))

### CI

- Add GitHub Actions workflow for test, lint, format([0e9ed30](https://github.com/PPRAMANIK62/mdt/commit/0e9ed300f142d7ba893c1d6e5f2e70bca758254d))

### Documentation

- **preview**: Document intentional viewport side effect pattern([41d09cc](https://github.com/PPRAMANIK62/mdt/commit/41d09cc9b4d139f3ce2f75e9aacef80d4938d1b2))
- Rewrite README with comprehensive feature docs, grouped keybindings, and dependency list([02eadb9](https://github.com/PPRAMANIK62/mdt/commit/02eadb9aaf512127dfd9ec4901fe0df569b6f065))
- Add app screenshots to README([a6a5c24](https://github.com/PPRAMANIK62/mdt/commit/a6a5c24f4e0f0f6f9200cfc388115440fee7e73c))
- Replace static screenshots with demo.gif in README([8af9ff1](https://github.com/PPRAMANIK62/mdt/commit/8af9ff113df43b0712d737cd98434364fdc26a2f))
- Add live preview feature design spec and implementation plan([8668a62](https://github.com/PPRAMANIK62/mdt/commit/8668a621e72ed094c3a0a8d17787869225549487))
- Update README with live preview, fuzzy finder, file watching, and fix max file size([03621d6](https://github.com/PPRAMANIK62/mdt/commit/03621d606c74336907b0d9835780a175631220eb))
- Update website URL to mdtui.pages.dev([b3154f2](https://github.com/PPRAMANIK62/mdt/commit/b3154f2ec3e89ceb8cf91618f6f217e6e224177d))

### Features

- Scaffold mdt project with ratatui + tui-markdown dependencies([dfabdf2](https://github.com/PPRAMANIK62/mdt/commit/dfabdf21e17bf7bfb7cb2bf691b4cb91b14d580e))
- Integrate tui-markdown for styled markdown rendering([ac377b7](https://github.com/PPRAMANIK62/mdt/commit/ac377b7769f2dcabcce59a19fff1203facde633e))
- Add file tree module with directory scanning and navigation([3c68621](https://github.com/PPRAMANIK62/mdt/commit/3c68621684f4a92989c351b887f87cfbd0ae20c0))
- Add app state, event loop with dirty flag, and split layout([7f1a70c](https://github.com/PPRAMANIK62/mdt/commit/7f1a70c61a950b750d9e80718b256e01b462de5c))
- Add vim-style keybindings with mode system and pending key buffer([6183646](https://github.com/PPRAMANIK62/mdt/commit/618364693dbd6b839fc8240b25a94ef829e53858))
- Add text editor with ratatui-textarea, file saving, and unsaved changes protection([1f42e8d](https://github.com/PPRAMANIK62/mdt/commit/1f42e8da38abce8f770161185b84c241573e4ec9))
- Replace tui-markdown with custom pulldown-cmark renderer for native-style markdown([5f8bc79](https://github.com/PPRAMANIK62/mdt/commit/5f8bc7932465cb3b223f9ebf4dfa3c632b68ace6))
- **markdown**: Replace RGB syntax highlighting with ANSI terminal colors and semantic token system([0adaff5](https://github.com/PPRAMANIK62/mdt/commit/0adaff589b8cd5e8aa3ac92dd8884f701d5c92ea))
- Change background color([2eb5f39](https://github.com/PPRAMANIK62/mdt/commit/2eb5f39de4742f4edbb64c1ba1d28e5ed318d6e5))
- File tree redesign([0ede1ce](https://github.com/PPRAMANIK62/mdt/commit/0ede1ce0a2a885d33a93decf5865dad7b27de8a6))
- **markdown**: Add wrap_spans utility and thread available_width through renderer([c67f255](https://github.com/PPRAMANIK62/mdt/commit/c67f255ecc8283fe5fa1488c98c2a989ad27daab))
- **markdown**: Width-aware wrapping for paragraphs, headings, blockquotes, lists([8a3a339](https://github.com/PPRAMANIK62/mdt/commit/8a3a33970b8cd2583e07cbb6c2de4c71fc47593e))
- **markdown**: Code block and table truncation, dynamic HR width([678084c](https://github.com/PPRAMANIK62/mdt/commit/678084c1f6ed61258a95651341a5f0c0268f240e))
- **preview**: Track viewport width and re-render on resize([d2a3b3e](https://github.com/PPRAMANIK62/mdt/commit/d2a3b3eaa460308aec5db7c14f319893e971ef27))
- Add link picker with search filtering and active search match highlighting([5b0ca0c](https://github.com/PPRAMANIK62/mdt/commit/5b0ca0c954c9abcf9bf9eabdbd0826ff13cedfab))
- **ui**: Add modal theme constants and reusable modal utilities([f974177](https://github.com/PPRAMANIK62/mdt/commit/f9741772678d4f0c342c22ff7848bbd7aed29091))
- Add file tree management (create, delete, rename, move)([d827ff6](https://github.com/PPRAMANIK62/mdt/commit/d827ff6b2837bf480d480b3cc73ccb000a54611f))
- Add CLI flags, file locking, integration tests, and misc fixes([165e969](https://github.com/PPRAMANIK62/mdt/commit/165e969064d83327043ea23d7fbf2d9e4e60caa7))
- Add scrollbar, mouse support, and heading jump navigation([d341615](https://github.com/PPRAMANIK62/mdt/commit/d341615080f50ae53f3783c42f3c496ec65197aa))
- Add fuzzy file finder with modular input handling([e403ef0](https://github.com/PPRAMANIK62/mdt/commit/e403ef02e0f6908d1a05d836d704976e4ef530fc))
- Add SplitOrientation enum and LivePreviewState struct([0257610](https://github.com/PPRAMANIK62/mdt/commit/0257610da7d4e220a31d6c3f26c1271847921556))
- Add toggle and update methods for live preview([5304be6](https://github.com/PPRAMANIK62/mdt/commit/5304be634b132c4a9ea7071af528715e77d6db19))
- Add Space+p and Space+s keybindings for live preview([31187b1](https://github.com/PPRAMANIK62/mdt/commit/31187b1a52d1cb7ab651419b507d544c9d5b3f07))
- Add :preview command to toggle live preview([51facc9](https://github.com/PPRAMANIK62/mdt/commit/51facc9635a9e4c6bac46670a976f02880a8e4ad))
- Set debounce timer on insert-mode keystrokes for live preview([4c2c53b](https://github.com/PPRAMANIK62/mdt/commit/4c2c53b86ae301df6dc2cb515bf86d0ec5909942))
- Add debounce check for live preview in event loop([8b28a50](https://github.com/PPRAMANIK62/mdt/commit/8b28a50dd32d5a9ac77f65e6cebe396b8ab7e2f3))
- Add draw_live_preview() for split-pane rendering([1f48798](https://github.com/PPRAMANIK62/mdt/commit/1f487981e29240d27979c2856167b8959d7a8023))
- Split editor area for live preview in draw()([4844894](https://github.com/PPRAMANIK62/mdt/commit/4844894054b050a0c93840a53e3d8e95c7bfc48d))
- Show preview indicator in status bar([17d69bf](https://github.com/PPRAMANIK62/mdt/commit/17d69bf00de0e4d978e357ecf741dbf767915497))
- Add live preview keybindings to help overlay([bd96205](https://github.com/PPRAMANIK62/mdt/commit/bd962059d2a44962f8fefc166f4a98edb490a50e))
- Render live preview on entering editor, cleanup on exit([3825588](https://github.com/PPRAMANIK62/mdt/commit/3825588ffb7ac8f3ceae57af69eb26a5cbe6e4ee))
- Added images and logo([300fa69](https://github.com/PPRAMANIK62/mdt/commit/300fa694a65d999ae18f91add93c8d9640f2b0c2))
- Add documentation website and configure monorepo([5b14467](https://github.com/PPRAMANIK62/mdt/commit/5b14467ea46622e92a16330ed1d845d67a83c59e))

### Miscellaneous

- Add rustfmt and clippy configuration([ede974f](https://github.com/PPRAMANIK62/mdt/commit/ede974fe5ec3a6f59f80da45f33c002d33a11470))
- Remove stale #[allow(dead_code)] from wrap_spans([18dbd47](https://github.com/PPRAMANIK62/mdt/commit/18dbd4723b9442dba3c1d9d5685f8be4bc3d60ac))
- **file_tree**: Increase DIR_SCAN_MAX_DEPTH to 5([a7cf39b](https://github.com/PPRAMANIK62/mdt/commit/a7cf39b6c793dce3edc0263734574a693c467cc5))
- **cargo**: Remove unnecessary lint suppressions([3c48485](https://github.com/PPRAMANIK62/mdt/commit/3c484856ae26fbe12c2dd673cc6d2fbcef76cfe4))
- Add release infrastructure and bump to v0.3.0([e9aaa06](https://github.com/PPRAMANIK62/mdt/commit/e9aaa06c28d6e6c9b3e9aef748fc626a59e2d5e0))

### Performance

- **preview**: Reduce String allocations in highlight_span([d9dc6e6](https://github.com/PPRAMANIK62/mdt/commit/d9dc6e6fab7522b4b8364fecbf15376f3176eae5))
- **markdown**: Reduce per-grapheme allocation in wrap_spans([4a14776](https://github.com/PPRAMANIK62/mdt/commit/4a14776e0eeae4191d65467025775a15809c6964))
- **search**: Reduce allocations in search, filter, and rendering hot paths([d6a7d57](https://github.com/PPRAMANIK62/mdt/commit/d6a7d570e428869cb8f99cfd7d9c998bf42199a0))
- Split render pipeline and fix 7 performance bottlenecks([c05ace0](https://github.com/PPRAMANIK62/mdt/commit/c05ace038cb9619a2957734f6440a77bca930db1))
- Reduce allocations, cache hot paths, and cut input latency (#1)([70549cc](https://github.com/PPRAMANIK62/mdt/commit/70549cc3bc766e315312120dd7d7e910f62a80bc))
- Pre-warm syntect regex compilation to eliminate file open delay([95b241c](https://github.com/PPRAMANIK62/mdt/commit/95b241c131847616f562c3b3b4f72f1c440e8702))

### Refactor

- Remove duplicate tests and dead needs_redraw field([26a4604](https://github.com/PPRAMANIK62/mdt/commit/26a4604f63f94974488d48ecec894d7108ca1175))
- Remove dead FileTree/FileEntry/scan_dir code([52e6daa](https://github.com/PPRAMANIK62/mdt/commit/52e6daa0e13ffb8066ef75dec680048f33f95779))
- Split app.rs into input/ handler modules([3de93e9](https://github.com/PPRAMANIK62/mdt/commit/3de93e94e765373fa63071ca07f1d95773573592))
- Extract status bar rendering to ui/status_bar.rs([c2fd48c](https://github.com/PPRAMANIK62/mdt/commit/c2fd48c6761c65a465e60c6b21a9034827356b13))
- **test**: Switch TempTestDir to tempfile crate([a3a9d7e](https://github.com/PPRAMANIK62/mdt/commit/a3a9d7ebdc4f99c96069a21039da4701928c5c4e))
- **markdown**: Extract BLOCKQUOTE_INDENT_COLS constant([19db24d](https://github.com/PPRAMANIK62/mdt/commit/19db24d76c2d2d9149f82bd8dbf268c734dac364))
- **editor**: Save_editor returns Result instead of bool([67bb6c8](https://github.com/PPRAMANIK62/mdt/commit/67bb6c8f8b38c64cd12927e8ba3a2895d553b21b))
- **markdown**: Split monolithic markdown.rs into focused submodules([1a7c1d6](https://github.com/PPRAMANIK62/mdt/commit/1a7c1d6498c2bda28bfdbfee0ca006542060803d))
- **app**: Move scroll methods to DocumentState and relocate open_file([d2588fe](https://github.com/PPRAMANIK62/mdt/commit/d2588fef64b03020d3de41baa5e7a929b4d76dfe))
- **ui**: Redesign help and links overlays with rounded borders and dimming([9ae7fca](https://github.com/PPRAMANIK62/mdt/commit/9ae7fcab442bde49a8e8f44d74a48cf468653872))
- **ui**: Use only terminal-safe ANSI colors, remove modal dim/shadow([ea7b3ab](https://github.com/PPRAMANIK62/mdt/commit/ea7b3ab47a06c40433ea713881940be127120803))
- **ui**: Improve modal system, help overlay colors, and link handling([05bca1c](https://github.com/PPRAMANIK62/mdt/commit/05bca1ca0ac9201afb9a0396d1bfea2f76254790))
- Split monolithic modules into focused submodules([aa5770c](https://github.com/PPRAMANIK62/mdt/commit/aa5770ce9b72883299061dae415770a382de0d5e))

### Styling

- Normalize formatting with cargo fmt([d487847](https://github.com/PPRAMANIK62/mdt/commit/d487847573b4d14fc94f3ecf841aeb94a151339e))
- Apply rustfmt formatting([b8eee90](https://github.com/PPRAMANIK62/mdt/commit/b8eee90df344223e9a813dded4e79ff33dd0fa90))

### Testing

- Add App state machine transition tests([3e02b69](https://github.com/PPRAMANIK62/mdt/commit/3e02b69cb0e0c38b8dc6ac480e3e6475fd168d78))
- Add input dispatch tests for handler modules([e42e7e6](https://github.com/PPRAMANIK62/mdt/commit/e42e7e649d47b4eb0edca99ba99eebf65305d4f1))
- **input**: Add search/scroll/navigation tests([9f912cf](https://github.com/PPRAMANIK62/mdt/commit/9f912cf8ae12d7ca283f301d22e8826f4ff27a31))
- **ui**: Add TestBackend render tests for preview([28447e1](https://github.com/PPRAMANIK62/mdt/commit/28447e1aa7ffd1ce36e892f4ab7b53f4ab9157ef))
- Add 27 tests for file watching and auto-reload feature([7159445](https://github.com/PPRAMANIK62/mdt/commit/715944550142e2d9b18ffab8efe30802d7ef0bc1))
- Add comprehensive unit tests across app and input modules([2e262ca](https://github.com/PPRAMANIK62/mdt/commit/2e262cab91f10e44c76abe07b642b909e03a76e1))

