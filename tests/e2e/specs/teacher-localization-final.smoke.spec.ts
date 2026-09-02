import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const RAW_TRANSLATION_KEY = /\b(?:teachers|students|assignments|grades|classes|materials|submissions|nav|common)\.[a-z0-9_.]+\b/i;
const TEACHER_ROUTES = [
  '/dashboard',
  '/dashboard/classes',
  '/dashboard/assignments',
  '/dashboard/knowledge-assets',
  '/dashboard/grading',
  '/dashboard/students',
] as const;

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

async function assertLocalizedTeacherChrome(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
    locale === 'fa' ? /^fa/i : /^en/i,
  );
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe(
    locale === 'fa' ? 'rtl' : 'ltr',
  );

  const visibleText = await page.locator('body').innerText();
  expect(visibleText).not.toMatch(RAW_TRANSLATION_KEY);

  if (locale === 'fa') {
    for (const untranslatedChrome of [
      'Create assignment',
      'View details',
      'Pending',
      'Submitted',
      'Graded',
      'Published',
      'Platform Administrator',
    ]) {
      await expect(page.getByText(untranslatedChrome, { exact: true })).toHaveCount(0);
    }
  }
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
  test(`all enabled teacher destinations keep localized product chrome in ${locale} @smoke @final @teacher @i18n`, async ({ page }) => {
    await signInTeacher(page, locale);

    for (const route of TEACHER_ROUTES) {
      await page.goto(route);
      await expect(page).toHaveURL(new RegExp(`${route.replaceAll('/', '\\/')}$`));
      await assertLocalizedTeacherChrome(page, locale);
    }
  });

  test(`teacher student cards tolerate localized labels in ${locale} @smoke @final @teacher @i18n @rtl`, async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 768 });
    await signInTeacher(page, locale);
    await page.goto('/dashboard/students');

    const card = page
      .getByText('E2E Student A', { exact: true })
      .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
    await expect(card).toBeVisible();
    await assertLocalizedTeacherChrome(page, locale);

    const overflow = await card.evaluate((element) => ({
      clientWidth: element.clientWidth,
      scrollWidth: element.scrollWidth,
    }));
    expect(overflow.scrollWidth).toBeLessThanOrEqual(overflow.clientWidth + 1);

    const stats = card.locator('.max-w-\\[7rem\\]').first();
    await expect(stats).toBeVisible();
    await expect(stats).toHaveCSS('text-align', locale === 'fa' ? 'start' : 'end');
  });
}
