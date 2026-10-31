// @smoke @navigation @accessibility @responsive — real mobile role navigation.
// Uses existing offline school fixtures, no mocked permissions or synthetic routes.
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

const mobileNav = (page: Page) => page.getByRole('navigation', { name: 'Mobile navigation' });

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('student bottom navigation follows routes and Back with one current item @smoke @navigation @student', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-student-a@example.test');
  const bar = mobileNav(page);
  await expect(bar).toBeVisible();
  await expect(bar.getByRole('button')).toHaveCount(4);
  await expect(bar.locator('[aria-current="page"]')).toHaveCount(1);

  const assignments = bar.getByRole('button', { name: 'Assignments', exact: true });
  await assignments.click();
  await expect(page).toHaveURL(/\/dashboard\/assignments$/);
  await expect(assignments).toHaveAttribute('aria-current', 'page');
  await expect(bar.locator('[aria-current="page"]')).toHaveCount(1);
  await page.goBack();
  await expect(page).toHaveURL(/\/dashboard$/);
  await expect(bar.locator('[aria-current="page"]')).toHaveCount(1);

  await testInfo.attach('mobile-bottom-nav-student-en', {
    body: await page.screenshot({ fullPage: true }), contentType: 'image/png',
  });
});

test('teacher primary routes are scoped and secondary screens remain in drawer @smoke @navigation @teacher', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 812 });
  await signIn(page, 'e2e-teacher-a@example.test');
  const bar = mobileNav(page);
  await expect(bar.getByRole('button')).toHaveCount(4);
  const grading = bar.getByRole('button', { name: 'Grading', exact: true });
  await grading.click();
  await expect(page).toHaveURL(/\/dashboard\/submissions$/);
  await expect(grading).toHaveAttribute('aria-current', 'page');
  await expect(bar.getByRole('button', { name: 'Children' })).toHaveCount(0);
  await expect(bar.getByRole('button', { name: 'Users' })).toHaveCount(0);

  const drawerToggle = page.getByRole('button', { name: 'Open navigation' });
  await drawerToggle.click();
  await expect(bar).toBeHidden();
  await expect(page.locator('.et-sidebar--mobile-open')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(bar).toBeVisible();
  await expect(drawerToggle).toBeFocused();
});

test('parent shortcuts are scoped, while unavailable reports never appear @smoke @navigation @parent', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-parent-a@example.test');
  const bar = mobileNav(page);
  await expect(bar.getByRole('button')).toHaveCount(3);
  await expect(bar.getByRole('button', { name: 'Reports' })).toHaveCount(0);
  await bar.getByRole('button', { name: 'Children' }).click();
  await expect(page).toHaveURL(/\/dashboard\/children$/);
  await expect(bar.getByRole('button', { name: 'Children' })).toHaveAttribute('aria-current', 'page');
});

test('manager and admin use authorized mobile destinations @smoke @navigation @manager @admin', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-manager-a@example.test');
  let bar = mobileNav(page);
  await expect(bar.getByRole('button')).toHaveCount(4);
  await bar.getByRole('button', { name: 'User Management' }).click();
  await expect(page).toHaveURL(/\/dashboard\/users$/);

  await page.goto('/login');
  await page.locator('input[type="email"]').fill('e2e-admin@example.test');
  await page.locator('input[type="password"]').fill('e2e-password');
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
  bar = mobileNav(page);
  await expect(bar.getByRole('button')).toHaveCount(3);
  await expect(bar.locator('[aria-current="page"]')).toHaveCount(1);
  await expect(bar.getByRole('button', { name: 'User Management' })).toHaveCount(0);
});

test('mobile bottom navigation fits 320 to 430px, hides at tablet boundary @smoke @navigation @responsive', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 720 });
  await signIn(page, 'e2e-student-a@example.test');
  for (const width of [320, 375, 390, 430]) {
    await page.setViewportSize({ width, height: 844 });
    const bar = mobileNav(page);
    await expect(bar).toBeVisible();
    const bounds = await bar.boundingBox();
    expect(bounds).not.toBeNull();
    expect(bounds!.left).toBeGreaterThanOrEqual(0);
    expect(bounds!.right).toBeLessThanOrEqual(width + 1);
    for (const item of await bar.getByRole('button').all()) {
      const box = await item.boundingBox();
      expect(box).not.toBeNull();
      expect(box!.width).toBeGreaterThanOrEqual(44);
      expect(box!.height).toBeGreaterThanOrEqual(44);
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1)).toBeFalsy();
    const gap = await page.locator('.et-dashboard-content-inner')
      .evaluate(el => parseFloat(getComputedStyle(el).paddingBottom));
    expect(gap).toBeGreaterThan(120);
  }
  await page.setViewportSize({ width: 768, height: 1024 });
  await expect(mobileNav(page)).toBeHidden();
  await expect(page.getByRole('button', { name: 'Open navigation' })).toBeVisible();
});

test('Persian RTL dark mode, visible focus and reduced motion @smoke @navigation @rtl @accessibility', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'fa'));
  await signIn(page, 'e2e-student-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');
  const bar = page.getByRole('navigation', { name: 'ناوبری موبایل' });
  await expect(bar).toBeVisible();
  await page.locator('html').evaluate(el => el.classList.add('dark'));
  await expect(bar.getByRole('button')).toHaveCount(4);
  const item = bar.getByRole('button').nth(2);
  await item.focus();
  await expect(item).toBeFocused();
  await expect(item).toHaveCSS('transition-duration', '0s');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/dashboard\/assignments$/);
  await expect(bar.locator('[aria-current="page"]')).toHaveCount(1);
  await testInfo.attach('mobile-bottom-nav-student-fa-dark', {
    body: await page.screenshot({ fullPage: true }), contentType: 'image/png',
  });
});

test('short keyboard-height views hide bar so it cannot cover form actions @smoke @navigation @responsive', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-teacher-a@example.test');
  await expect(mobileNav(page)).toBeVisible();
  await page.setViewportSize({ width: 390, height: 440 });
  await expect(mobileNav(page)).toBeHidden();
});
