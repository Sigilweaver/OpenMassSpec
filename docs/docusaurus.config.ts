import { themes as prismThemes } from 'prism-react-renderer';
import type { Config } from '@docusaurus/types';
import type * as Preset from '@docusaurus/preset-classic';

const config: Config = {
    title: 'OpenProteo',
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
    baseUrl: '/openproteo/docs/',

    organizationName: 'Sigilweaver',
    projectName: 'OpenProteo',

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
                    editUrl: 'https://github.com/Sigilweaver/OpenProteo/tree/main/docs/',
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
            { name: 'keywords', content: 'OpenProteo, mass spectrometry, proteomics, Thermo, Bruker, Waters, mzML, Arrow, Rust, Python' },
            { name: 'description', content: 'OpenProteo is a pure-Rust mass spectrometry I/O stack for Thermo, Bruker, and Waters acquisitions.' },
        ],
        colorMode: {
            defaultMode: 'dark',
            disableSwitch: false,
            respectPrefersColorScheme: true,
        },
        navbar: {
            title: 'OpenProteo',
            logo: {
                alt: 'Sigilweaver logo',
                src: 'img/logo.svg',
                href: 'https://sigilweaver.app',
                target: '_self',
            },
            items: [
                {
                    type: 'docSidebar',
                    sidebarId: 'docsSidebar',
                    position: 'left',
                    label: 'Docs',
                },
                {
                    type: 'dropdown',
                    label: 'Projects',
                    position: 'left',
                    items: [
                        { label: 'All Docs', href: 'https://sigilweaver.app/docs/' },
                        { label: 'OpenTFRaw (Thermo)', href: 'https://sigilweaver.app/opentfraw/docs/' },
                        { label: 'OpenTimsTDF (Bruker)', href: 'https://sigilweaver.app/opentimstdf/docs/' },
                        { label: 'OpenWRaw (Waters)', href: 'https://sigilweaver.app/openwraw/docs/' },
                    ],
                },
                {
                    href: 'https://github.com/Sigilweaver/OpenProteo',
                    label: 'GitHub',
                    position: 'right',
                },
                {
                    href: 'https://sigilweaver.app',
                    label: 'Website',
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
                        { label: 'GitHub', href: 'https://github.com/Sigilweaver/OpenProteo' },
                        { label: 'Issues', href: 'https://github.com/Sigilweaver/OpenProteo/issues' },
                    ],
                },
                {
                    title: 'Vendor readers',
                    items: [
                        { label: 'OpenTFRaw (Thermo)', href: 'https://github.com/Sigilweaver/OpenTFRaw' },
                        { label: 'OpenTDF (Bruker)', href: 'https://github.com/Sigilweaver/OpenTDF' },
                        { label: 'OpenWRaw (Waters)', href: 'https://github.com/Sigilweaver/OpenWRaw' },
                    ],
                },
                {
                    title: 'Sigilweaver',
                    items: [
                        { label: 'Website', href: 'https://sigilweaver.app' },
                        { label: 'Other projects', href: 'https://sigilweaver.app#projects' },
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
            copyright: `Copyright ${new Date().getFullYear()} Sigilweaver Holdings LLC. OpenProteo is Apache-2.0 licensed. Documentation licensed under <a href="https://creativecommons.org/licenses/by-sa/4.0/" target="_blank" rel="noopener noreferrer">CC-BY-SA 4.0</a>.`,
        },
        prism: {
            theme: prismThemes.github,
            darkTheme: prismThemes.dracula,
            additionalLanguages: ['rust', 'toml', 'bash'],
        },
    } satisfies Preset.ThemeConfig,
};

export default config;
