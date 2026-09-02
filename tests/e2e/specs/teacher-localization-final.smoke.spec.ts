import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const RAW_TRANSLATION_KEY = /\b(?:teacher|teachers|students|assignments|grades|classes|materials|submissions|nav|common)\.[a-z0-9_.]+\b/i;
const TEACHER_ROUTES = [
  '/dashboard',
  '/dashboard/classes',
  '/dashboard/assignments',
  '/dashboard/knowledge-assets',
  '/dashboard/submissions',
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

async function waitForTeacherRouteData(page: Page, route: typeof TEACHER_ROUTES[number]): Promise<void> {
  switch (route) {
    case '/dashboard':
      await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
      break;
    case '/dashboard/classes':
      await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();
      break;
    case '/dashboard/assignments':
      await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
      break;
    case '/dashboard/knowledge-assets':
      await expect(page.getByText('E2E Published Asset', { exact: true }).first()).toBeVisible();
      break;
    case '/dashboard/students':
      await expect(page.getByText('E2E Student A', { exact: true }).first()).toBeVisible();
      break;
    case '/dashboard/submissions':
      await expect(page.locator('.animate-pulse')).toHaveCount(0);
      break;
  }
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
      await waitForTeacherRouteData(page, route);
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

    const layout = await card.evaluate((element) => {
      const stats = Array.from(element.querySelectorAll('div')).find((candidate) =>
        candidate.getAttribute('style')?.includes('text-align: end'),
      );
      const submittedLabel = stats?.querySelector('div:last-child');
      const statsStyle = stats ? getComputedStyle(stats) : null;
      const labelStyle = submittedLabel ? getComputedStyle(submittedLabel) : null;
      return {
        clientWidth: element.clientWidth,
        scrollWidth: element.scrollWidth,
        statsMaxWidth: statsStyle?.maxWidth ?? '',
        statsTextAlign: statsStyle?.textAlign ?? '',
        labelOverflowWrap: labelStyle?.overflowWrap ?? '',
      };
    });
    expect(layout.scrollWidth).toBeLessThanOrEqual(layout.clientWidth + 1);
    expect(layout.statsMaxWidth).toBe('128px');
    expect(['start', 'end', 'left', 'right']).toContain(layout.statsTextAlign);
    expect(['anywhere', 'break-word']).toContain(layout.labelOverflowWrap);
  });
}
