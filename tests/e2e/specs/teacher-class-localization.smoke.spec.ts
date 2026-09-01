// @smoke @final @teacher @i18n — Teacher class grading/material localization acceptance.
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
  await page.goto('/dashboard/classes');
  await expect(page).toHaveURL(/\/dashboard\/classes$/);
}

function classCard(page: Page) {
  return page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
}

async function openClassAction(page: Page, icon: 'assignment' | 'folder'): Promise<void> {
  const card = classCard(page);
  await expect(card).toBeVisible();
  await card.locator(`xpath=.//button[.//span[normalize-space()="${icon}"]]`).click();
  await expect(page.getByRole('dialog')).toBeVisible();
}

async function expectNoRawTeacherKeys(page: Page): Promise<void> {
  const text = await page.getByRole('dialog').innerText();
  expect(text).not.toMatch(/\b(?:teachers|assignments|students|materials|nav)\.[a-z0-9_.-]+/i);
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
  { locale: 'en' as const, status: 'Published', emptyMaterials: 'No Materials Yet' },
  { locale: 'fa' as const, status: 'منتشرشده', emptyMaterials: 'منبعی وجود ندارد' },
]) {
  test(`teacher class grading and materials are localized in ${scenario.locale} @smoke @final @teacher @i18n`, async ({ page }) => {
    await signIn(page, scenario.locale);

    await openClassAction(page, 'assignment');
    let dialog = page.getByRole('dialog');
    await expect(dialog.getByText('E2E Assignment A1', { exact: true })).toBeVisible();
    await expect(dialog).toContainText(scenario.status);
    await expect(dialog).not.toContainText(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/);
    await expectNoRawTeacherKeys(page);

    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText('Published');
      await expect(dialog).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }

    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);

    await openClassAction(page, 'folder');
    dialog = page.getByRole('dialog');
    await expect(dialog).toContainText(scenario.emptyMaterials);
    await expect(dialog).not.toContainText(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/);
    await expectNoRawTeacherKeys(page);

    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText('No Materials Yet');
    }
  });
}
