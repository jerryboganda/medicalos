<script lang="ts">
	// Institution single sign-on: load and save an institution's OIDC provider.
	// INST-03: provider config is global-admin gated; secrets are write-only.
	import { base } from '$app/paths';
	import { Api, ApiError } from '$lib/api';

	let { refreshAudit }: { refreshAudit: () => Promise<void> } = $props();

	let oidcInstitutionId = $state('');
	let oidcIssuer = $state('');
	let oidcClientId = $state('');
	let oidcClientSecret = $state('');
	let oidcEnabled = $state('false');
	let oidcSecretConfigured = $state(false);
	let oidcClearSecret = $state(false);
	let oidcLoaded = $state(false);
	let oidcBusy = $state(false);
	let oidcMessage = $state('');
	let oidcError = $state('');
	let oidcSignInUrl = $state('');

	async function loadOidcProvider() {
		if (oidcBusy || !oidcInstitutionId.trim()) return;
		oidcBusy = true;
		oidcError = '';
		oidcMessage = '';
		oidcSignInUrl = '';
		try {
			const provider = await Api.getInstitutionOidc(oidcInstitutionId.trim());
			oidcIssuer = provider.issuer;
			oidcClientId = provider.client_id;
			oidcEnabled = String(provider.enabled);
			oidcSecretConfigured = provider.client_secret_configured;
			oidcClearSecret = false;
			oidcLoaded = true;
			oidcMessage = 'Provider settings loaded.';
		} catch (err) {
			if (err instanceof ApiError && err.code === 'oidc_provider_not_found') {
				oidcIssuer = '';
				oidcClientId = '';
				oidcEnabled = 'false';
				oidcSecretConfigured = false;
				oidcClearSecret = false;
				oidcLoaded = true;
				oidcMessage = 'No provider is configured yet.';
			} else {
				oidcError = err instanceof ApiError ? err.message : 'Provider settings could not be loaded.';
			}
		} finally {
			oidcBusy = false;
		}
	}

	async function saveOidcProvider(event: Event) {
		event.preventDefault();
		if (oidcBusy) return;
		oidcBusy = true;
		oidcError = '';
		oidcMessage = '';
		oidcSignInUrl = '';
		try {
			const provider = await Api.configureInstitutionOidc(oidcInstitutionId.trim(), {
				issuer: oidcIssuer.trim(),
				client_id: oidcClientId.trim(),
				...(oidcClientSecret ? { client_secret: oidcClientSecret } : {}),
				clear_client_secret: oidcClearSecret,
				enabled: oidcEnabled === 'true'
			});
			oidcSecretConfigured = provider.client_secret_configured;
			oidcEnabled = String(provider.enabled);
			oidcClientSecret = '';
			oidcClearSecret = false;
			oidcMessage = 'Provider settings saved.';
			if (provider.enabled) {
				oidcSignInUrl = `${window.location.origin}${base}/login?institution=${encodeURIComponent(oidcInstitutionId.trim())}`;
			}
			await refreshAudit();
		} catch (err) {
			oidcError = err instanceof ApiError ? err.message : 'Provider settings could not be saved.';
		} finally {
			oidcBusy = false;
		}
	}
</script>

<div class="card" data-testid="oidc-provider-settings">
	<h2>Institution single sign-on</h2>
	<p class="muted">
		Configure an OIDC provider for an institution. Learners must already have an exact issuer and
		subject enrollment; accounts are never linked by email.
	</p>
	<label class="field" for="oidc-institution-id">
		<span>Institution ID</span>
		<input id="oidc-institution-id" bind:value={oidcInstitutionId} data-testid="oidc-institution-id" />
	</label>
	<button
		class="btn"
		type="button"
		disabled={oidcBusy || !oidcInstitutionId.trim()}
		data-testid="oidc-load"
		onclick={loadOidcProvider}
	>
		{oidcBusy ? 'Loading…' : 'Load provider settings'}
	</button>
	{#if oidcMessage}<p class="muted" role="status">{oidcMessage}</p>{/if}
	{#if oidcError}<p class="error-text" role="alert">{oidcError}</p>{/if}
	{#if oidcLoaded}
		<form onsubmit={saveOidcProvider}>
			<label class="field" for="oidc-issuer">
				<span>Issuer URL</span>
				<input id="oidc-issuer" type="url" bind:value={oidcIssuer} required data-testid="oidc-issuer" />
			</label>
			<label class="field" for="oidc-client-id">
				<span>Client ID</span>
				<input id="oidc-client-id" bind:value={oidcClientId} required data-testid="oidc-client-id" />
			</label>
			<label class="field" for="oidc-client-secret">
				<span>Client secret (optional)</span>
				<input
					id="oidc-client-secret"
					type="password"
					autocomplete="new-password"
					bind:value={oidcClientSecret}
					data-testid="oidc-client-secret"
				/>
			</label>
		<p class="muted">
			{oidcSecretConfigured
				? 'A secret is stored. Leave this blank to keep it.'
				: 'No secret is stored. A public client can use PKCE.'}
		</p>
		{#if oidcSecretConfigured}
			<button
				class="linklike"
				type="button"
				disabled={oidcBusy || !!oidcClientSecret}
				onclick={() => (oidcClearSecret = !oidcClearSecret)}
			>
				{oidcClearSecret ? 'Keep stored secret' : 'Clear stored secret'}
			</button>
		{/if}
		<label class="field" for="oidc-enabled">
			<span>Sign-in status</span>
			<select id="oidc-enabled" bind:value={oidcEnabled} data-testid="oidc-enabled">
				<option value="false">Disabled</option>
				<option value="true">Enabled</option>
			</select>
		</label>
		<button
			class="btn primary"
			type="submit"
			disabled={oidcBusy || !oidcInstitutionId.trim() || !oidcIssuer.trim() || !oidcClientId.trim()}
			data-loading={oidcBusy}
			data-testid="oidc-save"
		>
			{oidcBusy ? 'Saving…' : 'Save provider settings'}
		</button>
		{#if oidcSignInUrl}
			<p class="muted" role="status">
				<a href={oidcSignInUrl}>Open learner sign-in</a>
			</p>
		{/if}
		</form>
	{/if}
</div>
