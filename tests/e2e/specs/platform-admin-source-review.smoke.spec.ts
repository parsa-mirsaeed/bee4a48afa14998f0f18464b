import { test, expect } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';

async function signInAdmin(page: import('@playwright/test').Page): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
  await page.goto('/');
  await page.locator('input[type="email"]').fill('e2e-admin@example.test');
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

test('platform admin source storage failure is bounded product UI @smoke @final @workflow-truth', async ({ page }) => {
  await signInAdmin(page);

  const card = page
    .getByText('E2E Verified OCR Asset', { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(card).toBeVisible();

  const popupPromise = page.waitForEvent('popup');
  await card.getByRole('link', { name: 'Review private PDF', exact: true }).click();
  const popup = await popupPromise;
  await popup.waitForLoadState('domcontentloaded');

  await expect(popup.getByRole('alert')).toContainText('Source review unavailable');
  await expect(popup.getByRole('alert')).toContainText('Source document is unavailable');
  await expect(popup.locator('body')).not.toContainText('{"error"');
  await expect(popup.locator('body')).not.toContainText('storage/v1/object');
  await popup.close();
});
