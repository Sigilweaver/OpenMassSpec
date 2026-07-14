import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
    docsSidebar: [
        'intro',
        {
            type: 'category',
            label: 'Getting started',
            collapsed: false,
            items: [
                'install',
                'quickstart-cli',
                'quickstart-rust',
                'quickstart-python',
            ],
        },
        {
            type: 'category',
            label: 'Reference',
            items: [
                'core',
                'format-detection',
                'conformance',
                'arrow-schema',
            ],
        },
        {
            type: 'category',
            label: 'Vendors',
            items: [
                'vendor-thermo',
                'vendor-bruker',
                'vendor-waters',
                'vendor-agilent',
                'vendor-sciex',
            ],
        },
        {
            type: 'category',
            label: 'Design',
            items: [
                'design-architecture',
                'design-crates',
                'design-pure-rust',
            ],
        },
    ],
};

export default sidebars;
