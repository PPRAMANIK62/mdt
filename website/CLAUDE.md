Use Bun as the package manager (`bun install`, `bun run <script>`).

## Stack

- [Astro](https://docs.astro.build/) with [Starlight](https://starlight.astro.build/) for docs
- [starlight-blog](https://github.com/HiDeoo/starlight-blog) plugin for blog posts
- Components are `.astro` files (not React)
- Custom CSS in `src/styles/` (no CSS framework)

## Commands

- `bun run dev` — start dev server
- `bun run build` — production build to `./dist/`
- `bun run preview` — preview production build
- `bun run lint` — lint with oxlint
- `bun run fmt` — format with oxfmt
- `bun run fmt:check` — check formatting

## Code Quality

- Linting: [oxlint](https://oxc.rs/docs/guide/usage/linter) (not ESLint)
- Formatting: [oxfmt](https://oxc.rs/docs/guide/usage/formatter) (not Prettier)
- Pre-commit hooks via husky run lint + format checks
