import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightBlog from 'starlight-blog';

export default defineConfig({
  site: 'https://mdt.purbayan.me',
  integrations: [
    starlight({
      title: 'mdt',
      description:
        'A fast, terminal-based markdown viewer and editor built with Rust.',
      logo: {
        src: './src/assets/logo.png',
        replacesTitle: false,
      },
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/PPRAMANIK62/mdt',
        },
      ],
      editLink: {
        baseUrl:
          'https://github.com/PPRAMANIK62/mdt/edit/main/website/',
      },
      customCss: ['./src/styles/global.css'],
      sidebar: [
        { slug: 'getting-started' },
        { slug: 'installation' },
        {
          label: 'Features',
          items: [
            { slug: 'file-browser' },
            { slug: 'editor' },
            { slug: 'live-preview' },
            { slug: 'search' },
          ],
        },
        { slug: 'keybindings' },
        { slug: 'configuration' },
      ],
      plugins: [
        starlightBlog({
          title: 'Blog',
          authors: {
            ppramanik: {
              name: 'Purbayan Pramanik',
              url: 'https://github.com/PPRAMANIK62',
            },
          },
        }),
      ],
    }),
  ],
});
