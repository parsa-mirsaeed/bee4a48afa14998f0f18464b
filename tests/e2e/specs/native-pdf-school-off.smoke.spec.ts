import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import { createHash, randomUUID } from 'node:crypto';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

// Real Dioxus application and real private Storage; no model provider calls.
// Dedicated fixture per desktop/mobile project, school opt-in starts OFF.
type Requests = Pick<APIRequestContext, 'get' | 'post'>;
const origin = new URL(process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8080');
function api(page: Page): Requests {
  const request = page.context().request;
  const call = async (method: 'get' | 'post', path: string, options: any = {}) => {
    const target = new URL(path, origin);
    const headers = { ...options.headers };
    if (target.origin === origin.origin && origin.protocol === 'http:'
        && ['localhost', '127.0.0.1'].includes(origin.hostname)) {
      headers.cookie = (await page.context().cookies())
        .filter(cookie => cookie.domain === origin.hostname && target.pathname.startsWith(cookie.path))
        .map(cookie => `${cookie.name}=${cookie.value}`).join('; ');
    }
    return request[method](path, { ...options, headers, maxRedirects: 0 });
  };
  return { get: (path, options) => call('get', path, options),
           post: (path, options) => call('post', path, options) };
}
async function login(page: Page, email: string) {
  const response = await api(page).post('/api/auth/login', {
    data: { email, password: 'e2e-password' },
  });
  expect(response.ok(), `failed to log in ${email}: ${response.status()}`).toBeTruthy();
}

function onePagePdf(): Buffer {
  let content = '%PDF-1.4\n';
  const offsets = [0];
  for (const [i, object] of [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>',
  ].entries()) {
    offsets.push(Buffer.byteLength(content));
    content += `${i + 1} 0 obj\n${object}\nendobj\n`;
  }
  const xref = Buffer.byteLength(content);
  content += 'xref\n0 4\n0000000000 65535 f \n';
  content += offsets.slice(1).map(n => `${String(n).padStart(10, '0')} 00000 n \n`).join('');
  content += `trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(content);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});
test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('student PDF and teacher-approved rubric cannot bypass disabled school inference @smoke @final @teacher @student @ai-grading @security', async ({ page }, info) => {
  test.setTimeout(120_000);
  const mobile = info.project.name.includes('mobile');
  const suffix = mobile ? '401' : '301';
  const locale = mobile ? 'fa' : 'en';
  const assignmentId = `f1600000-0000-0000-0000-000000000${suffix}`;
  const assignmentTitle = `E2E Native PDF Gate ${suffix}`;
  const buffer = onePagePdf();

  await page.addInitScript(selected => {
    localStorage.setItem('edutalent_locale', selected);
  }, locale);
  await login(page, 'e2e-attachment-student@example.test');
  const request = api(page);
  const reserve = await request.post('/api/submissions/attachments/reserve', {
    data: {
      input: {
        assignment_id: assignmentId,
        request_id: randomUUID(),
        filename: 'math-answer.pdf',
        media_type: 'application/pdf',
        byte_size: buffer.length,
        sha256: createHash('sha256').update(buffer).digest('hex'),
      },
    },
  });
  expect(reserve.ok(), `reserve failed: ${reserve.status()}`).toBeTruthy();
  const { id: attachmentId } = await reserve.json();

  const upload = await request.post(
    `/api/submissions/attachments/upload?id=${attachmentId}`, {
      data: buffer,
      headers: { 'content-type': 'application/pdf' },
    },
  );
  expect(upload.ok(), `private PDF upload failed: ${upload.status()}`).toBeTruthy();
  const finalize = await request.post('/api/submissions/finalize', {
    data: {
      assignment_id: assignmentId,
      content: 'My typed mathematical reasoning is included with the PDF.',
      request_id: randomUUID(),
      expected_revision: null,
      attachment_ids: [attachmentId],
    },
  });
  expect(finalize.ok(), `submission finalization failed: ${finalize.status()}`).toBeTruthy();
  const submitted = await request.post('/api/submissions/get_for_assignment', {
    data: { assignment_id: assignmentId },
  });
  expect(submitted.ok()).toBeTruthy();
  expect((await submitted.json()).grade).toBeNull();

  await login(page, 'e2e-attachment-teacher@example.test');
  await page.goto('/dashboard/submissions');
  const card = page.getByText(assignmentTitle, { exact: true }).first()
    .locator('xpath=ancestor::div[contains(@class, "overflow-hidden")][1]');
  await expect(card).toBeVisible();
  await card.getByRole('button').last().click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const panel = dialog.locator('section').filter({
    hasText: locale === 'fa' ? 'دستیار تصحیح هوشمند' : 'AI grading assistant',
  }).last();
  await expect(panel).toBeVisible();
  await expect.poll(() => page.evaluate(() => document.documentElement.dir))
    .toBe(locale === 'fa' ? 'rtl' : 'ltr');
  await expect(panel).toContainText(locale === 'fa' ? 'مجوز مدرسه' : 'school approval');

  await panel.locator('textarea').fill(locale === 'fa' ? 'دقت | ۲۰' : 'Accuracy | 20');
  await panel.getByRole('button', {
    name: locale === 'fa' ? 'تأیید معیارها' : 'Approve rubric',
  }).click();

  const generate = panel.getByRole('button', {
    name: locale === 'fa' ? 'تولید پیشنهاد نمره' : 'Generate AI suggestion',
  });
  await expect(generate).toBeVisible();
  await generate.click();
  await expect(panel.getByRole('status')).toContainText(
    locale === 'fa' ? 'عملیات تصحیح هوشمند انجام نشد' : 'The AI grading request could not be completed',
  );
  await expect(panel.getByRole('button', {
    name: locale === 'fa' ? 'تأیید و انتشار نمره' : 'Accept & publish grade',
  })).toHaveCount(0);

  // An unreviewed and unauthorized suggestion cannot become an official
  // student-visible grade, even after a full browser round trip.
  await login(page, 'e2e-attachment-student@example.test');
  const after = await api(page).post('/api/submissions/get_for_assignment', {
    data: { assignment_id: assignmentId },
  });
  expect(after.ok()).toBeTruthy();
  expect((await after.json()).grade).toBeNull();
});
