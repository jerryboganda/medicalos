/// <reference lib="webworker" />

import { base } from '$app/paths';
import { build, files, version } from '$service-worker';

const SHELL_CACHE = `medical-os-shell-${version}`;
const STATIC_CACHE = `medical-os-static-${version}`;
const offlinePath = `${base}/offline` || '/offline';
const apiPath = '/api/';
const sw = self as unknown as ServiceWorkerGlobalScope;

sw.addEventListener('install', (event) => {
	event.waitUntil(
		Promise.all([
			caches.open(STATIC_CACHE).then((cache) =>
				cache.addAll([...build, ...files].map((path) => new URL(path, sw.location.origin).href))
			),
			caches.open(SHELL_CACHE).then((cache) =>
				cache.add(new URL(offlinePath, sw.location.origin).href)
			)
		]).then(() => sw.skipWaiting())
	);
});

sw.addEventListener('activate', (event) => {
	event.waitUntil(
		caches.keys().then(async (names) => {
			await Promise.all(
				names
					.filter((name) =>
						(name.startsWith('medical-os-shell-') && name !== SHELL_CACHE) ||
						(name.startsWith('medical-os-static-') && name !== STATIC_CACHE)
					)
					.map((name) => caches.delete(name))
			);
			await sw.clients.claim();
		})
	);
});

sw.addEventListener('fetch', (event) => {
	const request = event.request;
	if (request.method !== 'GET') return;
	const url = new URL(request.url);
	if (
		url.origin !== sw.location.origin ||
		url.pathname === '/api' ||
		url.pathname.startsWith(apiPath) ||
		url.pathname.startsWith('/v1/') ||
		url.pathname.startsWith('/v2/')
	) return;

	if (request.mode === 'navigate') {
		event.respondWith(
			fetch(request).then(async (response) => {
				if (response.ok && url.pathname === offlinePath) {
					const cache = await caches.open(SHELL_CACHE);
					await cache.put(new URL(offlinePath, sw.location.origin).href, response.clone());
				}
				return response;
			}).catch(async () => {
				const cached = await caches.match(new URL(offlinePath, sw.location.origin).href);
				return cached ?? Response.error();
			})
		);
		return;
	}

	if (url.pathname.includes('/_app/immutable/') || [...files].some((path) => new URL(path, sw.location.origin).pathname === url.pathname)) {
		event.respondWith(
			caches.match(request).then((cached) => cached ?? fetch(request).then(async (response) => {
				if (response.ok) {
					const cache = await caches.open(STATIC_CACHE);
					await cache.put(request, response.clone());
				}
				return response;
			}))
		);
	}
});
