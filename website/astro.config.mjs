import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Project GitHub Pages site: https://calvinchengx.github.io/otogo/
//
// `base` and the deploy workflow's upload path have to change together: a base
// that does not match where the artifact is served resolves every internal
// link and asset one directory off, and Starlight still builds cleanly, so the
// mistake only shows up in the published site.
export default defineConfig({
  site: 'https://calvinchengx.github.io',
  base: '/otogo/',
  integrations: [
    starlight({
      title: 'otogo',
      description:
        'Goal loops where the agent owns the turns and a person owns the rounds. Every claim names its witnesses — the loop can add evidence, never rewrite it.',
      social: [
        { icon: 'github', label: 'GitHub', href: 'https://github.com/calvinchengx/otogo' },
      ],
      editLink: {
        baseUrl: 'https://github.com/calvinchengx/otogo/edit/main/docs/',
      },
      sidebar: [
        {
          label: 'Getting started',
          items: [{ slug: '01-quickstart' }, { slug: '02-the-round' }],
        },
        {
          label: 'How it holds',
          items: [
            { slug: '03-authority' },
            { slug: '04-witnesses' },
            { slug: '05-configuration' },
            { slug: '06-skills' },
          ],
        },
        {
          label: 'Examples',
          items: [
            { slug: '07-example-emulator-parity' },
            { slug: '08-example-agent-product' },
          ],
        },
        {
          label: 'Project',
          items: [{ slug: '09-beyond-feature-work' }, { slug: '10-testing' }],
        },
      ],
    }),
  ],
});
