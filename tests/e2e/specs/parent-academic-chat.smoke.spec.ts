import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { allowHttpResponse, watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';

async function signIn(page: Page, email: string, locale: 'en' | 'fa') {
  await page.addInitScript(value => localStorage.setItem('edutalent_locale', value), locale);
  const response = await page.context().request.post('/api/auth/login', {
    data: { email, password: PASSWORD },
  });
  expect(response.ok()).toBeTruthy();
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});
test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

for (const locale of ['en', 'fa'] as const) {
  test(`Parent academic AI chat preserves selected-child scope and truthful unavailable state in ${locale} @smoke @parent @workflow-truth @i18n`, async ({ page }) => {
    await signIn(page, 'e2e-parent-a@example.test', locale);
    await page.goto('/dashboard');

    const open = locale === 'fa' ? 'باز کردن گفت‌وگوی تحصیلی' : 'Open academic AI chat';
    await page.getByRole('button', { name: open }).click();
    await expect(page).toHaveURL(/\/dashboard\/ai-chat$/);
    // The same title occurs in the navigation, topbar, and chat heading.
    // Prove the actual chat panel is visible rather than weakening to .first().
    await expect(page.getByRole('heading', {
      level: 2,
      name: locale === 'fa'
        ? 'گفت‌وگو با هوش مصنوعی درباره وضعیت تحصیلی فرزندم'
        : "AI chat about your child's learning",
    })).toBeVisible();

    const childSelector = page.getByRole('combobox', {
      name: locale === 'fa' ? 'دانش‌آموز' : 'Student',
    });
    await expect(childSelector.locator('option')).toHaveCount(2);
    await childSelector.selectOption({ label: 'E2E Student A' });

    await page.getByRole('textbox', {
      name: locale === 'fa' ? 'پرسش شما' : 'Your question',
    }).fill(locale === 'fa'
      ? 'کدام تکلیف‌ها هنوز انجام نشده‌اند؟'
      : 'Which assignments are pending?');
    const ask = page.getByRole('button', {
      name: locale === 'fa' ? 'پرسش از هوش مصنوعی' : 'Ask AI',
    });
    await expect(ask).toBeDisabled();
    await page.getByRole('checkbox', {
      name: /processed by the school's AI provider|پردازش می‌شوند/,
    }).check();
    await expect(ask).toBeEnabled();
    // Gateway is intentionally absent in this offline browser proof. Only
    // this exact endpoint/status is expected; all other HTTP failures still
    // fail the browser console guard.
    allowHttpResponse('/api/parent/scoped/child/academic-chat', 500);
    await ask.click();

    // The browser runner deliberately has no AI gateway. Never forge academic
    // facts, turn old assignments into generated advice or silently retry
    // another provider. Existing Parent Grades/Assignments remain available.
    await expect(page.getByRole('alert')).toContainText(locale === 'fa'
      ? 'راهنمایی هوش مصنوعی در دسترس نیست'
      : 'AI guidance is unavailable');
    await expect(page.getByRole('log').locator('article')).toHaveCount(0);
    await expect(page.locator('body')).not.toContainText('AI gateway internal token');
  });
}

test('Unlinked Parent sees no selectable student and cannot trigger AI chat @smoke @parent @workflow-truth', async ({ page }) => {
  await signIn(page, 'e2e-parent-empty@example.test', 'en');
  await page.goto('/dashboard/ai-chat');
  await expect(page.getByText('School administration must link a student before academic information appears.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Ask AI' })).toHaveCount(0);
});

test('Parent AI endpoint rejects cross-school child IDs and Teacher roles @smoke @parent @authorization @workflow-truth', async ({ page }) => {
  await signIn(page, 'e2e-parent-a@example.test', 'en');
  await page.goto('/dashboard/ai-chat');
  await page.getByRole('combobox', { name: 'Student' }).selectOption({ label: 'E2E Student A' });
  await page.getByRole('textbox', { name: 'Your question' }).fill('Which assignments are late?');
  await page.getByRole('checkbox', { name: /processed by the school's AI provider/ }).check();

  const captured = page.waitForRequest(req =>
    new URL(req.url()).pathname === '/api/parent/scoped/child/academic-chat');
  allowHttpResponse('/api/parent/scoped/child/academic-chat', 500);
  await page.getByRole('button', { name: 'Ask AI' }).click();
  const original = await captured;
  await expect(page.getByRole('alert')).toContainText('AI guidance is unavailable');

  const body = original.postData();
  expect(body).toContain('c0000000-0000-0000-0000-0000000000a3');
  const headers = { 'content-type': original.headers()['content-type'] };
  const crossSchool = await page.context().request.post(original.url(), {
    headers,
    data: body!.replace(
      'c0000000-0000-0000-0000-0000000000a3',
      'c0000000-0000-0000-0000-0000000000b3',
    ),
  });
  expect(crossSchool.status()).toBeGreaterThanOrEqual(400);
  // Server-function errors may be deliberately bodyless at the HTTP boundary.
  // The security invariant is denial before an AI answer or child records
  // become visible, not leaking the internal child-ownership error code.
  const crossSchoolBody = await crossSchool.text();
  expect(crossSchoolBody).not.toContain('academic_records');
  expect(crossSchoolBody).not.toContain('E2E Student B');

  await signIn(page, 'e2e-teacher-a@example.test', 'en');
  const wrongRole = await page.context().request.post(original.url(), { headers, data: body! });
  expect(wrongRole.status()).toBeGreaterThanOrEqual(400);
  const wrongRoleBody = await wrongRole.text();
  expect(wrongRoleBody).not.toContain('academic_records');
  expect(wrongRoleBody).not.toContain('E2E Student A');
});
