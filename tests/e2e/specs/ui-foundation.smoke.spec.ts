// @smoke @pr2 @accessibility — PR-2 design-system and shell acceptance.
import AxeBuilder from '@axe-core/playwright';
import { test, expect, type Page, type TestInfo } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';

async function useEnglish(page: Page): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
}

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

function navItemWithIcon(page: Page, icon: string) {
  return page.locator('.et-ui-sidebar-nav__item', {
    has: page.locator('span.material-icons-outlined', { hasText: new RegExp(`^${icon}$`) }),
  }).first();
}

async function attachViewportEvidence(page: Page, testInfo: TestInfo, name: string): Promise<void> {
  await testInfo.attach(name, {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
}

function durationToMs(value: string): number {
  const trimmed = value.trim();
  if (trimmed.endsWith('ms')) return Number.parseFloat(trimmed);
  if (trimmed.endsWith('s')) return Number.parseFloat(trimmed) * 1000;
  return Number.POSITIVE_INFINITY;
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('shared login fields, reveal control, recovery dialog and WCAG scan are accessible @smoke @pr2 @accessibility', async ({ page }) => {
  await useEnglish(page);
  await page.goto('/');

  const email = page.locator('input[type="email"]');
  const password = page.locator('input[name="password"]');
  for (const input of [email, password]) {
    const id = await input.getAttribute('id');
    expect(id, 'shared fields must generate a control ID').toBeTruthy();
    await expect(page.locator(`label[for="${id}"]`)).toBeVisible();
  }

  const passwordField = password.locator('xpath=ancestor::div[contains(@class,"et-ui-field")][1]');
  const reveal = passwordField.locator('.et-ui-input-action');
  await expect(reveal).toHaveAttribute('aria-label', /Show password|نمایش رمز عبور/);
  await reveal.click();
  await expect(passwordField.locator('input')).toHaveAttribute('type', 'text');
  await expect(reveal).toHaveAttribute('aria-label', /Hide password|پنهان کردن رمز عبور/);
  await reveal.click();
  await expect(passwordField.locator('input')).toHaveAttribute('type', 'password');

  await page.locator('.et-auth-help').click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute('aria-modal', 'true');
  expect(await dialog.getAttribute('aria-labelledby')).toMatch(/^et-dialog-title-/);
  await expect(dialog).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);

  const result = await new AxeBuilder({ page })
    .include('.et-auth-panel')
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
    .analyze();
  expect(
    result.violations,
    result.violations.map((violation) => `${violation.id}: ${violation.help}`).join('\n'),
  ).toEqual([]);
});

test('desktop section navigation survives back and refresh with one active destination @smoke @pr2', async ({ page }, testInfo) => {
  await useEnglish(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await signIn(page, 'e2e-manager-a@example.test');

  const users = navItemWithIcon(page, 'groups');
  const classes = navItemWithIcon(page, 'class');
  await users.click();
  await expect(page).toHaveURL(/\/dashboard\/users$/);
  await expect(users).toHaveAttribute('aria-current', 'page');

  await classes.click();
  await expect(page).toHaveURL(/\/dashboard\/classes$/);
  await page.goBack();
  await expect(page).toHaveURL(/\/dashboard\/users$/);
  await expect(navItemWithIcon(page, 'groups')).toHaveAttribute('aria-current', 'page');

  await page.reload();
  await expect(page).toHaveURL(/\/dashboard\/users$/);
  await expect(navItemWithIcon(page, 'groups')).toHaveAttribute('aria-current', 'page');

  const active = page.locator('.et-ui-sidebar-nav__item[aria-current="page"]');
  await expect(active).toHaveCount(1);
  await attachViewportEvidence(page, testInfo, 'pr2-desktop-ltr');
});

test('mobile drawer keeps navigation parity and keyboard focus lifecycle @smoke @pr2 @accessibility', async ({ page }, testInfo) => {
  await useEnglish(page);
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-manager-a@example.test');

  const menu = page.getByRole('button', { name: 'Open navigation' });
  await expect(menu).toBeVisible();
  const navCount = await page.locator('.et-ui-sidebar-nav__item').count();
  expect(navCount).toBeGreaterThan(1);

  await menu.click();
  const sidebar = page.locator('.et-sidebar');
  await expect(sidebar).toHaveClass(/et-sidebar--mobile-open/);
  await expect(sidebar.locator('.et-sidebar-mobile-close')).toBeFocused();

  await page.keyboard.press('Escape');
  await expect(sidebar).not.toHaveClass(/et-sidebar--mobile-open/);
  await expect(menu).toBeFocused();

  await menu.click();
  await navItemWithIcon(page, 'groups').click();
  await expect(page).toHaveURL(/\/dashboard\/users$/);
  await expect(sidebar).not.toHaveClass(/et-sidebar--mobile-open/);
  await expect(page.locator('#dashboard-main-content')).toBeFocused();
  expect(await page.locator('.et-ui-sidebar-nav__item').count()).toBe(navCount);

  await attachViewportEvidence(page, testInfo, 'pr2-mobile-ltr');
});

test('direct cross-role section URL is denied without changing the requested URL @smoke @pr2', async ({ page }) => {
  await useEnglish(page);
  await signIn(page, 'e2e-teacher-a@example.test');

  await page.goto('/dashboard/users');
  await expect(page).toHaveURL(/\/dashboard\/users$/);
  const state = page.locator('.et-ui-data-state');
  await expect(state).toContainText(/access denied/i);
  await expect(page.locator('.et-ui-sidebar-nav__item')).toHaveCount(0);
});

test('notification popover closes with Escape and shell passes focused WCAG scan @smoke @pr2 @accessibility', async ({ page }) => {
  await useEnglish(page);
  await signIn(page, 'e2e-manager-a@example.test');

  const trigger = page.locator('.et-notification-trigger button');
  await trigger.click();
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
  await expect(page.locator('.et-notification-panel')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.locator('.et-notification-panel')).toHaveCount(0);
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');

  const result = await new AxeBuilder({ page })
    .include('.et-topbar')
    .include('.et-sidebar')
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
    .analyze();
  expect(
    result.violations,
    result.violations.map((violation) => `${violation.id}: ${violation.help}`).join('\n'),
  ).toEqual([]);
});

test('Farsi shell is RTL and reduced-motion preference suppresses nonessential transitions @smoke @pr2 @rtl @accessibility', async ({ page }, testInfo) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await signIn(page, 'e2e-manager-a@example.test');

  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^fa/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  const transitionDurations = await page.locator('.et-ui-sidebar-nav__item').first().evaluate((element) =>
    getComputedStyle(element).transitionDuration.split(',').map((part) => part.trim()),
  );
  expect(
    transitionDurations.every((duration) => durationToMs(duration) <= 0.1),
    `reduced-motion transition durations: ${transitionDurations.join(', ')}`,
  ).toBeTruthy();

  await attachViewportEvidence(page, testInfo, 'pr2-desktop-rtl');
});
