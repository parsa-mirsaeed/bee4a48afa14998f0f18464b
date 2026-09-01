import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const ENGLISH_MONTH = /\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/;
const PERSIAN_MONTH = /(?:ژانویه|فوریه|مارس|آوریل|مه|ژوئن|ژوئیه|اوت|سپتامبر|اکتبر|نوامبر|دسامبر)/;

async function signInTeacher(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);
  await page.goto('/');
  await page.locator('input[type="email"]').fill('e2e-teacher-a@example.test');
  await page.locator('input[type="password"]').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
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
  test(`teacher overview assignment chrome is localized in ${locale} @smoke @final @teacher @i18n`, async ({ page }) => {
    await signInTeacher(page, locale);

    const row = page
      .getByText('E2E Assignment A1', { exact: true })
      .locator('xpath=ancestor::div[contains(@class, "et-list-row")][1]');
    await expect(row).toBeVisible();
    const date = row.locator('.et-list-aside p.mt-1');

    if (locale === 'en') {
      await expect(row).toContainText('Published');
      await expect(row).toContainText('Complete');
      await expect(row).toContainText('submitted');
      await expect(date).toContainText(ENGLISH_MONTH);
    } else {
      await expect(row).toContainText('منتشرشده');
      await expect(row).toContainText('تکمیل‌شده');
      await expect(row).toContainText('ارسال‌شده');
      await expect(row).not.toContainText(/\bsubmitted\b/i);
      await expect(date).not.toContainText(ENGLISH_MONTH);
      await expect(date).toContainText(PERSIAN_MONTH);
      await expect(date).toContainText(/[۰-۹]/);
    }

    await expect(row).not.toContainText(/(?:teachers\.|assignments\.|students\.|nav\.)/);
  });
}
