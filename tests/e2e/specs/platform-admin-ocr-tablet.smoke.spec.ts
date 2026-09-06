import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const ADMIN = 'e2e-admin@example.test';
const TABLET_VIEWPORT = { width: 1024, height: 768 };

async function openAdminDashboard(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await page.setViewportSize(TABLET_VIEWPORT);
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);

  const login = await page.context().request.post('/api/auth/login', {
    data: { email: ADMIN, password: PASSWORD },
  });
  expect(login.ok(), 'platform admin session setup failed').toBeTruthy();

  const response = await page.goto('/dashboard');
  expect(response === null || response.status() < 400).toBeTruthy();
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
    new RegExp(`^${locale}`, 'i'),
  );
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe(
    locale === 'fa' ? 'rtl' : 'ltr',
  );
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

for (const scenario of [
  {
    locale: 'en' as const,
    updateOcr: 'Update verified OCR',
    verifiedText: 'Verified source text',
    cancel: 'Cancel',
    details: 'Verification details',
  },
  {
    locale: 'fa' as const,
    updateOcr: 'به‌روزرسانی OCR تأییدشده',
    verifiedText: 'متن تأییدشده منبع',
    cancel: 'انصراف',
    details: 'جزئیات تأیید',
  },
]) {
  test(`platform admin OCR editor stays usable at 1024x768 in ${scenario.locale} @smoke @platform-admin @i18n @workflow-truth @tablet`, async ({ page }) => {
    await openAdminDashboard(page, scenario.locale);

    const card = page.locator('article').filter({
      has: page.getByText('E2E Verified OCR Asset', { exact: true }),
    });
    const trigger = card.getByRole('button', { name: scenario.updateOcr, exact: true });
    await expect(trigger).toBeVisible();
    await trigger.click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toBeInViewport({ ratio: 1 });
    await expect(dialog).toBeFocused();

    const editor = dialog.getByLabel(scenario.verifiedText, { exact: true });
    await expect(editor).toBeVisible();
    await expect(editor).toBeInViewport({ ratio: 1 });
    await expect(editor).toHaveValue('E2E preverified OCR text');

    for (const element of [dialog, editor, dialog.getByRole('button', { name: scenario.cancel, exact: true })]) {
      const box = await element.boundingBox();
      expect(box).not.toBeNull();
      expect(box!.x).toBeGreaterThanOrEqual(0);
      expect(box!.y).toBeGreaterThanOrEqual(0);
      expect(box!.x + box!.width).toBeLessThanOrEqual(TABLET_VIEWPORT.width);
      expect(box!.y + box!.height).toBeLessThanOrEqual(TABLET_VIEWPORT.height);
    }

    const horizontalOverflow = await page.evaluate(() =>
      document.documentElement.scrollWidth > document.documentElement.clientWidth,
    );
    expect(horizontalOverflow, 'tablet viewport must not introduce page-level horizontal overflow').toBeFalsy();

    const details = dialog.locator('details');
    await details.locator('summary').filter({ hasText: scenario.details }).click();
    await expect(details.locator('code').first()).toBeVisible();
    await expect(details.locator('code')).toHaveCount(5);
    await details.locator('summary').click();
    await expect(editor).toHaveValue('E2E preverified OCR text');

    await dialog.getByRole('button', { name: scenario.cancel, exact: true }).click();
    await expect(dialog).toHaveCount(0);
    await expect(trigger).toBeFocused();

    // Reopening must capture a fresh return target and preserve Escape parity.
    await trigger.click();
    await expect(dialog).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);
    await expect(trigger).toBeFocused();
  });
}
