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

  await page.locator('.et-ui-sidebar-nav__item', { has: page.locator('span.material-icons-outlined', { hasText: /^groups$/ }) }).first().click();
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

test('teacher priority shows persisted grading workload and opens submissions @smoke @aurora @teacher', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await signIn(page, 'e2e-teacher-a@example.test');
  const priority = page.locator('.et-priority-panel');
  await expect(priority).toBeVisible();
  await expect(priority.locator('.et-priority-panel__value')).toContainText(/^\d+$/);
  await priority.locator('.et-priority-panel__action').click();
  await expect(page).toHaveURL(/\/dashboard\/submissions$/);
});

test('student task summary opens the real assignment list @smoke @aurora @student', async ({ page }) => {
  await signIn(page, 'e2e-student-a@example.test');
  await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();
  const summary = page.locator('.et-student-progress');
  await expect(summary).toBeVisible();
  await summary.locator('.et-student-progress__action').click();
  await expect(page).toHaveURL(/\/dashboard\/assignments$/);
  await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
});

test('parent academic assistance stays scoped to a real enabled route @smoke @aurora @parent', async ({ page }) => {
  await signIn(page, 'e2e-parent-a@example.test');
  await expect(page.getByText('E2E Student A', { exact: true })).toBeVisible();
  const assistance = page.locator('.et-feature-panel');
  await expect(assistance).toBeVisible();
  await assistance.locator('.et-feature-panel__action').click();
  await expect(page).toHaveURL(/\/dashboard\/ai-chat$/);
});

test('manager primary action navigates to authorized user management @smoke @aurora @manager', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  const firstAction = page.locator('.et-action-grid--manager .et-action-card').first();
  await expect(firstAction).toBeVisible();
  await firstAction.click();
  await expect(page).toHaveURL(/\/dashboard\/users$/);
});

test('admin retains three localized knowledge-review stages @smoke @aurora @admin', async ({ page }) => {
  await signIn(page, 'e2e-admin@example.test');
  const steps = page.locator('.et-admin-process-steps > li');
  await expect(steps).toHaveCount(3);
});
