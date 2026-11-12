import { defineCollection } from 'astro:content';
import { docsLoader } from '@astrojs/starlight/loaders';
import { docsSchema } from '@astrojs/starlight/schema';

// Starlight 0.34 + Astro 5: the docs collection must be defined explicitly
// with Starlight's loader AND schema. Without the loader, Astro falls back
// to legacy file handling, schema defaults (e.g. `draft: false`) are not
// applied, and Starlight's sidebar lookup (`data.draft === false`) finds
// no routes — the build fails with 'The slug "overview" ... does not exist'.
export const collections = {
	docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
};
