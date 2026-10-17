import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

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
    await expect(page.getByText(locale === 'fa'
      ? 'گفت‌وگو با هوش مصنوعی درباره وضعیت تحصیلی فرزندم'
      : "AI chat about your child's learning")).toBeVisible();

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
