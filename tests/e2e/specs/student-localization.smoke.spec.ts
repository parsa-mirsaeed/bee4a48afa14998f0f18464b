import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const STUDENT_ROUTES = [
  '/dashboard',
  '/dashboard/classes',
  '/dashboard/assignments',
  '/dashboard/grades',
] as const;
const RAW_TRANSLATION_KEY = /\b(?:student|assignments|grades|classes|materials|nav|common)\.[a-z0-9_.]+\b/i;
const ENGLISH_MONTH = /\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/;
const RAW_UTC_TIMESTAMP = /\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?Z\b/;

async function signInStudent(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);
  await page.goto('/');
  await page.locator('input[type="email"]').fill('e2e-student-a@example.test');
  await page.locator('input[type="password"]').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

async function waitForStudentRouteData(page: Page, route: (typeof STUDENT_ROUTES)[number]): Promise<void> {
  switch (route) {
    case '/dashboard':
    case '/dashboard/classes':
    case '/dashboard/grades':
      await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();
      break;
    case '/dashboard/assignments':
      await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
      break;
  }
}

async function assertLocalizedStudentChrome(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
    locale === 'fa' ? /^fa/i : /^en/i,
  );
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe(
    locale === 'fa' ? 'rtl' : 'ltr',
  );

  const visibleText = await page.locator('body').innerText();
  expect(visibleText).not.toMatch(RAW_TRANSLATION_KEY);
  expect(visibleText).not.toMatch(RAW_UTC_TIMESTAMP);

  if (locale === 'fa') {
    expect(visibleText).not.toMatch(ENGLISH_MONTH);
    for (const untranslatedChrome of [
      'Assignments',
      'Grades',
      'My Classes',
      'Pending',
      'Overdue',
      'Submitted',
      'Graded',
      'View Details',
      'Recorded grades',
      'Assignment details',
      'My submission',
    ]) {
      await expect(page.getByText(untranslatedChrome, { exact: true })).toHaveCount(0);
    }
  }
}

async function openClassAction(page: Page, icon: 'folder_open' | 'assignment' | 'show_chart'): Promise<void> {
  const classCard = page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
  await expect(classCard).toBeVisible();
  await classCard.locator(`xpath=.//button[.//span[normalize-space()="${icon}"]]`).click();
  await expect(page.getByRole('dialog')).toBeVisible();
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
  test(`all enabled student destinations keep localized product chrome in ${locale} @smoke @student @i18n`, async ({ page }) => {
    await signInStudent(page, locale);

    for (const route of STUDENT_ROUTES) {
      await page.goto(route);
      await expect(page).toHaveURL(new RegExp(`${route.replaceAll('/', '\\/')}$`));
      await waitForStudentRouteData(page, route);
      await assertLocalizedStudentChrome(page, locale);
    }
  });

  test(`student class assignments grades and materials dialogs stay localized in ${locale} @smoke @student @i18n`, async ({ page }) => {
    await signInStudent(page, locale);
    await page.goto('/dashboard/classes');
    await waitForStudentRouteData(page, '/dashboard/classes');

    await openClassAction(page, 'assignment');
    let dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('E2E Assignment A1');
    await assertLocalizedStudentChrome(page, locale);
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);

    await openClassAction(page, 'show_chart');
    dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('E2E Assignment A1');
    await assertLocalizedStudentChrome(page, locale);
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);

    await openClassAction(page, 'folder_open');
    dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('E2E Class Material A1');
    await assertLocalizedStudentChrome(page, locale);
  });

  test(`student assignment details and submission editor stay localized in ${locale} @smoke @student @i18n`, async ({ page }) => {
    await signInStudent(page, locale);
    await page.goto('/dashboard/assignments');
    await waitForStudentRouteData(page, '/dashboard/assignments');

    const card = page
      .getByText('E2E Submission Journey Mobile', { exact: true })
      .locator('xpath=ancestor::article[1]');
    await expect(card).toBeVisible();
    // This localization probe uses the dedicated non-mutated journey fixture so
    // earlier lifecycle acceptance cannot turn its submission editor into a
    // graded read-only view within the same final-suite process.
    await card.getByRole('button').first().click();

    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await assertLocalizedStudentChrome(page, locale);

    await dialog.getByRole('button', {
      name: locale === 'fa' ? 'باز کردن ارسال من' : 'Open my submission',
      exact: true,
    }).click();
    dialog = page.getByRole('dialog');
    await expect(dialog.locator('textarea')).toBeVisible();
    await assertLocalizedStudentChrome(page, locale);
  });

  test(`student recorded grade details stay localized in ${locale} @smoke @student @i18n`, async ({ page }) => {
    await signInStudent(page, locale);
    await page.goto('/dashboard/grades');
    await waitForStudentRouteData(page, '/dashboard/grades');

    const classCard = page
      .getByText('E2E Class A1', { exact: true })
      .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
    await classCard.getByRole('button').click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('E2E Assignment A1');
    await assertLocalizedStudentChrome(page, locale);
  });
}