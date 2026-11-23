import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import { createHash, randomUUID } from 'node:crypto';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

// Explicitly isolated synthetic-positive CI mode. The release Dioxus app,
// PostgreSQL queue/worker/Teacher RLS, real private Storage, and budget are
// exercised together. The local gateway is a STRICT CONTRACT FIXTURE rather
// than the production gateway; the real gateway->mock-provider transport is
// separately tested by native_pdf_two_hop_mock_gateway_preserves_original_and_rubric.
// Do not count this alone as complete native-PDF production release acceptance.
const origin = new URL(process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8080');

function api(page: Page) {
  const request = page.context().request;
  async function call(method: 'get' | 'post', path: string, options: any = {}) {
    const url = new URL(path, origin);
    const headers = { ...options.headers };
    if (url.origin === origin.origin && origin.protocol === 'http:'
        && ['localhost', '127.0.0.1'].includes(origin.hostname)) {
      headers.cookie = (await page.context().cookies())
        .filter(c => c.domain === origin.hostname && url.pathname.startsWith(c.path))
        .map(c => `${c.name}=${c.value}`).join('; ');
    }
    return request[method](path, { ...options, headers, maxRedirects: 0 });
  }
  return { get: (path: string, options?: any) => call('get', path, options),
           post: (path: string, options?: any) => call('post', path, options) };
}

async function login(page: Page, email: string) {
  const neutral = await page.goto('/healthz');
  expect(neutral?.ok()).toBeTruthy();
  const result = await api(page).post('/api/auth/login', {
    data: { email, password: 'e2e-password' },
  });
  expect(result.ok(), `login failed for ${email}: ${result.status()}`).toBeTruthy();
}

function onePagePdf(): Buffer {
  let body = '%PDF-1.4\n';
  const offsets = [0];
  for (const [i, object] of [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>',
  ].entries()) {
    offsets.push(Buffer.byteLength(body));
    body += `${i + 1} 0 obj\n${object}\nendobj\n`;
  }
  const xref = Buffer.byteLength(body);
  body += 'xref\n0 4\n0000000000 65535 f \n';
  body += offsets.slice(1).map(n => `${String(n).padStart(10, '0')} 00000 n \n`).join('');
  return Buffer.from(body + `trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});
test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('synthetic original PDF flows through real queue/worker and Teacher decision @native-pdf-positive @security', async ({ page }, info) => {
  expect(process.env.E2E_NATIVE_PDF_POSITIVE).toBe('1');
  test.setTimeout(180_000);
  const mobile = info.project.name.includes('mobile');
  const locale = mobile ? 'fa' : 'en';
  const suffix = mobile ? '801' : '701';
  const action = mobile ? 'reject' : 'accept';
  const assignmentId = `f1600000-0000-0000-0000-000000000${suffix}`;
  const title = `E2E Native PDF Positive ${suffix}`;
  const original = onePagePdf();
  const sha256 = createHash('sha256').update(original).digest('hex');
  await page.addInitScript(value => localStorage.setItem('edutalent_locale', value), locale);

  // Actual Student authentication, private original and immutable submission.
  await login(page, 'e2e-attachment-student@example.test');
  const reserve = await api(page).post('/api/submissions/attachments/reserve', {
    data: { input: {
      assignment_id: assignmentId, request_id: randomUUID(),
      filename: 'synthetic-answer.pdf', media_type: 'application/pdf',
      byte_size: original.length, sha256,
    } },
  });
  expect(reserve.ok(), `reserve: ${reserve.status()}`).toBeTruthy();
  const { id: attachmentId } = await reserve.json();
  const upload = await api(page).post(`/api/submissions/attachments/upload?id=${attachmentId}`, {
    data: original, headers: { 'content-type': 'application/pdf' },
  });
  expect(upload.ok(), `private original: ${upload.status()}`).toBeTruthy();
  const finish = await api(page).post('/api/submissions/finalize', {
    data: {
      assignment_id: assignmentId,
      content: 'Synthetic calculation in the attached original PDF.',
      request_id: randomUUID(),
      expected_revision: null,
      attachment_ids: [attachmentId],
    },
  });
  expect(finish.ok(), `finalize: ${finish.status()}`).toBeTruthy();
  const submitted = await api(page).post('/api/submissions/get_for_assignment', {
    data: { assignment_id: assignmentId },
  });
  expect(submitted.ok()).toBeTruthy();
  const submission = await submitted.json();
  expect(submission.grade).toBeNull();
  expect(submission.id).toBeTruthy();
  expect(submission.revision).toBeTruthy();

  // Only the assigned Teacher approves the precise 20-point rubric.
  await login(page, 'e2e-attachment-teacher@example.test');
  async function openReview() {
    await page.goto('/dashboard/submissions');
    const card = page.getByText(title, { exact: true }).first()
      .locator('xpath=ancestor::div[contains(@class, "overflow-hidden")][1]');
    await expect(card).toBeVisible();
    await card.getByRole('button').last().click();
    const panel = page.getByRole('dialog').locator('section').filter({
      hasText: mobile ? 'دستیار تصحیح هوشمند' : 'AI grading assistant',
    }).last();
    await expect(panel).toBeVisible();
    return panel;
  }
  let panel = await openReview();
  await panel.locator('textarea').fill(mobile ? 'دقت | ۲۰' : 'Accuracy | 20');
  await panel.getByRole('button', {
    name: mobile ? 'تأیید معیارها' : 'Approve rubric',
  }).click();

  // The actual background worker must claim this newly approved PDF job,
  // authorize Teacher/RLS, read the original, commit a bounded budget and
  // persist a private suggestion returned from the loopback gateway contract.
  await expect.poll(async () => {
    const current = await api(page).get(
      `/api/ai-grading/suggestions/current?submission_id=${submission.id}`,
    );
    if (!current.ok()) return 'not-ready';
    const suggestion = await current.json();
    return suggestion?.status === 'pending_review' ? 'ready' : 'not-ready';
  }, { timeout: 100_000, intervals: [500, 1000, 1500] }).toBe('ready');

  const proof = await page.context().request.get('http://127.0.0.1:8090/__e2e/pdf-calls');
  expect(proof.ok()).toBeTruthy();
  const calls = (await proof.json()) as Array<{
    submission_id: string; revision: string; attachment_id: string;
    sha256: string; input_bytes: number;
  }>;
  const matching = calls.filter(call => call.submission_id === submission.id);
  expect(matching, 'exactly one durable budget-approved gateway attempt').toHaveLength(1);
  expect(matching[0]).toEqual({
    submission_id: submission.id,
    revision: submission.revision,
    attachment_id: attachmentId,
    sha256,
    input_bytes: original.length,
  });

  panel = await openReview();
  await expect(panel).toContainText('18.00 / 20');
  await expect(panel).toContainText('Synthetic PDF grading evidence: Teacher review required.');
  const review = page.waitForResponse(response =>
    new URL(response.url()).pathname === '/api/ai-grading/suggestions/review'
  );
  await panel.getByRole('button', {
    name: action === 'accept' ? 'Accept & publish grade' : 'رد پیشنهاد',
  }).click();
  const decision = await review;
  expect(decision.ok(), `Teacher ${action}: ${decision.status()}`).toBeTruthy();
  expect(await decision.json()).toBe(true);

  await login(page, 'e2e-attachment-student@example.test');
  const after = await api(page).post('/api/submissions/get_for_assignment', {
    data: { assignment_id: assignmentId },
  });
  expect(after.ok()).toBeTruthy();
  expect((await after.json()).grade).toBe(action === 'accept' ? '18/20' : null);

  await login(page, 'e2e-attachment-parent@example.test');
  await page.goto('/dashboard/children');
  const child = page.getByText('E2E Attachment Student', { exact: true })
    .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
  await expect(child).toBeVisible();
  const grades = page.waitForResponse(response =>
    new URL(response.url()).pathname === '/api/parent/scoped/child/grades'
  );
  await child.getByRole('button', {
    name: mobile ? 'مشاهده نمرات' : 'View Grades', exact: true,
  }).click();
  const parent = await grades;
  expect(parent.ok()).toBeTruthy();
  const visible = (await parent.json()).filter((row: { assignment_title: string }) =>
    row.assignment_title === title
  );
  expect(visible).toHaveLength(action === 'accept' ? 1 : 0);
  if (action === 'accept') expect(visible[0].points).toBe('18/20');
});
