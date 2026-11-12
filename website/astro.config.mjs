// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import sitemap from '@astrojs/sitemap';

const jsonLd = {
	'@context': 'https://schema.org',
	'@type': 'SoftwareApplication',
	name: 'dusimagez',
	applicationCategory: 'DeveloperApplication',
	operatingSystem: 'Cross-platform',
	description:
		'Offline image compressor — reduce image file size with maximum control. One Rust core, bindings for Rust, JavaScript, Python, Go, PHP, Kotlin and Swift, plus a CLI. No upload, no network at runtime.',
	author: { '@type': 'Person', name: 'dusmamud' },
	offers: { '@type': 'Offer', price: '0', priceCurrency: 'INR' },
	softwareVersion: '0.1.0',
	url: 'https://dusimagez.pages.dev/',
	programmingLanguage: ['Rust', 'JavaScript', 'TypeScript', 'Python', 'Go', 'PHP', 'Kotlin', 'Swift'],
	license: 'https://github.com/dusmamud/dusimagez/blob/main/LICENSE',
};

// https://astro.build/config
export default defineConfig({
	site: 'https://dusimagez.pages.dev',
	integrations: [
		starlight({
			title: 'dusimagez',
			description:
				'Offline image compressor — reduce image file size with maximum control. One Rust core, bindings for Rust, JavaScript, Python, Go, PHP, Kotlin and Swift. No upload, no network at runtime.',
			logo: {
				src: './src/assets/logo.svg',
				alt: 'dusimagez logo',
			},
			favicon: '/favicon.svg',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/dusmamud/dusimagez' },
			],
			head: [
				{ tag: 'meta', attrs: { property: 'og:type', content: 'website' } },
				{ tag: 'meta', attrs: { property: 'og:site_name', content: 'dusimagez' } },
				{ tag: 'meta', attrs: { property: 'og:title', content: 'dusimagez — Offline image compressor' } },
				{
					tag: 'meta',
					attrs: {
						property: 'og:description',
						content:
							'Reduce image file size with maximum control — quality, target size, presets, subsampling, PNG palette, metadata control. Offline, one Rust core, bindings for 7 languages plus a CLI.',
					},
				},
				{ tag: 'meta', attrs: { property: 'og:url', content: 'https://dusimagez.pages.dev/' } },
				{ tag: 'meta', attrs: { property: 'og:image', content: 'https://dusimagez.pages.dev/og.svg' } },
				{ tag: 'meta', attrs: { name: 'twitter:card', content: 'summary_large_image' } },
				{ tag: 'meta', attrs: { name: 'twitter:title', content: 'dusimagez — Offline image compressor' } },
				{
					tag: 'meta',
					attrs: {
						name: 'twitter:description',
						content:
							'Reduce image file size with maximum control. Offline, one Rust core, bindings for 7 languages plus a CLI.',
					},
				},
				{ tag: 'meta', attrs: { name: 'twitter:image', content: 'https://dusimagez.pages.dev/og.svg' } },
				{
					tag: 'script',
					attrs: { type: 'application/ld+json' },
					content: JSON.stringify(jsonLd),
				},
			],
			editLink: {
				baseUrl: 'https://github.com/dusmamud/dusimagez/edit/main/website/',
			},
			sidebar: [
				{
					label: 'Start here',
					items: [
						{ label: 'Overview', slug: 'overview' },
						{ label: 'Compressor guide', slug: 'guide' },
						{ label: 'Comparison', slug: 'comparison' },
						{ label: 'Changelog', slug: 'changelog' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Options', slug: 'options' },
						{ label: 'Presets', slug: 'presets' },
						{ label: 'Formats', slug: 'formats' },
						{ label: 'Error codes', slug: 'errors' },
					],
				},
				{
					label: 'Quickstarts',
					items: [
						{ label: 'Rust', slug: 'quickstart-rust' },
						{ label: 'JavaScript / TypeScript', slug: 'quickstart-js' },
						{ label: 'Python', slug: 'quickstart-python' },
						{ label: 'Go', slug: 'quickstart-go' },
						{ label: 'PHP', slug: 'quickstart-php' },
						{ label: 'Kotlin (Android)', slug: 'quickstart-kotlin' },
						{ label: 'Swift (iOS)', slug: 'quickstart-swift' },
					],
				},
			],
		}),
		sitemap(),
	],
});
