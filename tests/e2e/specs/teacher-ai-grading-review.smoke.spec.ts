import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

// Read-only browser acceptance: exercise the actual bilingual Teacher grading
// modal with a seeded, ungraded text assignment. Never request model inference,
// approve a rubric, or publish a grade from the shared E2E fixture.
const teacher = 'e2e-teacher-a@example.test';
const targetAssignment = 'E2E Authorization Submission A';

async function establishSession(page: Page, locale: 'en' | 'fa') {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);
  const auth = await page.request.post('/api/auth/login', {
    data: { email: teacher, password: 'e2e-password' },
  });
  expect(auth.ok(), `Teacher login failed: ${auth.status()}`).toBeTruthy();
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
    title: 'AI grading assistant',
    description: 'AI suggestions are private until you accept and publish a grade.',
    rubric: 'Teacher rubric: one criterion per line',
    invalid: 'Use 1–20 distinct criteria.',
  },
  {
    locale: 'fa' as const,
    title: 'دستیار تصحیح هوشمند',
    description: 'نمره پیشنهادی هوش مصنوعی تا زمان تأیید شما خصوصی است.',
    rubric: 'معیارهای معلم: هر سطر',
    invalid: 'بین ۱ تا ۲۰ معیار متفاوت',
  },
]) {
  test(`teacher AI grading panel is localized, rubric validation is fail-closed in ${scenario.locale} @smoke @teacher @ai-grading @i18n @rtl`, async ({ page }) => {
    await establishSession(page, scenario.locale);
    await page.goto('/dashboard/submissions');
    await expect(page).toHaveURL(/\/dashboard\/submissions$/);
    await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
      new RegExp(`^${scenario.locale}`, 'i'),
    );
    await expect.poll(() => page.evaluate(() => document.documentElement.dir))
      .toBe(scenario.locale === 'fa' ? 'rtl' : 'ltr');

    const card = page.getByText(targetAssignment, { exact: true }).first()
      .locator('xpath=ancestor::div[contains(@class, "overflow-hidden")][1]');
    await expect(card).toBeVisible();
    await card.getByRole('button').last().click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    // The Material icon contributes text to the heading element, so exact
    // heading text locators are invalid even though the panel is visible.
    const panel = dialog.locator('section').filter({ hasText: scenario.title }).last();
    await expect(panel).toBeVisible();
    await expect(panel).toContainText(scenario.title);
    await expect(panel).toContainText(scenario.description);
    await expect(panel).not.toContainText('ai_grading.');

    // The seeded assignment has no approved AI rubric. This path must
    // validate a Teacher-authored rubric locally before any server mutation.
    await expect(panel).toContainText(scenario.rubric);
    const criteria = panel.locator('textarea');
    await expect(criteria).toBeVisible();
    await criteria.fill('duplicated | 50\nduplicated | 50');
    await panel.getByRole('button', {
      name: scenario.locale === 'fa' ? 'تأیید معیارها' : 'Approve rubric',
    }).click();
    await expect(panel).toContainText(scenario.invalid);
    await expect(panel.getByRole('button', {
      name: scenario.locale === 'fa' ? 'تأیید و انتشار نمره' : 'Accept & publish grade',
    })).toHaveCount(0);
    await expect(panel).not.toContainText('grading.');
  });
}
