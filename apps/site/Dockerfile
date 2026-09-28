# Static marketing-site image: nginx serves the built Astro output.
# Built by GitHub Actions (deploy.yml site-image, AGENTS.md compute policy);
# local Orca staging (docs/deployment/orca-desktop.md) only pulls and runs it.
FROM nginx:1.27-alpine
COPY dist /usr/share/nginx/html
EXPOSE 80
