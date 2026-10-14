import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
    title: 'OpenMassSpec',
    tagline: 'Pure-Rust mass spectrometry I/O for Thermo, Bruker, and Waters',
    favicon: 'img/favicon.ico',

    markdown: {
        mermaid: true,
        hooks: {
            onBrokenMarkdownLinks: 'warn',
        },
    },

    themes: ['@docusaurus/theme-mermaid'],

    url: 'https://sigilweaver.app',
    baseUrl: '/openmassspec/docs/',

    organizationName: 'Sigilweaver',
    projectName: 'OpenMassSpec',

    onBrokenLinks: 'throw',

    i18n: {
        defaultLocale: 'en',
        locales: ['en'],
    },

    presets: [
        [
            'classic',
            {
                docs: {
                    routeBasePath: '/',
                    sidebarPath: './sidebars.ts',
                    editUrl: 'https://github.com/Sigilweaver/OpenMassSpec/tree/main/docs/',
                },
                blog: false,
                sitemap: {
                    changefreq: 'weekly',
                    priority: 0.5,
                    filename: 'sitemap.xml',
                },
                theme: {
                    customCss: './src/css/custom.css',
                },
            } satisfies Preset.Options,
        ],
    ],

    themeConfig: {
        metadata: [
            { name: 'keywords', content: 'OpenMassSpec, mass spectrometry, proteomics, Thermo, Bruker, Waters, mzML, Arrow, Rust, Python' },
            { name: 'description', content: 'OpenMassSpec is a pure-Rust mass spectrometry I/O stack for Thermo, Bruker, and Waters acquisitions.' },
        ],
        colorMode: {
            defaultMode: 'dark',
            disableSwitch: false,
            respectPrefersColorScheme: true,
        },
        navbar: {
            title: 'Sigilweaver',
            logo: {
                alt: 'Sigilweaver logo',
                src: 'img/logo.svg',
                href: 'https://sigilweaver.app',
                target: '_self',
            },
            items: [
                {
                    type: 'dropdown',
                    label: 'OpenMassSpec',
                    position: 'left',
                    items: [
                        { label: 'OpenTFRaw (Thermo)', href: 'https://sigilweaver.app/opentfraw/docs/' },
                        { label: 'OpenTimsTDF (Bruker)', href: 'https://sigilweaver.app/opentimstdf/docs/' },
                        { label: 'OpenWRaw (Waters)', href: 'https://sigilweaver.app/openwraw/docs/' },
                    ],
                },
                {
                    label: 'Core',
                    href: 'https://docs.rs/openmassspec-core',
                    position: 'left',
                },
                {
                    href: 'https://github.com/Sigilweaver/OpenMassSpec',
                    label: 'GitHub',
                    position: 'right',
                },
            ],
        },
        footer: {
            style: 'dark',
            links: [
                {
                    title: 'Project',
                    items: [
                        { label: 'GitHub', href: 'https://github.com/Sigilweaver/OpenMassSpec' },
                        { label: 'Issues', href: 'https://github.com/Sigilweaver/OpenMassSpec/issues' },
                    ],
                },
                {
                    title: 'Vendor readers',
                    items: [
                        { label: 'OpenTFRaw (Thermo)', href: 'https://github.com/Sigilweaver/OpenTFRaw' },
                        { label: 'OpenTimsTDF (Bruker)', href: 'https://github.com/Sigilweaver/OpenTimsTDF' },
                        { label: 'OpenWRaw (Waters)', href: 'https://github.com/Sigilweaver/OpenWRaw' },
                    ],
                },
                {
                    title: 'Legal',
                    items: [
                        { label: 'Terms of Use', href: 'https://sigilweaver.app/terms' },
                        { label: 'Privacy Policy', href: 'https://sigilweaver.app/privacy' },
                    ],
                },
            ],
            copyright: `Copyright ${new Date().getFullYear()} Sigilweaver Holdings LLC. OpenMassSpec is Apache-2.0 licensed. Documentation licensed under <a href="https://creativecommons.org/licenses/by-sa/4.0/" target="_blank" rel="noopener noreferrer">CC-BY-SA 4.0</a>.`,
        },
        prism: {
            theme: prismThemes.github,
            darkTheme: prismThemes.dracula,
            additionalLanguages: ['rust', 'toml', 'bash'],
        },
    } satisfies Preset.ThemeConfig,
};

export default config;
