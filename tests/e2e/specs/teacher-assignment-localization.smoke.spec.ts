import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const TEACHER = 'e2e-teacher-a@example.test';

async function signIn(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);
  await page.goto('/');
  await page.locator('input[type="email"]').fill(TEACHER);
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

for (const scenario of [
  {
    locale: 'en' as const,
    create: 'Create assignment',
    viewDetails: 'View details',
    detailsTitle: 'Assignment details',
    titleLabel: 'Title',
    classLabel: 'Class',
    dueLabel: 'Due date',
    instructionsLabel: 'Instructions',
    createDraft: 'Create draft',
    status: 'Published',
  },
  {
    locale: 'fa' as const,
    create: 'ایجاد تکلیف',
    viewDetails: 'مشاهده جزئیات',
    detailsTitle: 'جزئیات تکلیف',
    titleLabel: 'عنوان',
    classLabel: 'کلاس',
    dueLabel: 'تاریخ مهلت',
    instructionsLabel: 'دستورالعمل‌ها',
    createDraft: 'ایجاد پیش‌نویس',
    status: 'منتشرشده',
  },
]) {
  test(`teacher assignment create/detail chrome is localized in ${scenario.locale} @smoke @final @teacher @i18n`, async ({ page }) => {
    await signIn(page, scenario.locale);
    await page.goto('/dashboard/assignments');
    await expect(page).toHaveURL(/\/dashboard\/assignments$/);
    await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
      new RegExp(`^${scenario.locale}`, 'i'),
    );

    const pageBody = page.locator('body');
    await expect(pageBody).not.toContainText('teacher.assignments.');

    const assignmentCard = page
      .getByText('E2E Assignment A1', { exact: true })
      .locator('xpath=ancestor::article[1]');
    await expect(assignmentCard).toBeVisible();
    await expect(assignmentCard).toContainText(scenario.status);

    if (scenario.locale === 'fa') {
      await expect(assignmentCard).not.toContainText('Published');
      await expect(assignmentCard).not.toContainText('Due ');
      await expect(assignmentCard).not.toContainText(' submitted');
      await expect(assignmentCard).not.toContainText('View details');
      await expect(assignmentCard).not.toContainText('Delete');
      await expect(assignmentCard).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }

    await page.getByRole('button', { name: scenario.create, exact: true }).click();
    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAccessibleName(scenario.create);
    await expect(dialog).toContainText(scenario.titleLabel);
    await expect(dialog).toContainText(scenario.classLabel);
    await expect(dialog).toContainText(scenario.dueLabel);
    await expect(dialog).toContainText(scenario.instructionsLabel);
    await expect(dialog.getByRole('button', { name: scenario.createDraft, exact: true })).toBeVisible();
    await expect(dialog).not.toContainText('teacher.assignments.');
    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText('Create draft');
      await expect(dialog).not.toContainText('Select one of your classes');
      await expect(dialog).not.toContainText('Instructions');
    }
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);

    await assignmentCard.getByRole('button', { name: scenario.viewDetails, exact: true }).click();
    dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAccessibleName(scenario.detailsTitle);
    await expect(dialog).toContainText(scenario.status);
    await expect(dialog).not.toContainText('teacher.assignments.');
    await expect(dialog).not.toContainText(/T\d{2}:\d{2}:\d{2}/);
    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText('Status:');
      await expect(dialog).not.toContainText('Published');
      await expect(dialog).not.toContainText('Due ');
      await expect(dialog).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }
  });
}
