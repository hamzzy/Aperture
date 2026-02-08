import {themes as prismThemes} from 'prism-react-renderer';
import type {Config} from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
  title: 'Aperture',
  tagline: 'Production-grade distributed profiler for CPU & GPU workloads',
  favicon: 'img/favicon.ico',

  future: {
    v4: true,
  },

  url: 'https://aperture.dev',
  baseUrl: '/',

  organizationName: 'aperture',
  projectName: 'aperture',

  onBrokenLinks: 'throw',

  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'warn',
    },
  },

  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      {
        docs: {
          sidebarPath: './sidebars.ts',
          editUrl: 'https://github.com/yourusername/aperture/tree/main/docs-site/',
        },
        blog: false,
        theme: {
          customCss: './src/css/custom.css',
        },
      } satisfies Preset.Options,
    ],
  ],

  themeConfig: {
    image: 'img/aperture-social-card.png',
    colorMode: {
      defaultMode: 'dark',
      respectPrefersColorScheme: true,
    },
    navbar: {
      title: 'Aperture',
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docsSidebar',
          position: 'left',
          label: 'Docs',
        },
        {
          to: '/docs/api-reference',
          label: 'API',
          position: 'left',
        },
        {
          href: 'https://github.com/yourusername/aperture',
          label: 'GitHub',
          position: 'right',
        },
      ],
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Documentation',
          items: [
            {label: 'Getting Started', to: '/docs/getting-started'},
            {label: 'Architecture', to: '/docs/architecture'},
            {label: 'API Reference', to: '/docs/api-reference'},
          ],
        },
        {
          title: 'Guides',
          items: [
            {label: 'Run Examples', to: '/docs/guides/run-examples'},
            {label: 'Symbol Resolution', to: '/docs/guides/symbol-resolution'},
            {label: 'Kubernetes', to: '/docs/guides/kubernetes'},
          ],
        },
        {
          title: 'More',
          items: [
            {
              label: 'GitHub',
              href: 'https://github.com/yourusername/aperture',
            },
          ],
        },
      ],
      copyright: `Copyright ${new Date().getFullYear()} Aperture Contributors. Built with Docusaurus.`,
    },
    prism: {
      theme: prismThemes.github,
      darkTheme: prismThemes.dracula,
      additionalLanguages: ['bash', 'rust', 'toml', 'protobuf', 'json', 'yaml'],
    },
  } satisfies Preset.ThemeConfig,
};

export default config;
