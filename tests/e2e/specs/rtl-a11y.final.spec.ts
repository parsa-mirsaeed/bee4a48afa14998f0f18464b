// @final @rtl @accessibility — Tier-2 layout and accessibility evidence.
import AxeBuilder from '@axe-core/playwright';
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

async function signInEnglish(page: Page, email: string): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
  await signIn(page, email);
}

async function navigateWithIcon(page: Page, icon: string): Promise<void> {
  const mobileMenu = page.locator('.et-mobile-menu-button');
  if (await mobileMenu.isVisible()) {
    await mobileMenu.click();
    await expect(page.locator('.et-sidebar')).toHaveClass(/et-sidebar--mobile-open/);
  }

  const item = page.locator('.et-ui-sidebar-nav__item', {
    has: page.locator('span.material-icons-outlined', {
      hasText: new RegExp(`^${icon}$`),
    }),
  }).first();
  await expect(item).toBeVisible();
  await item.click();
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('Farsi and English document direction are coherent @final @rtl', async ({ page }) => {
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^fa/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  await page.locator('.language-switcher-toggle').click();
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^en/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');
});

test('Persian grade dates and numbers are isolated LTR inside the RTL document @final @rtl', async ({ page }) => {
  await signIn(page, 'e2e-student-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  await navigateWithIcon(page, 'grade');
  const classCard = page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"et-ui-card")][1]');
  await classCard.getByRole('button').click();

  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const seededGradeRow = dialog
    .getByText('E2E Assignment A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"rounded-lg")][1]');
  const isolatedValues = seededGradeRow.locator('bdi[dir="ltr"]');
  await expect(isolatedValues).toHaveCount(3);
  for (let index = 0; index < 3; index += 1) {
    await expect(isolatedValues.nth(index)).toHaveCSS('direction', 'ltr');
  }
});

test('shared grading modal exposes generated dialog semantics and keyboard focus entry @final @accessibility', async ({ page }) => {
  await signInEnglish(page, 'e2e-teacher-a@example.test');
  await navigateWithIcon(page, 'grading');

  const assignmentTitle = page.getByText('E2E Authorization Submission A', { exact: true });
  await expect(assignmentTitle).toBeVisible();
  await expect(page.getByText('submissions.grade_btn', { exact: true })).toHaveCount(0);
  await expect(page.getByText('submissions.review_description', { exact: true })).toHaveCount(0);

  const submissionCard = assignmentTitle.locator(
    'xpath=ancestor::div[contains(@class,"rounded-xl")][.//button[.//span[normalize-space()="grading"]]][1]',
  );
  const gradeButton = submissionCard.locator('xpath=.//button[.//span[normalize-space()="grading"]]');
  await expect(gradeButton).toContainText('Grade Submission');
  await gradeButton.click();

  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAttribute('aria-modal', 'true');
  const labelledBy = await dialog.getAttribute('aria-labelledby');
  expect(labelledBy).toMatch(/^et-dialog-title-/);
  await expect(dialog.locator(`#${labelledBy}`)).toContainText('Grade Submission');
  await expect(dialog).toContainText('Grade (0-100)');
  await expect(dialog).toBeFocused();

  await page.keyboard.press('Tab');
  const closeButton = dialog.getByRole('button', { name: 'Close', exact: true });
  await expect(closeButton).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(dialog.locator('input[type="number"]')).toBeFocused();
  await page.keyboard.press('Shift+Tab');
  await expect(closeButton).toBeFocused();
});

test('login has no automated WCAG A/AA violations @final @accessibility', async ({ page }) => {
  await page.goto('/');
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
    .analyze();
  expect(
    result.violations,
    result.violations.map((violation) => `${violation.id}: ${violation.help}`).join('\n'),
  ).toEqual([]);
});

test('login form controls follow the visible keyboard order @final @accessibility', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
  });

  const visited: string[] = [];
  for (let step = 0; step < 18; step += 1) {
    await page.keyboard.press('Tab');
    visited.push(
      await page.evaluate(() => {
        const element = document.activeElement;
        if (!(element instanceof HTMLElement)) return '';
        if (element instanceof HTMLInputElement) return `input:${element.type}`;
        if (element instanceof HTMLAnchorElement) return `href:${element.getAttribute('href') ?? ''}`;
        if (element instanceof HTMLButtonElement) return `button:${element.type}`;
        return element.tagName.toLowerCase();
      }),
    );
  }

  const emailIndex = visited.indexOf('input:email');
  const passwordIndex = visited.indexOf('input:password');
  const submitIndex = visited.indexOf('button:submit');

  expect(emailIndex, `email not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThanOrEqual(0);
  expect(passwordIndex, `password not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(emailIndex);
  expect(submitIndex, `submit not keyboard reachable: ${visited.join(' -> ')}`).toBeGreaterThan(passwordIndex);
});
