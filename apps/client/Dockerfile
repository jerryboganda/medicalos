# Static client image: nginx serves the SPA + proxies /api/ to the API
# container (same origin — no CORS needed). Built by GitHub Actions
# (AGENTS.md compute policy); the VPS only pulls and runs it.
FROM nginx:1.27-alpine
COPY build /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY security-headers.conf /etc/nginx/conf.d/security-headers.conf
COPY lti-embed-headers.conf /etc/nginx/conf.d/lti-embed-headers.conf
EXPOSE 80
CMD ["sh", "-c", "nginx -t && exec nginx -g 'daemon off;'"]
