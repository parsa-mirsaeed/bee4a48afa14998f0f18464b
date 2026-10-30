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
  const close = sidebar.locator('.et-sidebar-mobile-close');
  await expect(close, 'Mounted drawer close control must capture focus even with reduced motion').toBeFocused();
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

test('manager directory has accessible filters, semantic table headings, and 44px row actions @smoke @aurora @manager @accessibility', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/users');

  const filters = page.locator('.et-manager-users__filters');
  const selects = filters.locator('select');
  await expect(selects).toHaveCount(2);
  for (const select of await selects.all()) {
    await expect(select).toHaveAttribute('aria-label', /.+/);
  }
  await expect(filters.locator('input')).toHaveAttribute('aria-label', /.+/);

  const directory = page.locator('.et-manager-users__table');
  await expect(page.getByText('E2E Teacher A', { exact: true })).toBeVisible();
  await expect(directory.locator('thead th[scope="col"]')).toHaveCount(5);
  const tabs = page.locator('.et-manager-users__tabs');
  await expect(tabs.locator('button[aria-pressed="true"]')).toHaveCount(1);

  const editAction = directory.locator('.et-manager-users__row-action').first();
  await expect(editAction).toHaveAttribute('aria-label', /.+/);
  const box = await editAction.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThanOrEqual(44);
  expect(box!.height).toBeGreaterThanOrEqual(44);
});

test('manager mobile directory scrolls horizontally within its panel, not the page @smoke @aurora @manager @responsive', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/users');
  await expect(page.getByText('E2E Teacher A', { exact: true })).toBeVisible();
  const scroll = page.locator('.et-manager-users__table-scroll');
  await expect(scroll).toBeVisible();
  expect(await scroll.evaluate((element) => element.scrollWidth > element.clientWidth)).toBeTruthy();
  await scroll.evaluate((element) => element.scrollTo({ left: element.scrollWidth }));
  const pageOverflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
  expect(pageOverflow, 'wide directory table should not force document horizontal scroll').toBeFalsy();
});

test('Persian manager directory uses RTL-aware filter and search affordances @smoke @aurora @manager @rtl', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'fa'));
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/users');
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');
  await expect(page.getByText('E2E Teacher A', { exact: true })).toBeVisible();

  const firstFilterIcon = page.locator('.et-manager-users__filter > .material-icons-outlined').first();
  const searchIcon = page.locator('.et-manager-users__search > .material-icons-outlined');
  await expect(firstFilterIcon).toHaveCSS('left', '12px');
  await expect(searchIcon).toHaveCSS('right', '12px');
  await expect(page.locator('.et-manager-users__tabs button[aria-pressed="true"]')).toHaveCount(1);
});

test('manager overview explains actual work rather than release internals in EN and FA @smoke @aurora @i18n', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  const hero = page.locator('.et-overview-intro');
  await expect(hero).toContainText('Manage your school community, classrooms, and learning resources in one place.');
  await expect(page.locator('body')).not.toContainText('production-backed workflows');
  await expect(page.locator('body')).not.toContainText('Synthetic activity');

  // Exercise the visible language switcher. The suite's addInitScript pins
  // English at navigation time, so a storage-write-plus-reload cannot switch
  // this test to Persian and would hide a real translation regression.
  const languageSwitcher = page.locator('.et-sidebar .language-switcher-toggle');
  await expect(languageSwitcher).toBeVisible();
  await languageSwitcher.click();
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');
  await expect(hero).toContainText('مدیریت کاربران، کلاس‌ها و منابع آموزشی مدرسه');
  await expect(page.locator('body')).not.toContainText('شاخص‌های ساختگی');
});

test('school classes inventory cards open details with Enter and Space @smoke @aurora @manager @accessibility', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/classes');
  const card = page.locator('.et-manager-classes__card').filter({ hasText: 'E2E Class A1' }).first();
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('role', 'button');
  await expect(card).toHaveAttribute('tabindex', '0');
  await card.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);

  await card.focus();
  await page.keyboard.press('Space');
  await expect(page.getByRole('dialog')).toBeVisible();
});

test('school classes switch view accessibly and label the subject selector @smoke @aurora @manager @forms', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/classes');
  const view = page.locator('.et-manager-classes__view-switch');
  await expect(view).toHaveAttribute('role', 'group');
  const buttons = view.getByRole('button');
  await expect(buttons).toHaveCount(2);
  await expect(buttons.nth(0)).toHaveAttribute('aria-pressed', 'true');
  await buttons.nth(1).click();
  await expect(buttons.nth(1)).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('.et-manager-classes__list')).toBeVisible();

  await page.locator('.et-manager-classes__toolbar').getByRole('button').last().click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const subject = dialog.locator('#et-create-class-subject');
  await expect(subject).toBeVisible();
  await expect(subject).toHaveAttribute('aria-required', 'true');
  await expect(dialog.locator('label[for="et-create-class-subject"]')).toBeVisible();
});

test('class enrollment controls expose a student label and safe touch targets @smoke @aurora @manager @accessibility', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/classes');
  const card = page.locator('.et-manager-classes__card').filter({ hasText: 'E2E Class A1' }).first();
  await expect(card).toBeVisible();
  await card.click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const enrollment = dialog.locator('#et-class-enroll-student');
  await expect(enrollment).toHaveAttribute('aria-required', 'true');
  await expect(dialog.locator('label[for="et-class-enroll-student"]')).toBeVisible();
  const remove = dialog.locator('.et-manager-classes__remove-action').first();
  await expect(remove).toBeVisible();
  await expect(remove).toHaveAttribute('aria-label', /.+/);
  const box = await remove.boundingBox();
  expect(box).not.toBeNull();
  expect(box!.width).toBeGreaterThanOrEqual(44);
  expect(box!.height).toBeGreaterThanOrEqual(44);
});

test('roster removal requires confirmation and Cancel preserves enrollment @smoke @aurora @manager @safety', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/classes');
  const card = page.locator('.et-manager-classes__card').filter({ hasText: 'E2E Class A1' }).first();
  await expect(card).toBeVisible();
  await card.click();

  const dialog = page.getByRole('dialog');
  const roster = dialog.locator('.et-manager-classes__roster');
  const rows = roster.locator('.et-manager-classes__roster-row');
  await expect(rows.first()).toBeVisible();
  const originalCount = await rows.count();
  const row = rows.first();
  const originalStudent = await row.locator('p').first().innerText();
  const remove = row.locator('.et-manager-classes__remove-action');

  await remove.click();
  await expect(remove).toHaveAttribute('aria-expanded', 'true');
  const confirmation = row.locator('.et-manager-classes__remove-confirmation');
  await expect(confirmation).toContainText(originalStudent);
  await expect(confirmation.getByRole('button', { name: /delete/i })).toBeVisible();
  await confirmation.getByRole('button', { name: 'Cancel' }).click();
  await expect(confirmation).toHaveCount(0);
  await expect(remove).toHaveAttribute('aria-expanded', 'false');
  await expect(rows).toHaveCount(originalCount);
  await expect(row).toContainText(originalStudent);
});

test('admin regeneration confirmation survives an overview data refresh and closes with Escape @smoke @aurora @admin @accessibility', async ({ page }) => {
  await signIn(page, 'e2e-admin@example.test');
  await page.goto('/dashboard/personalization');
  const admin = page.locator('.et-admin-personalization');
  await expect(admin).toBeVisible();
  const card = admin.locator('.et-admin-policy-card', { hasText: 'E2E School A' });
  await expect(card).toBeVisible();

  const regenerate = card.getByRole('button', { name: 'Retry with current AI settings', exact: true });
  await regenerate.click();
  const confirmation = card.getByRole('alertdialog');
  await expect(confirmation).toBeVisible();
  await expect(confirmation).toBeFocused();
  await expect(confirmation).toContainText('new jobs');

  const refresh = admin.locator('> div').first().getByRole('button', { name: 'Refresh', exact: true });
  await refresh.click();
  await expect(confirmation).toBeVisible();
  await expect(confirmation).toContainText('new jobs');

  await confirmation.press('Escape');
  await expect(confirmation).toHaveCount(0);
});
