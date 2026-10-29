// @smoke @aurora @accessibility — presentation regression for Aurora Workspace.
// This tests rendered styles and navigation instead of trusting token declarations.
import { test, expect, type Page, type TestInfo } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill('e2e-password');
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('Aurora shell and shared role navigation render with the new visual contract @smoke @aurora', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await signIn(page, 'e2e-manager-a@example.test');

  const sidebar = page.locator('.et-sidebar');
  const hero = page.locator('.et-overview-intro');
  await expect(sidebar).toHaveCSS('background-color', 'rgb(20, 24, 46)');
  await expect(sidebar).toHaveCSS('width', '276px');
  await expect(page.locator('.et-topbar-kicker')).toHaveText('Workspace');
  await expect(hero).toBeVisible();
  await expect(hero).toHaveCSS('border-radius', '24px');
  await expect(page.locator('.et-ui-sidebar-nav__item[aria-current="page"]')).toHaveCount(1);

  await testInfo.attach('aurora-desktop', {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });

  await page.locator('.et-ui-sidebar-nav__item', { hasText: 'Users' }).first().click();
  await expect(page).toHaveURL(/\/dashboard\/users$/);
  await expect(page.locator('.et-ui-sidebar-nav__item[aria-current="page"]')).toHaveCount(1);
});

test('Aurora mobile drawer keeps keyboard lifecycle and reduced-motion @smoke @aurora @accessibility', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await signIn(page, 'e2e-manager-a@example.test');

  const open = page.getByRole('button', { name: 'Open navigation' });
  const sidebar = page.locator('.et-sidebar');
  await expect(open).toBeVisible();
  await expect(page.locator('.et-overview-intro')).toHaveCSS('border-radius', '19px');

  await open.click();
  await expect(sidebar).toHaveClass(/et-sidebar--mobile-open/);
  await expect(sidebar.locator('.et-sidebar-mobile-close')).toBeFocused();
  await expect(sidebar.locator('.et-ui-sidebar-nav__item').first()).toHaveCSS('transition-duration', '0s');
  await page.keyboard.press('Escape');
  await expect(open).toBeFocused();

  await testInfo.attach('aurora-mobile', {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});
