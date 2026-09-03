import AxeBuilder from '@axe-core/playwright';
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const STUDENT = 'e2e-student-a@example.test';
const PENDING_ASSIGNMENT = 'E2E Submission Journey Desktop';

async function openAssignments(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);
  const login = await page.context().request.post('/api/auth/login', {
    data: { email: STUDENT, password: PASSWORD },
  });
  expect(login.ok(), 'student session setup failed').toBeTruthy();
  const response = await page.goto('/dashboard/assignments');
  expect(response === null || response.status() < 400).toBeTruthy();
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
    new RegExp(`^${locale}`, 'i'),
  );
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe(
    locale === 'fa' ? 'rtl' : 'ltr',
  );
}

async function expectNoSeriousAxeViolations(page: Page, include?: string): Promise<void> {
  let builder = new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']);
  if (include) builder = builder.include(include);
  const result = await builder.analyze();
  const serious = result.violations.filter(
    (violation) => violation.impact === 'serious' || violation.impact === 'critical',
  );
  expect(
    serious,
    serious.map((violation) => `${violation.id}: ${violation.help}`).join('\n'),
  ).toEqual([]);
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
    all: 'All',
    pending: 'Pending',
    start: 'Start assignment',
    openSubmission: 'Open my submission',
    work: 'My submission',
  },
  {
    locale: 'fa' as const,
    all: 'همه',
    pending: 'در انتظار',
    start: 'شروع تکلیف',
    openSubmission: 'باز کردن ارسال من',
    work: 'ارسال من',
  },
]) {
  test(`student assignment filters and submission editor are accessible in ${scenario.locale} @smoke @final @student @accessibility @rtl`, async ({ page }) => {
    await openAssignments(page, scenario.locale);

    const allFilter = page.getByRole('button', { name: scenario.all, exact: true });
    const pendingFilter = page.getByRole('button', { name: scenario.pending, exact: true });
    await expect(allFilter).toHaveAttribute('aria-pressed', 'true');
    await expect(pendingFilter).toHaveAttribute('aria-pressed', 'false');
    await pendingFilter.click();
    await expect(pendingFilter).toHaveAttribute('aria-pressed', 'true');
    await expect(allFilter).toHaveAttribute('aria-pressed', 'false');

    await expectNoSeriousAxeViolations(page);

    const card = page
      .getByText(PENDING_ASSIGNMENT, { exact: true })
      .locator('xpath=ancestor::article[1]');
    await expect(card).toBeVisible();
    await card.getByRole('button', { name: scenario.start, exact: true }).click();

    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: scenario.openSubmission, exact: true }).click();

    dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAccessibleName(scenario.work);
    const editor = dialog.getByLabel(scenario.work, { exact: true });
    await expect(editor).toBeVisible();
    await expect(dialog).toBeFocused();
    await expectNoSeriousAxeViolations(page, '[role="dialog"]');

    await page.keyboard.press('Tab');
    await expect(dialog.getByRole('button').first()).toBeFocused();
    await page.keyboard.press('Tab');
    await expect(editor).toBeFocused();

    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toHaveCount(0);
  });
}
