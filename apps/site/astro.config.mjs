import { defineConfig } from 'astro/config';

// Marketing site (ADR 0002): landing, per-exam pages, pricing, help, legal,
// download and the app-link association files. Holds no learner data; calls
// only public/checkout endpoints (master plan §31.1). Static output only —
// the deployment origin is an open owner decision, so no `site` (and
// therefore no canonical/sitemap absolute URLs) is set yet.
export default defineConfig({});
