import { mockDeviceRegistration } from './mock-device-registration';

import { createHash, generateKeyPairSync, sign } from 'node:crypto';
import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
	await mockDeviceRegistration(page, ['e2e-user-token']);
});

const examId = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const chapterId = 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb';
const signing = generateKeyPairSync('ed25519');
const publicKey = signing.publicKey.export({ format: 'der', type: 'spki' }).subarray(-32);
const publicKeyHex = publicKey.toString('hex');
const keyId = createHash('sha256').update(publicKey).digest('hex').slice(0, 16);
const deviceId = 'e2e-device-01';
const questionIds = Array.from(
  { length: 26 },
  (_, i) => `${(i + 1).toString(16).padStart(8, '0')}-aaaa-4aaa-8aaa-aaaaaaaaaaaa`
);

function canonicalValue(value: unknown): string {
  if (value === null) return 'z;';
  if (value === false) return 'f;';
  if (value === true) return 't;';
  if (typeof value === 'number') return `n${value};`;
  if (typeof value === 'string') {
    const bytes = Buffer.from(value, 'utf8');
    return `s${bytes.length}:${bytes.toString('hex')};`;
  }
  if (Array.isArray(value)) return `a${value.length}:${value.map(canonicalValue).join('')}`;
  if (value && typeof value === 'object') {
    const entries = Object.entries(value).sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
    return `o${entries.length}:${entries.map(([key, item]) => canonicalValue(key) + canonicalValue(item)).join('')}`;
  }
  throw new Error('Unexpected offline resource field.');
}

function receiptFor(ids: string[], request_nonce: string) {
  const issued_at = new Date().toISOString();
  const checksums = ids.map((id) => resource(id, questionIds.indexOf(id)).checksum);
  const payload = { checksums, device_id: deviceId, exam_id: examId, issued_at, request_nonce };
  return {
    ...payload,
    signature: sign(null, Buffer.from(canonicalValue(payload)), signing.privateKey).toString('hex')
  };
}

function resource(id: string, index: number) {
  const content = {
    question_version_id: id,
    vignette: `Fictional practice case ${index + 1}.`,
    lead_in: 'Which option matches the case?',
    difficulty: 'easy',
    options: [
      { text: 'Option A', rationale: 'This is the keyed option.' },
      { text: 'Option B', rationale: 'This is a distractor.' }
    ],
    correct_index: 0,
    key_learning_point: 'Use the case evidence.',
    exam_tip: null,
    source_ref: 'Synthetic source',
    tutoring_cards: []
  };
  const canonical = canonicalValue(content);
  const checksum = createHash('sha256').update(canonical).digest('hex');
  return { ...content, checksum };
}

function signedManifest() {
  const items = questionIds.map((id, index) => {
    const item = resource(id, index);
    return {
      question_version_id: id,
      checksum: item.checksum
    };
  });
  const canonical = [
    'medical-os-pack-manifest-v4',
    `exam ${examId}`,
    `device ${JSON.stringify(deviceId)}`,
    `chapters ${chapterId}`,
    ...items.map((item) => `${item.question_version_id} ${item.checksum}`),
    ''
  ].join('\n');
  return {
    exam_id: examId,
    device_id: deviceId,
    chapters: [chapterId],
    items,
    manifest_version: 4,
    canonical_format: 'medical-os-pack-manifest-v4',
    item_checksum_algorithm: 'sha256(medical-os-resource-canonical-v1)',
    algorithm: 'ed25519',
    key_id: keyId,
    verification_key: publicKeyHex,
    signature: sign(null, Buffer.from(canonical), signing.privateKey).toString('hex')
  };
}

test('offline packs verify, resume, enforce lease expiry, reopen offline, and remove revoked content', async ({ page }) => {
  const manifest = signedManifest();
  const resourceRequests: string[][] = [];
  const requestNonces: string[] = [];
  let firstBatchResponse: Record<string, unknown> | undefined;
  let replayCachedBatchOnce = false;
  let sessionRequest: { preset: string; chapter_ids: string[]; source: string; question_count: number } | null = null;
  let failSecondBatchOnce = true;
  let leaseRevoked = false;

  await page.addInitScript(() => {
    localStorage.setItem('mlos_token', 'e2e-user-token');
    localStorage.setItem('mlos_pack_device', 'e2e-device-01');
  });
  await page.route('**/v1/me/curriculum', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        chapters: [
          {
            chapter_id: chapterId,
            chapter_name: 'Fictional practice chapter',
            system: 'Synthetic system',
            subject: 'Synthetic subject',
            exam_id: examId,
            exam: 'Test exam',
            published_questions: questionIds.length
          }
        ]
      })
    })
  );
  await page.route('**/v1/packs/lease', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        lease_id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
        expires_at: new Date(Date.now() + 14 * 86_400_000).toISOString(),
        content_as_of: new Date().toISOString(),
        pack_key: '11'.repeat(32),
        algorithm: 'AES-GCM-256'
      })
    })
  );
  await page.route('**/v1/me/packs', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        leases: leaseRevoked
          ? []
          : [
              {
                lease_id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
                exam_id: examId,
                device_id: deviceId,
                chapters: [chapterId],
                expires_at: new Date(Date.now() + 14 * 86_400_000).toISOString(),
                content_as_of: new Date().toISOString()
              }
            ]
      })
    })
  );
  await page.route('**/v1/packs/lease/*', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ revoked: true }) })
  );
  await page.route('**/v1/practice/sessions', async (route) => {
    sessionRequest = route.request().postDataJSON();
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ session_id: 'eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee' })
    });
  });
  await page.route('**/v2/packs/*/manifest*', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(manifest) })
  );
  await page.route('**/v2/packs/*/resources', async (route) => {
    const body = route.request().postDataJSON() as {
      question_version_ids: string[];
      request_nonce: string;
    };
    const ids = body.question_version_ids;
    resourceRequests.push(ids);
    requestNonces.push(body.request_nonce);
    if (resourceRequests.length === 2 && failSecondBatchOnce) {
      failSecondBatchOnce = false;
      await route.fulfill({
        status: 503,
        contentType: 'application/json',
        body: JSON.stringify({ error: { code: 'temporarily_unavailable', message: 'Try again.' } })
      });
      return;
    }
    if (replayCachedBatchOnce && ids.length === 25 && firstBatchResponse) {
      replayCachedBatchOnce = false;
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify(firstBatchResponse)
      });
      return;
    }
    const response = {
      resources: ids.map((id) => resource(id, questionIds.indexOf(id))),
      receipt: receiptFor(ids, body.request_nonce)
    };
    if (ids.length === 25 && !firstBatchResponse) firstBatchResponse = response;
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(response)
    });
  });

  await page.goto('/offline');
  await page.evaluate(async () => {
    if ('serviceWorker' in navigator) await navigator.serviceWorker.ready;
  });
  await page.reload();
  await page.getByTestId(`pack-chapter-${chapterId}`).check();
  await page.getByTestId('pack-download').click();
  await expect(page.getByTestId('pack-error')).toContainText('Try again');
  await expect(page.getByTestId('pack-progress')).toContainText('25');

  await page.getByTestId('pack-download').click();
  await expect(page.getByTestId('pack-ready')).toBeVisible();
  expect(resourceRequests.map((batch) => batch.length)).toEqual([25, 1, 1]);
  expect(resourceRequests[2]).toEqual([questionIds[25]]);
  expect(requestNonces).toHaveLength(3);
  expect(requestNonces.every((nonce) => /^[0-9a-f]{64}$/.test(nonce))).toBe(true);
  expect(new Set(requestNonces).size).toBe(requestNonces.length);

  const openPageErrors: string[] = [];
  page.on('pageerror', (cause) => openPageErrors.push(cause.message));
  await page.getByTestId('pack-open').click();
  // Decrypting and opening the pack is real crypto work and can exceed the
  // 5s default under CI parallel load.
  await expect
    .poll(
      async () => {
      if (await page.getByTestId('pack-question').isVisible()) return 'opened';
      const error = page.getByTestId('pack-error');
      const reader = page.locator('.reader');
      const errorText = (await error.count()) ? await error.textContent() : 'none';
      const readerText = (await reader.count()) ? await reader.innerText() : 'absent';
      return `opening; error=${errorText}; reader=${readerText.replace(/\s+/g, ' ').slice(0, 240)}; pageerror=${openPageErrors.at(-1) ?? 'none'}`;
    })
    .toBe('opened', { timeout: 20_000 });
  await expect(page.getByTestId('pack-question')).toContainText('Fictional practice case 1');
  await expect(page.getByTestId('pack-answer-reveal')).not.toHaveAttribute('open', '');
  await page.getByTestId('pack-answer-reveal').locator('summary').click();
  await expect(page.getByTestId('pack-answer-reveal')).toContainText('Option A');
  await expect(page.getByTestId('pack-start-session')).toBeVisible();

  for (const width of [320, 375, 414, 768, 1280]) {
    await page.setViewportSize({ width, height: 900 });
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      `page should fit a ${width}px viewport`
    ).toBe(true);
  }

  await page.getByTestId('pack-start-session').click();
  await expect(page).toHaveURL(/\/session\/eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee$/);
  expect(sessionRequest).toEqual({
    preset: 'tutor',
    chapter_ids: [chapterId],
    source: 'any',
    question_count: 10
  });
  await page.goto('/offline');

  await page.context().setOffline(true);
  const offlineApiRequests: string[] = [];
  page.on('request', (request) => {
    if (/\/v[12]\//.test(new URL(request.url()).pathname)) offlineApiRequests.push(request.url());
  });
  await page.reload();
  expect(offlineApiRequests).toEqual([]);
  await expect(page.getByTestId('pack-ready')).toBeVisible();
  await expect(page.getByTestId('offline-best-effort')).toBeVisible();
  await page.getByTestId('pack-open').click();
  await expect(page.getByTestId('pack-question')).toContainText('Fictional practice case 1');
  await page.getByTestId('pack-answer-reveal').locator('summary').click();
  await expect(page.getByTestId('pack-answer-reveal')).toContainText('Option A');
  await expect(page.getByTestId('pack-start-session')).toHaveCount(0);

  await page.context().setOffline(false);
  await page.reload();
  await page.evaluate(async ({ examId, deviceId }) => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = indexedDB.open('medical-os-offline-packs', 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const transaction = db.transaction('packs', 'readwrite');
    const store = transaction.objectStore('packs');
    const record = await new Promise<Record<string, unknown> | undefined>((resolve, reject) => {
      const request = store.get(`${examId}:${deviceId}`);
      request.onsuccess = () => resolve(request.result as Record<string, unknown> | undefined);
      request.onerror = () => reject(request.error);
    });
    if (!record) throw new Error('Downloaded offline pack record is missing.');
    record.expires_at = new Date(Date.now() - 60_000).toISOString();
    store.put(record);
    await new Promise<void>((resolve, reject) => {
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error);
    });
    db.close();
  }, { examId, deviceId });
  await page.reload();
  await expect(page.getByTestId('pack-open')).toBeDisabled();
  await expect(page.getByTestId('pack-open')).toHaveText('Lease expired');
  await page.getByTestId('pack-remove').click();
  await expect(page.getByTestId('pack-empty')).toBeVisible();

  replayCachedBatchOnce = true;
  await page.getByTestId('pack-download').click();
  await expect(page.getByTestId('pack-error')).toContainText('request challenge');
  await expect(page.getByTestId('pack-ready')).toHaveCount(0);
  await expect(page.getByTestId('pack-partial')).toContainText('0 of 26 questions');

  await page.getByTestId('pack-download').click();
  await expect(page.getByTestId('pack-ready')).toBeVisible();
  leaseRevoked = true;
  await page.reload();
  await page.getByTestId('pack-open').click();
  await expect(page.getByTestId('pack-error')).toContainText('removed');
  await expect(page.getByTestId('pack-empty')).toBeVisible();
});

test('browser quota exhaustion keeps the pack incomplete and reports the limit', async ({ page }) => {
  const manifest = signedManifest();
  await page.addInitScript(() => {
    localStorage.setItem('mlos_token', 'e2e-user-token');
    localStorage.setItem('mlos_pack_device', 'e2e-device-01');
    Object.defineProperty(navigator, 'storage', {
      configurable: true,
      value: {
        estimate: async () => ({ quota: 1024, usage: 1020 }),
        persist: async () => false
      }
    });
  });
  await page.route('**/v1/me/curriculum', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        chapters: [
          {
            chapter_id: chapterId,
            chapter_name: 'Fictional practice chapter',
            system: 'Synthetic system',
            subject: 'Synthetic subject',
            exam_id: examId,
            exam: 'Test exam',
            published_questions: questionIds.length
          }
        ]
      })
    })
  );
  await page.route('**/v1/packs/lease', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        lease_id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
        expires_at: new Date(Date.now() + 14 * 86_400_000).toISOString(),
        content_as_of: new Date().toISOString(),
        pack_key: '11'.repeat(32),
        algorithm: 'AES-GCM-256'
      })
    })
  );
  await page.route('**/v2/packs/*/manifest*', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(manifest) })
  );
  await page.route('**/v2/packs/*/resources', (route) => {
    const body = route.request().postDataJSON() as {
      question_version_ids: string[];
      request_nonce: string;
    };
    const ids = body.question_version_ids;
    return route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        resources: ids.map((id) => resource(id, questionIds.indexOf(id))),
        receipt: receiptFor(ids, body.request_nonce)
      })
    });
  });
  await page.goto('/offline');
  await expect(page.getByTestId('offline-best-effort')).toBeVisible();
  await page.getByTestId(`pack-chapter-${chapterId}`).check();
  await page.getByTestId('pack-download').click();
  await expect(page.getByTestId('pack-error')).toContainText(/storage/i);
  await expect(page.getByTestId('pack-ready')).toHaveCount(0);
});

test('download rejects an invalid signed receipt before saving its batch', async ({ page }) => {
  const manifest = signedManifest();
  let resourceRequests = 0;
  await page.addInitScript(() => {
    localStorage.setItem('mlos_token', 'e2e-user-token');
    localStorage.setItem('mlos_pack_device', 'e2e-device-01');
  });
  await page.route('**/v1/me/curriculum', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        chapters: [
          {
            chapter_id: chapterId,
            chapter_name: 'Fictional practice chapter',
            system: 'Synthetic system',
            subject: 'Synthetic subject',
            exam_id: examId,
            exam: 'Test exam',
            published_questions: questionIds.length
          }
        ]
      })
    })
  );
  await page.route('**/v1/me/packs', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ leases: [] })
    })
  );
  await page.route('**/v1/packs/lease', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        lease_id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
        expires_at: new Date(Date.now() + 14 * 86_400_000).toISOString(),
        content_as_of: new Date().toISOString(),
        pack_key: '11'.repeat(32),
        algorithm: 'AES-GCM-256'
      })
    })
  );
  await page.route('**/v2/packs/*/manifest*', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(manifest) })
  );
  await page.route('**/v2/packs/*/resources', async (route) => {
    const body = route.request().postDataJSON() as {
      question_version_ids: string[];
      request_nonce: string;
    };
    const ids = body.question_version_ids;
    const receipt = receiptFor(ids, body.request_nonce);
    receipt.signature = '00'.repeat(64);
    resourceRequests += 1;
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ resources: ids.map((id) => resource(id, questionIds.indexOf(id))), receipt })
    });
  });

  await page.goto('/offline');
  await page.evaluate(async () => {
    if ('serviceWorker' in navigator) await navigator.serviceWorker.ready;
  });
  await page.reload();
  await page.getByTestId(`pack-chapter-${chapterId}`).check();
  await page.getByTestId('pack-download').click();

  await expect(page.getByTestId('pack-error')).toContainText('signature');
  await expect(page.getByTestId('pack-partial')).toContainText('0 of 26 questions');
  await expect(page.getByTestId('pack-open')).toBeDisabled();
  expect(resourceRequests).toBe(1);
});

test('invalid manifest signatures are rejected before any question resource is downloaded', async ({ page }) => {
  const manifest = signedManifest();
  manifest.signature = '00'.repeat(64);
  let resourceRequests = 0;

  await page.addInitScript(() => {
    localStorage.setItem('mlos_token', 'e2e-user-token');
    localStorage.setItem('mlos_pack_device', 'e2e-device-01');
  });
  await page.route('**/v1/me/curriculum', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        chapters: [
          {
            chapter_id: chapterId,
            chapter_name: 'Fictional practice chapter',
            system: 'Synthetic system',
            subject: 'Synthetic subject',
            exam_id: examId,
            exam: 'Test exam',
            published_questions: questionIds.length
          }
        ]
      })
    })
  );
  await page.route('**/v1/packs/lease', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        lease_id: 'dddddddd-dddd-4ddd-8ddd-dddddddddddd',
        expires_at: new Date(Date.now() + 14 * 86_400_000).toISOString(),
        content_as_of: new Date().toISOString(),
        pack_key: '11'.repeat(32),
        algorithm: 'AES-GCM-256'
      })
    })
  );
  await page.route('**/v2/packs/*/manifest*', (route) =>
    route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(manifest) })
  );
  await page.route('**/v2/packs/*/resources', async (route) => {
    resourceRequests += 1;
    await route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ resources: [] }) });
  });

  await page.goto('/offline');
  await page.getByTestId(`pack-chapter-${chapterId}`).check();
  await page.getByTestId('pack-download').click();
  await expect(page.getByTestId('pack-error')).toContainText('manifest signature is invalid');
  await expect(page.getByTestId('pack-empty')).toBeVisible();
  expect(resourceRequests).toBe(0);
});
