# mdt Website

Documentation site for [mdt](https://github.com/ppramanik62/markdown-tui) — a terminal-based markdown viewer/editor built in Rust.

Built with [Astro](https://astro.build/) + [Starlight](https://starlight.astro.build/).

## Development

```bash
cd website
bun install
bun run dev        # Start dev server at localhost:4321
bun run build      # Production build to ./dist/
bun run preview    # Preview production build
bun run lint       # Lint with oxlint
bun run fmt        # Format with oxfmt
bun run fmt:check  # Check formatting
```

## Deployment

Deployed to [Cloudflare Pages](https://pages.cloudflare.com/) via Git integration.

Cloudflare Pages config:
- **Root directory:** `website`
- **Framework preset:** Astro
- **Build command:** `bun run build`
- **Build output directory:** `dist`
