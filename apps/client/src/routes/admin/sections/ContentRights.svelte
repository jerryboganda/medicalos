<script lang="ts">
	// Content rights ledger: record grants (uses, contract and scope terms) and revoke them. The list
	// is page-owned because extraction QA picks active grants from it. No requirement ID cited
	// inline; e2e: lib06-private-imports.spec.ts.
	import { onMount } from 'svelte';
	import { Api, ApiError, type AdminContentRight } from '$lib/api';

	let {
		contentRights = $bindable(),
		refreshAudit
	}: { contentRights: AdminContentRight[]; refreshAudit: () => Promise<void> } = $props();

	let rightsBusy = $state(false);
	let rightsError = $state('');
	let rightsMessage = $state('');
	let rightsRefCode = $state('');
	let rightsLicensor = $state('');
	let rightsTerritory = $state('worldwide');
	let rightsPermittedUses = $state<string[]>(['display']);
	let rightsValidFrom = $state('');
	let rightsValidTo = $state('');
	let rightsNotes = $state('');
	let rightsContractRef = $state('');
	let rightsContractVersion = $state('');
	let rightsAssetRefs = $state('');
	let rightsAudiences = $state('');
	let rightsSeatLimit = $state<number | undefined>();
	let rightsOfflineTerms = $state('');
	let rightsQuotationLimit = $state<number | undefined>();
	let rightsAiTerms = $state('');
	let rightsDerivativeTerms = $state('');
	let rightsAttribution = $state('');
	let rightsRoyaltyTerms = $state('');
	let revocationReasons = $state<Record<string, string>>({});
	let revokingRightsId = $state('');

	const contentRightUses = [
		{ value: 'display', label: 'Display' },
		{ value: 'search', label: 'Search' },
		{ value: 'offline', label: 'Offline use' },
		{ value: 'embeddings', label: 'Embeddings' },
		{ value: 'ai', label: 'AI use' },
		{ value: 'derivatives', label: 'Derivatives' },
		{ value: 'translation', label: 'Translation' },
		{ value: 'private_import', label: 'Private learner imports' },
		{ value: 'document_extraction', label: 'Document extraction' }
	];

	async function loadContentRights() {
		rightsBusy = true;
		rightsError = '';
		try {
			contentRights = (await Api.listContentRights()).rights;
		} catch (err) {
			rightsError = err instanceof ApiError ? err.message : 'Content rights could not be loaded.';
		} finally {
			rightsBusy = false;
		}
	}

	function toggleContentRightUse(value: string, checked: boolean) {
		rightsPermittedUses = checked
			? [...new Set([...rightsPermittedUses, value])]
			: rightsPermittedUses.filter((use) => use !== value);
	}

	function splitLines(value: string) {
		return value
			.split(/\r?\n/)
			.map((item) => item.trim())
			.filter(Boolean);
	}

	async function createContentRight(event: Event) {
		event.preventDefault();
		if (rightsBusy || rightsPermittedUses.length === 0) return;
		rightsBusy = true;
		rightsError = '';
		rightsMessage = '';
		try {
			const result = await Api.createContentRights({
				ref_code: rightsRefCode.trim(),
				licensor: rightsLicensor.trim(),
				territory: rightsTerritory.trim() || 'worldwide',
				permitted_uses: rightsPermittedUses,
				valid_from: rightsValidFrom,
				...(rightsValidTo ? { valid_to: rightsValidTo } : {}),
				...(rightsNotes.trim() ? { notes: rightsNotes.trim() } : {}),
				...(rightsContractRef.trim() ? { contract_ref: rightsContractRef.trim() } : {}),
				...(rightsContractVersion.trim()
					? { contract_version: rightsContractVersion.trim() }
					: {}),
				asset_refs: splitLines(rightsAssetRefs),
				audiences: splitLines(rightsAudiences),
				...(rightsSeatLimit !== undefined ? { seat_limit: rightsSeatLimit } : {}),
				...(rightsOfflineTerms.trim() ? { offline_terms: rightsOfflineTerms.trim() } : {}),
				...(rightsQuotationLimit !== undefined
					? { quotation_limit_words: rightsQuotationLimit }
					: {}),
				...(rightsAiTerms.trim() ? { ai_terms: rightsAiTerms.trim() } : {}),
				...(rightsDerivativeTerms.trim()
					? { derivative_terms: rightsDerivativeTerms.trim() }
					: {}),
				...(rightsAttribution.trim() ? { attribution: rightsAttribution.trim() } : {}),
				...(rightsRoyaltyTerms.trim() ? { royalty_terms: rightsRoyaltyTerms.trim() } : {})
			});
			rightsRefCode = '';
			rightsLicensor = '';
			rightsTerritory = 'worldwide';
			rightsValidFrom = '';
			rightsValidTo = '';
			rightsNotes = '';
			rightsContractRef = '';
			rightsContractVersion = '';
			rightsAssetRefs = '';
			rightsAudiences = '';
			rightsSeatLimit = undefined;
			rightsOfflineTerms = '';
			rightsQuotationLimit = undefined;
			rightsAiTerms = '';
			rightsDerivativeTerms = '';
			rightsAttribution = '';
			rightsRoyaltyTerms = '';
			rightsMessage = 'Rights record ' + result.ref_code + ' created.';
			await Promise.all([loadContentRights(), refreshAudit()]);
		} catch (err) {
			rightsError =
				err instanceof ApiError ? err.message : 'Content rights could not be created.';
		} finally {
			rightsBusy = false;
		}
	}

	async function revokeContentRight(rightsId: string) {
		const reason = revocationReasons[rightsId]?.trim() ?? '';
		if (!reason || rightsBusy || revokingRightsId) return;
		revokingRightsId = rightsId;
		rightsError = '';
		rightsMessage = '';
		try {
			await Api.revokeContentRights(rightsId, reason);
			revocationReasons = { ...revocationReasons, [rightsId]: '' };
			rightsMessage = 'Rights record revoked. New reads and imports are blocked.';
			await Promise.all([loadContentRights(), refreshAudit()]);
		} catch (err) {
			rightsError = err instanceof ApiError ? err.message : 'Content rights could not be revoked.';
		} finally {
			revokingRightsId = '';
		}
	}

	onMount(loadContentRights);
</script>

<section class="card" aria-labelledby="content-rights-heading" data-testid="content-rights">
	<h2 id="content-rights-heading">Content rights</h2>
	<p class="muted">
		Record the permitted uses from the source agreement. This ledger records operator-supplied
		information; it does not verify or replace the underlying license.
	</p>
	<form onsubmit={createContentRight}>
		<label class="field" for="rights-ref-code">
			<span>Reference code</span>
			<input id="rights-ref-code" bind:value={rightsRefCode} maxlength="60" required />
		</label>
		<label class="field" for="rights-licensor">
			<span>Licensor</span>
			<input id="rights-licensor" bind:value={rightsLicensor} required />
		</label>
		<label class="field" for="rights-territory">
			<span>Territory</span>
			<input id="rights-territory" bind:value={rightsTerritory} placeholder="worldwide" />
		</label>
		<fieldset
			class="permitted-uses"
			disabled={rightsBusy}
		>
			<legend>Permitted uses</legend>
			{#each contentRightUses as use (use.value)}
				<label class="cluster">
					<input
						type="checkbox"
						checked={rightsPermittedUses.includes(use.value)}
						onchange={(event) =>
							toggleContentRightUse(use.value, event.currentTarget.checked)}
					/>
					<span>{use.label}</span>
				</label>
			{/each}
		</fieldset>
		<label class="field" for="rights-valid-from">
			<span>Valid from</span>
			<input id="rights-valid-from" type="date" bind:value={rightsValidFrom} required />
		</label>
		<label class="field" for="rights-valid-to">
			<span>Valid through (optional)</span>
			<input
				id="rights-valid-to"
				type="date"
				bind:value={rightsValidTo}
				min={rightsValidFrom || undefined}
			/>
		</label>
		<label class="field" for="rights-notes">
			<span>Notes (optional)</span>
			<textarea id="rights-notes" bind:value={rightsNotes} rows="2"></textarea>
		</label>
		<details>
			<summary>Contract and scope terms</summary>
			<label class="field" for="rights-contract-ref">
				<span>Contract reference</span>
				<input id="rights-contract-ref" bind:value={rightsContractRef} maxlength="2000" />
			</label>
			<label class="field" for="rights-contract-version">
				<span>Contract version</span>
				<input id="rights-contract-version" bind:value={rightsContractVersion} maxlength="2000" />
			</label>
			<label class="field" for="rights-assets">
				<span>Covered asset references (one per line)</span>
				<textarea id="rights-assets" bind:value={rightsAssetRefs} rows="3"></textarea>
			</label>
			<label class="field" for="rights-audiences">
				<span>Permitted audiences (one per line)</span>
				<textarea id="rights-audiences" bind:value={rightsAudiences} rows="2"></textarea>
			</label>
			<label class="field" for="rights-seat-limit">
				<span>Seat limit (optional)</span>
				<input id="rights-seat-limit" type="number" min="1" step="1" bind:value={rightsSeatLimit} />
			</label>
			<label class="field" for="rights-offline-terms">
				<span>Offline-use terms</span>
				<textarea id="rights-offline-terms" bind:value={rightsOfflineTerms} maxlength="2000" rows="2"></textarea>
			</label>
			<label class="field" for="rights-quotation-limit">
				<span>Quotation limit in words</span>
				<input id="rights-quotation-limit" type="number" min="0" max="1000000" step="1" bind:value={rightsQuotationLimit} />
			</label>
			<label class="field" for="rights-ai-terms">
				<span>AI processing permissions</span>
				<textarea id="rights-ai-terms" bind:value={rightsAiTerms} maxlength="2000" rows="2"></textarea>
			</label>
			<label class="field" for="rights-derivative-terms">
				<span>Derivative terms</span>
				<textarea id="rights-derivative-terms" bind:value={rightsDerivativeTerms} maxlength="2000" rows="2"></textarea>
			</label>
			<label class="field" for="rights-attribution">
				<span>Required attribution</span>
				<textarea id="rights-attribution" bind:value={rightsAttribution} maxlength="2000" rows="2"></textarea>
			</label>
			<label class="field" for="rights-royalty-terms">
				<span>Royalty terms</span>
				<textarea id="rights-royalty-terms" bind:value={rightsRoyaltyTerms} maxlength="2000" rows="2"></textarea>
			</label>
		</details>
		<button
			class="btn primary"
			type="submit"
			disabled={rightsBusy || rightsPermittedUses.length === 0 || !rightsValidFrom}
			data-testid="content-rights-create"
		>
			{rightsBusy ? 'Saving…' : 'Record rights'}
		</button>
	</form>
	{#if rightsMessage}
		<p class="muted" role="status" data-testid="content-rights-message">{rightsMessage}</p>
	{/if}
	{#if rightsError}
		<p class="error-text" role="alert">{rightsError}</p>
		<button class="btn" type="button" disabled={rightsBusy} onclick={loadContentRights}>
			Retry loading rights
		</button>
	{/if}

	<h3>Recorded grants</h3>
	{#if rightsBusy && contentRights.length === 0}
		<p class="muted is-loading" role="status">Loading rights records…</p>
	{:else if contentRights.length === 0}
		<p class="muted">No content rights records yet.</p>
	{:else}
		<ul class="bare-list">
			{#each contentRights as right (right.rights_id)}
				<li
					class="rights-record"
					data-testid={'content-right-' + right.rights_id}
				>
					<strong>{right.ref_code}</strong>
					<span class="chip">{right.status.charAt(0).toUpperCase() + right.status.slice(1)}</span>
					<p class="muted">{right.licensor} · {right.territory}</p>
						<p class="muted">Permitted: {right.permitted_uses.join(', ')}</p>
					<p class="muted">
						{right.valid_from}{right.valid_to ? ' – ' + right.valid_to : ' – no end date'}
					</p>
						{#if right.notes}
							<p class="muted">Notes: {right.notes}</p>
						{/if}
						{#if right.contract_ref || right.contract_version}
							<p class="muted">
								Contract: {right.contract_ref || 'reference not recorded'}
								{right.contract_version ? ' · ' + right.contract_version : ''}
							</p>
						{/if}
						{#if right.asset_refs.length > 0}
							<p class="muted">Assets: {right.asset_refs.join(', ')}</p>
						{/if}
						{#if right.audiences.length > 0}
							<p class="muted">Audiences: {right.audiences.join(', ')}</p>
						{/if}
						{#if right.seat_limit || right.quotation_limit_words !== null}
							<p class="muted">
								{right.seat_limit ? 'Seats: ' + right.seat_limit : ''}
								{right.quotation_limit_words !== null
									? ' · Quotation words: ' + right.quotation_limit_words
									: ''}
							</p>
						{/if}
						{#if right.offline_terms}<p class="muted">Offline: {right.offline_terms}</p>{/if}
						{#if right.ai_terms}<p class="muted">AI permissions: {right.ai_terms}</p>{/if}
						{#if right.derivative_terms}<p class="muted">Derivatives: {right.derivative_terms}</p>{/if}
						{#if right.attribution}<p class="muted">Attribution: {right.attribution}</p>{/if}
						{#if right.royalty_terms}<p class="muted">Royalties: {right.royalty_terms}</p>{/if}
					{#if right.revocation_note}
						<p class="muted">Revocation reason: {right.revocation_note}</p>
					{/if}
					{#if !right.revoked_at}
						<label class="field" for={'rights-revoke-' + right.rights_id}>
							<span>Reason to revoke {right.ref_code}</span>
							<input
								id={'rights-revoke-' + right.rights_id}
								bind:value={revocationReasons[right.rights_id]}
								maxlength="500"
								required
							/>
						</label>
						<button
							class="btn danger-text"
							type="button"
							disabled={rightsBusy || !!revokingRightsId || !revocationReasons[right.rights_id]?.trim()}
							onclick={() => revokeContentRight(right.rights_id)}
						>
							{revokingRightsId === right.rights_id ? 'Revoking…' : 'Revoke rights'}
						</button>
					{/if}
				</li>
			{/each}
		</ul>
	{/if}
	<button class="btn" type="button" disabled={rightsBusy} onclick={loadContentRights}>
		Refresh rights
	</button>
</section>

<style>
	.rights-record {
		min-width: 0;
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}

	.permitted-uses {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-md);
		margin: var(--space-md) 0;
		padding: var(--space-md);
		border: 1px solid var(--color-surface-elevated);
		border-radius: var(--radius-control);
	}
</style>
