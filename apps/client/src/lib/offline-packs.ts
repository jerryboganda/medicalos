import { Api, type PackManifest, type PackQuestionResource } from '$lib/api';
import { browserDeviceId as deviceId } from './device-identity';

export { deviceId };

const DB_NAME = 'medical-os-offline-packs';
const DB_VERSION = 1;
export const MAX_PACK_QUESTIONS = 500;
const RESOURCE_BATCH_SIZE = 25;

interface PackRecord {
	pack_id: string;
	exam_id: string;
	device_id: string;
	chapters: string[];
	lease_id: string;
	expires_at: string;
	content_as_of: string;
	manifest: PackManifest;
	key: CryptoKey;
	item_count: number;
	downloaded_count: number;
	byte_count: number;
	saved_at: string;
	receipts?: PackDownloadReceipt[];
}

interface PackDownloadReceipt {
	device_id: string;
	exam_id: string;
	issued_at: string;
	checksums: string[];
	signature: string;
}

interface EncryptedResource {
	record_id: string;
	pack_id: string;
	question_version_id: string;
	checksum: string;
	iv: ArrayBuffer;
	ciphertext: ArrayBuffer;
	byte_count: number;
}

export interface OfflinePackSummary {
	pack_id: string;
	exam_id: string;
	device_id: string;
	chapters: string[];
	lease_id: string;
	expires_at: string;
	content_as_of: string;
	item_count: number;
	downloaded_count: number;
	byte_count: number;
	saved_at: string;
}

export interface OfflinePackContent extends OfflinePackSummary {
	resources: PackQuestionResource[];
}

export interface PackDownloadProgress {
	done: number;
	total: number;
}

let database: Promise<IDBDatabase> | undefined;

function openDatabase(): Promise<IDBDatabase> {
	if (database) return database;
	database = new Promise((resolve, reject) => {
		if (typeof indexedDB === 'undefined') {
			reject(new Error('This browser does not support offline pack storage.'));
			return;
		}
		const request = indexedDB.open(DB_NAME, DB_VERSION);
		request.onupgradeneeded = () => {
			const db = request.result;
			if (!db.objectStoreNames.contains('packs')) {
				db.createObjectStore('packs', { keyPath: 'pack_id' });
			}
			if (!db.objectStoreNames.contains('resources')) {
				const resources = db.createObjectStore('resources', { keyPath: 'record_id' });
				resources.createIndex('pack_id', 'pack_id', { unique: false });
			}
		};
		request.onsuccess = () => {
			request.result.onversionchange = () => request.result.close();
			resolve(request.result);
		};
		request.onerror = () => reject(request.error ?? new Error('Could not open offline storage.'));
		request.onblocked = () => reject(new Error('Close another Medical OS tab to update offline storage.'));
	});
	return database;
}

function requestValue<T>(request: IDBRequest<T>): Promise<T> {
	return new Promise((resolve, reject) => {
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => reject(request.error ?? new Error('Offline storage request failed.'));
	});
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
	return new Promise((resolve, reject) => {
		transaction.oncomplete = () => resolve();
		transaction.onerror = () => reject(transaction.error ?? new Error('Offline storage write failed.'));
		transaction.onabort = () => reject(transaction.error ?? new Error('Offline storage write was cancelled.'));
	});
}

export function packId(examId: string, id = deviceId()): string {
	return `${examId}:${id}`;
}

function hexBytes(value: string): Uint8Array {
	if (!/^(?:[0-9a-f]{2})+$/i.test(value)) throw new Error('The pack contains an invalid cryptographic value.');
	return Uint8Array.from(value.match(/.{2}/g) ?? [], (byte) => Number.parseInt(byte, 16));
}

function bytesHex(value: ArrayBuffer): string {
	return [...new Uint8Array(value)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function arrayBuffer(bytes: Uint8Array): ArrayBuffer {
	const output = new ArrayBuffer(bytes.byteLength);
	new Uint8Array(output).set(bytes);
	return output;
}

function canonicalValue(value: unknown): string {
	if (value === null) return 'z;';
	if (value === false) return 'f;';
	if (value === true) return 't;';
	if (typeof value === 'number') return `n${value};`;
	if (typeof value === 'string') {
		const bytes = new TextEncoder().encode(value);
		return `s${bytes.length}:${[...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('')};`;
	}
	if (Array.isArray(value)) return `a${value.length}:${value.map(canonicalValue).join('')}`;
	if (value && typeof value === 'object') {
		const entries = Object.entries(value).sort(([left], [right]) =>
			left < right ? -1 : left > right ? 1 : 0
		);
		return `o${entries.length}:${entries.map(([key, item]) => canonicalValue(key) + canonicalValue(item)).join('')}`;
	}
	throw new Error('The pack contains an unsupported resource value.');
}

function manifestCanonical(manifest: PackManifest): string {
	return [
		manifest.canonical_format,
		`exam ${manifest.exam_id}`,
		`device ${JSON.stringify(manifest.device_id)}`,
		`chapters ${manifest.chapters.join(',')}`,
		...manifest.items.map((item) => `${item.question_version_id} ${item.checksum}`),
		''
	].join('\n');
}

async function verifyEd25519Signature(
	publicKey: Uint8Array,
	signature: Uint8Array,
	message: string
): Promise<boolean> {
	try {
		const key = await crypto.subtle.importKey(
			'raw',
			arrayBuffer(publicKey),
			{ name: 'Ed25519' } as AlgorithmIdentifier,
			false,
			['verify']
		);
		return await crypto.subtle.verify(
			{ name: 'Ed25519' } as AlgorithmIdentifier,
			key,
			arrayBuffer(signature),
			arrayBuffer(new TextEncoder().encode(message))
		);
	} catch {
		throw new Error('This browser cannot verify Ed25519 offline pack signatures. Update the browser and try again.');
	}
}

async function verifyManifest(manifest: PackManifest, expectedExam?: string, expectedDevice?: string) {
	if (
		manifest.manifest_version !== 4 ||
		manifest.algorithm !== 'ed25519' ||
		manifest.canonical_format !== 'medical-os-pack-manifest-v4' ||
		manifest.item_checksum_algorithm !== 'sha256(medical-os-resource-canonical-v1)' ||
		(expectedExam && manifest.exam_id !== expectedExam) ||
		(expectedDevice && manifest.device_id !== expectedDevice) ||
		!Array.isArray(manifest.items) ||
		manifest.items.length > MAX_PACK_QUESTIONS
	) {
		throw new Error('This pack manifest is unsupported or exceeds the browser pack limit.');
	}
	const ids = new Set<string>();
	for (const item of manifest.items) {
		if (ids.has(item.question_version_id) || !/^[0-9a-f]{64}$/i.test(item.checksum)) {
			throw new Error('The pack manifest contains a duplicate or invalid item.');
		}
		ids.add(item.question_version_id);
	}
	const publicKey = hexBytes(manifest.verification_key);
	const signature = hexBytes(manifest.signature);
	if (publicKey.byteLength !== 32 || signature.byteLength !== 64) {
		throw new Error('The pack signature has an invalid length.');
	}
	const keyId = bytesHex(await crypto.subtle.digest('SHA-256', arrayBuffer(publicKey))).slice(0, 16);
	if (keyId !== manifest.key_id) throw new Error('The pack signing key ID does not match.');
	if (!(await verifyEd25519Signature(publicKey, signature, manifestCanonical(manifest)))) {
		throw new Error('The pack manifest signature is invalid.');
	}
}

async function verifyDownloadReceipt(
	receipt: PackDownloadReceipt | undefined,
	manifest: PackManifest,
	examId: string,
	deviceId: string,
	checksums: string[]
) {
	if (
		!receipt ||
		receipt.device_id !== deviceId ||
		receipt.exam_id !== examId ||
		!Array.isArray(receipt.checksums) ||
		JSON.stringify(receipt.checksums) !== JSON.stringify(checksums)
	) {
		throw new Error('The signed download receipt did not cover this verified batch. Resume to retry it.');
	}
	if (
		typeof receipt.signature !== 'string' ||
		!/^[0-9a-f]{128}$/i.test(receipt.signature)
	) {
		throw new Error('The signed download receipt signature has an invalid format. Resume to retry it.');
	}
	const payload = {
		checksums: receipt.checksums,
		device_id: receipt.device_id,
		exam_id: receipt.exam_id,
		issued_at: receipt.issued_at
	};
	const valid = await verifyEd25519Signature(
		hexBytes(manifest.verification_key),
		hexBytes(receipt.signature),
		canonicalValue(payload)
	);
	if (!valid) throw new Error('The signed download receipt signature is invalid. Resume to retry it.');
}

async function resourceChecksum(resource: PackQuestionResource): Promise<string> {
	const { checksum: _checksum, ...content } = resource;
	return bytesHex(
		await crypto.subtle.digest('SHA-256', arrayBuffer(new TextEncoder().encode(canonicalValue(content))))
	);
}

async function importPackKey(hexKey: string): Promise<CryptoKey> {
	const bytes = hexBytes(hexKey);
	if (bytes.byteLength !== 32) throw new Error('The server returned an invalid per-device pack key.');
	return crypto.subtle.importKey('raw', arrayBuffer(bytes), 'AES-GCM', false, ['encrypt', 'decrypt']);
}

async function encrypt(
	key: CryptoKey,
	pack: string,
	identity: string,
	value: unknown
): Promise<{ iv: ArrayBuffer; ciphertext: ArrayBuffer; byte_count: number }> {
	const iv = crypto.getRandomValues(new Uint8Array(12));
	const additionalData = arrayBuffer(new TextEncoder().encode(`${pack}\0${identity}`));
	const plaintext = new TextEncoder().encode(JSON.stringify(value));
	const ciphertext = await crypto.subtle.encrypt(
		{ name: 'AES-GCM', iv: arrayBuffer(iv), additionalData },
		key,
		plaintext
	);
	return { iv: arrayBuffer(iv), ciphertext, byte_count: iv.byteLength + ciphertext.byteLength };
}

async function decrypt<T>(
	key: CryptoKey,
	pack: string,
	identity: string,
	iv: ArrayBuffer,
	ciphertext: ArrayBuffer
): Promise<T> {
	const plaintext = await crypto.subtle.decrypt(
		{
			name: 'AES-GCM',
			iv,
			additionalData: arrayBuffer(new TextEncoder().encode(`${pack}\0${identity}`))
		},
		key,
		ciphertext
	);
	return JSON.parse(new TextDecoder().decode(plaintext)) as T;
}

function publicSummary(pack: PackRecord): OfflinePackSummary {
	const { key: _key, manifest: _manifest, ...summary } = pack;
	return summary;
}

export async function listOfflinePacks(): Promise<OfflinePackSummary[]> {
	const db = await openDatabase();
	const tx = db.transaction('packs', 'readonly');
	const done = transactionDone(tx);
	const records = await requestValue(tx.objectStore('packs').getAll() as IDBRequest<PackRecord[]>);
	await done;
	return records.map(publicSummary).sort((left, right) => left.exam_id.localeCompare(right.exam_id));
}

export async function getOfflinePack(
	examId: string,
	id = deviceId(),
	allowExpired = false
): Promise<OfflinePackContent | null> {
	const db = await openDatabase();
	const packKey = packId(examId, id);
	const tx = db.transaction(['packs', 'resources'], 'readonly');
	const done = transactionDone(tx);
	const packRequest = tx.objectStore('packs').get(packKey) as IDBRequest<PackRecord | undefined>;
	const resourcesRequest = tx.objectStore('resources').index('pack_id').getAll(packKey) as IDBRequest<EncryptedResource[]>;
	const [pack, encryptedResources] = await Promise.all([requestValue(packRequest), requestValue(resourcesRequest)]);
	await done;
	if (!pack) return null;
	await verifyManifest(pack.manifest, pack.exam_id, pack.device_id);
	if (!allowExpired && Date.now() >= Date.parse(pack.expires_at)) {
		throw new Error('This offline pack lease has expired. Reconnect to renew the pack.');
	}
	const manifestItems = new Map(pack.manifest.items.map((item) => [item.question_version_id, item.checksum]));
	const resources: PackQuestionResource[] = [];
	for (const record of encryptedResources) {
		if (manifestItems.get(record.question_version_id) !== record.checksum) continue;
		const resource = await decrypt<PackQuestionResource>(
			pack.key,
			pack.pack_id,
			record.question_version_id + '\0' + record.checksum,
			record.iv,
			record.ciphertext
		);
		if (
			resource.question_version_id !== record.question_version_id ||
			resource.checksum !== record.checksum ||
			(await resourceChecksum(resource)) !== record.checksum
		) {
			throw new Error('Saved pack content failed its integrity check. Remove the pack and download it again.');
		}
		resources.push(resource);
	}
	const order = new Map(pack.manifest.items.map((item, index) => [item.question_version_id, index]));
	resources.sort((left, right) => (order.get(left.question_version_id) ?? 0) - (order.get(right.question_version_id) ?? 0));
	return { ...publicSummary(pack), resources };
}

async function savedRecords(packKey: string): Promise<EncryptedResource[]> {
	const db = await openDatabase();
	const tx = db.transaction('resources', 'readonly');
	const done = transactionDone(tx);
	const records = await requestValue(
		tx.objectStore('resources').index('pack_id').getAll(packKey) as IDBRequest<EncryptedResource[]>
	);
	await done;
	return records;
}

async function storeBatch(pack: PackRecord, resources: EncryptedResource[], removeIds: string[] = []) {
	const db = await openDatabase();
	const tx = db.transaction(['packs', 'resources'], 'readwrite');
	const done = transactionDone(tx);
	tx.objectStore('packs').put(pack);
	const resourceStore = tx.objectStore('resources');
	for (const id of removeIds) resourceStore.delete(id);
	for (const resource of resources) resourceStore.put(resource);
	await done;
}

async function assertStorageCapacity(resources: PackQuestionResource[]) {
	if (!navigator.storage?.estimate) return;
	let estimate: StorageEstimate;
	try {
		estimate = await navigator.storage.estimate();
	} catch {
		return;
	}
	if (estimate.quota === undefined || estimate.usage === undefined) return;
	const required = Math.ceil(
		resources.reduce((sum, resource) => sum + new TextEncoder().encode(JSON.stringify(resource)).byteLength + 28, 0) * 1.25
	);
	if (estimate.quota - estimate.usage < required) {
		throw new Error('There is not enough browser storage for this batch. Remove another pack or free device storage, then resume.');
	}
}

export async function requestPersistentStorage(): Promise<boolean | null> {
	if (!navigator.storage?.persist) return null;
	try {
		return await navigator.storage.persist();
	} catch {
		return false;
	}
}

export async function downloadPack(
	examId: string,
	selectedChapters: string[],
	onProgress: (progress: PackDownloadProgress) => void
): Promise<OfflinePackSummary> {
	const id = deviceId();
	const key = packId(examId, id);
	const db = await openDatabase();
	const readTx = db.transaction('packs', 'readonly');
	const readDone = transactionDone(readTx);
	const existing = await requestValue(readTx.objectStore('packs').get(key) as IDBRequest<PackRecord | undefined>);
	await readDone;
	const chapters = [...new Set([...(existing?.chapters ?? []), ...selectedChapters])].sort();
	if (!chapters.length) throw new Error('Choose at least one chapter.');

	const previousResources: PackQuestionResource[] = [];
	const previousRecords = existing ? await savedRecords(key) : [];
	if (existing) {
		const previous = await getOfflinePack(examId, id, true);
		previousResources.push(...(previous?.resources ?? []));
	}

	const lease = await Api.createPackLease({ exam_id: examId, device_id: id, chapters });
	const manifest = await Api.packManifest(examId, chapters, id);
	await verifyManifest(manifest, examId, id);
	if (manifest.chapters.length !== chapters.length || chapters.some((chapter) => !manifest.chapters.includes(chapter))) {
		throw new Error('The signed pack scope does not match the selected chapters.');
	}
	if (manifest.items.length > MAX_PACK_QUESTIONS) {
		throw new Error(`This browser pack has ${manifest.items.length} questions; the limit is ${MAX_PACK_QUESTIONS}. Choose fewer chapters.`);
	}
	const nextKey = await importPackKey(lease.pack_key);
	const savedAt = new Date().toISOString();
	const pack: PackRecord = {
		pack_id: key,
		exam_id: examId,
		device_id: id,
		chapters,
		lease_id: lease.lease_id,
		expires_at: lease.expires_at,
		content_as_of: lease.content_as_of,
		manifest,
		key: nextKey,
		item_count: manifest.items.length,
		downloaded_count: 0,
		byte_count: 0,
		saved_at: savedAt
	};
	const checksums = new Map(manifest.items.map((item) => [item.question_version_id, item.checksum]));
	const reusable = previousResources.filter((resource) => checksums.get(resource.question_version_id) === resource.checksum);
	const reencrypted: EncryptedResource[] = [];
	for (const resource of reusable) {
		const encrypted = await encrypt(nextKey, key, resource.question_version_id + '\0' + resource.checksum, resource);
		reencrypted.push({
			record_id: `${key}:${resource.question_version_id}`,
			pack_id: key,
			question_version_id: resource.question_version_id,
			checksum: resource.checksum,
			...encrypted
		});
		pack.byte_count += encrypted.byte_count;
	}
	pack.downloaded_count = reencrypted.length;
	await assertStorageCapacity(reusable);
	const reusableIds = new Set(reusable.map((resource) => resource.question_version_id));
	const removeIds = previousRecords
		.filter((record) => !reusableIds.has(record.question_version_id))
		.map((record) => record.record_id);
	await storeBatch(pack, reencrypted, removeIds);

	const complete = new Set(reusable.map((resource) => resource.question_version_id));
	const missing = manifest.items.filter((item) => !complete.has(item.question_version_id));
	onProgress({ done: complete.size, total: manifest.items.length });
	for (let offset = 0; offset < missing.length; offset += RESOURCE_BATCH_SIZE) {
		const batch = missing.slice(offset, offset + RESOURCE_BATCH_SIZE);
		const response = await Api.packResources(examId, {
			device_id: id,
			chapters,
			question_version_ids: batch.map((item) => item.question_version_id)
		});
		if (response.resources.length !== batch.length) throw new Error('The server returned an incomplete pack batch. Resume to retry it.');
		const expected = new Map(batch.map((item) => [item.question_version_id, item.checksum]));
		const found = new Set<string>();
		const batchResources: PackQuestionResource[] = [];
		for (const resource of response.resources) {
			if (
				found.has(resource.question_version_id) ||
				expected.get(resource.question_version_id) !== resource.checksum ||
				(await resourceChecksum(resource)) !== resource.checksum
			) {
				throw new Error('A downloaded question failed its signed checksum. No part of this batch was saved.');
			}
			found.add(resource.question_version_id);
			batchResources.push(resource);
		}
		if (found.size !== batch.length) throw new Error('The server returned an unexpected pack batch. Resume to retry it.');
		const receipt = response.receipt;
		const batchChecksums = batchResources.map((resource) => resource.checksum);
		await verifyDownloadReceipt(receipt, manifest, examId, id, batchChecksums);
		pack.receipts = [...(pack.receipts ?? []), receipt].slice(-50);
		await assertStorageCapacity(batchResources);
		const encryptedBatch: EncryptedResource[] = [];
		for (const resource of batchResources) {
			const encrypted = await encrypt(nextKey, key, resource.question_version_id + '\0' + resource.checksum, resource);
			encryptedBatch.push({
				record_id: `${key}:${resource.question_version_id}`,
				pack_id: key,
				question_version_id: resource.question_version_id,
				checksum: resource.checksum,
				...encrypted
			});
			pack.byte_count += encrypted.byte_count;
		}
		pack.downloaded_count = complete.size + batchResources.length;
		await storeBatch(pack, encryptedBatch);
		for (const resource of batchResources) complete.add(resource.question_version_id);
		onProgress({ done: complete.size, total: manifest.items.length });
	}
	return publicSummary(pack);
}

export async function removeOfflinePack(examId: string, id = deviceId()): Promise<string | null> {
	const key = packId(examId, id);
	const db = await openDatabase();
	const read = db.transaction('packs', 'readonly');
	const readDone = transactionDone(read);
	const pack = await requestValue(read.objectStore('packs').get(key) as IDBRequest<PackRecord | undefined>);
	await readDone;
	const tx = db.transaction(['packs', 'resources'], 'readwrite');
	const done = transactionDone(tx);
	const resourceStore = tx.objectStore('resources');
	const keyRequest = resourceStore.index('pack_id').getAllKeys(key);
	keyRequest.onsuccess = () => {
		for (const recordId of keyRequest.result) resourceStore.delete(recordId);
		tx.objectStore('packs').delete(key);
	};
	await done;
	return pack?.lease_id ?? null;
}
