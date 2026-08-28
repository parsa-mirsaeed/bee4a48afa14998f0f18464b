// @smoke @pr2 @accessibility — interaction-size and modal focus-regression proof.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

async function useEnglish(page: Page): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
}

async function expectMinimumTarget(locator: ReturnType<Page['locator']>): Promise<void> {
  const box = await locator.boundingBox();
  expect(box, 'interactive target must have a rendered box').not.toBeNull();
  expect(box!.width, `target width was ${box!.width}px`).toBeGreaterThanOrEqual(44);
  expect(box!.height, `target height was ${box!.height}px`).toBeGreaterThanOrEqual(44);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('login dialog traps keyboard focus, restores its trigger, and shared targets meet the 44px contract @smoke @pr2 @accessibility', async ({ page }) => {
  await useEnglish(page);
  await page.goto('/');

  const passwordField = page.locator('input[type="password"]').locator('xpath=ancestor::div[contains(@class,"et-ui-field")][1]');
  const reveal = passwordField.getByRole('button', { name: 'Show password' });
  await expectMinimumTarget(reveal);

  const recoveryTrigger = page.locator('.et-auth-help');
  await recoveryTrigger.click();

  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog).toBeFocused();

  const closeButton = dialog.getByRole('button', { name: 'Close', exact: true });
  await expect(closeButton).toHaveCount(1);
  await expectMinimumTarget(closeButton);

  await page.keyboard.press('Tab');
  await expect(closeButton).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(closeButton).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(closeButton).toBeFocused();

  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(recoveryTrigger).toBeFocused();
});
