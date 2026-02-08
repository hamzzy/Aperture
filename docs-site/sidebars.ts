import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  docsSidebar: [
    'intro',
    'getting-started',
    'architecture',
    {
      type: 'category',
      label: 'Guides',
      items: [
        'guides/run-examples',
        'guides/symbol-resolution',
        'guides/wasm-filters',
        'guides/kubernetes',
        'guides/alerting',
      ],
    },
    'api-reference',
    'roadmap',
  ],
};

export default sidebars;
