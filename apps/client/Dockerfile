# Static client image: nginx serves the SPA + proxies /api/ to the API
# container (same origin — no CORS needed). Built by GitHub Actions
# (AGENTS.md compute policy); the VPS only pulls and runs it.
FROM nginx:1.27-alpine
COPY build /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
