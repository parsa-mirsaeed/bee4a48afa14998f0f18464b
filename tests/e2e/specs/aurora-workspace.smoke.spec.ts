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
  const confirmDelete = confirmation.getByRole('button', { name: /delete/i });
  await expect(confirmDelete).toBeVisible();

  // In dark mode --et-color-danger is light pink, so white foreground is
  // unreadable. Exercise the live CSS cascade rather than checking a class.
  await page.locator('html').evaluate((html) => html.classList.add('dark'));
  const contrast = await confirmDelete.evaluate((button) => {
    const style = getComputedStyle(button);
    const rgb = (value: string) => {
      const channels = value.match(/[\\d.]+/g)?.slice(0, 3).map(Number) ?? [];
      if (channels.length !== 3) throw new Error(`Cannot parse CSS color ${value}`);
      return channels.map((v) => {
        const x = v / 255;
        return x <= 0.04045 ? x / 12.92 : Math.pow((x + 0.055) / 1.055, 2.4);
      });
    };
    const lum = (value: string) => {
      const [r, g, b] = rgb(value);
      return r * 0.2126 + g * 0.7152 + b * 0.0722;
    };
    const foreground = lum(style.color);
    const background = lum(style.backgroundColor);
    return (Math.max(foreground, background) + 0.05) /
      (Math.min(foreground, background) + 0.05);
  });
  expect(contrast, 'dark-mode destructive confirmation should meet 4.5:1 text contrast').toBeGreaterThanOrEqual(4.5);
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

  const refresh = admin.locator(':scope > div').first().getByRole('button', { name: 'Refresh', exact: true });
  await refresh.click();
  await expect(confirmation).toBeVisible();
  await expect(confirmation).toContainText('new jobs');

  await confirmation.press('Escape');
  await expect(confirmation).toHaveCount(0);
});

test('Aurora tablet in Persian dark mode retains navigation focus, contrast tokens and no sideways page scroll @smoke @aurora @rtl @dark @responsive', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 820, height: 1024 });
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'fa'));
  await signIn(page, 'e2e-manager-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');

  await page.locator('html').evaluate((element) => element.classList.add('dark'));
  await expect(page.locator('.et-dashboard-main')).toHaveCSS('background-color', 'rgb(16, 19, 38)');
  await expect(page.locator('.et-topbar')).toHaveCSS('background-color', 'rgb(27, 32, 57)');
  const menu = page.locator('.et-mobile-menu-button');
  await expect(menu).toBeVisible();
  await menu.click();
  const close = page.locator('.et-sidebar-mobile-close');
  await expect(close).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(menu).toBeFocused();
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1)).toBeFalsy();

  await testInfo.attach('aurora-tablet-dark-fa', {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});

test('manager account deactivation is confirmed and Cancel leaves access unchanged @smoke @aurora @manager @safety', async ({ page }) => {
  await signIn(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/users');
  const directory = page.locator('.et-manager-users__table');
  const row = directory.locator('tbody tr').filter({ hasText: 'E2E Teacher A' });
  await expect(row).toBeVisible();
  const originalStatus = await row.locator('td').nth(2).innerText();
  const trigger = row.locator('.et-manager-users__row-action[aria-expanded]');
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await trigger.click();

  const confirmation = row.locator('.et-manager-users__deactivation-prompt');
  await expect(confirmation).toContainText('E2E Teacher A');
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
  await confirmation.getByRole('button', { name: 'Cancel' }).click();
  await expect(confirmation).toHaveCount(0);
  await expect(trigger).toHaveAttribute('aria-expanded', 'false');
  await expect(row.locator('td').nth(2)).toHaveText(originalStatus);
});

test('Aurora sign-in matches the workspace on desktop and mobile without changing authentication @smoke @aurora @login @responsive', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await expect(page.locator('.et-auth-context')).toBeVisible();
  await expect(page.locator('.et-auth-context')).toHaveCSS('background-image', /linear-gradient/);
  await expect(page.locator('.et-auth-shell')).toHaveCSS('position', 'relative');
  await expect(page.locator('.et-auth-context')).toHaveCSS('position', 'absolute');
  await expect(page.locator('.et-auth-context__story')).toHaveCSS('position', 'absolute');
  await expect(page.locator('.et-auth-panel')).toHaveCSS('position', 'relative');
  await expect(page.locator('.et-auth-form-wrap')).toHaveCSS('border-radius', '24px');
  // Catch the regression where the SSR Persian grid positioned the brand
  // panel across the hydrated English form. An element can look visible yet
  // remain unclickable because a sibling intercepts pointer events.
  const desktopHitTest = await page.evaluate(() => {
    const context = document.querySelector<HTMLElement>('.et-auth-context')!;
    const panel = document.querySelector<HTMLElement>('.et-auth-panel')!;
    const button = document.querySelector<HTMLButtonElement>('.et-auth-form button[type="submit"]')!;
    const contextBox = context.getBoundingClientRect();
    const panelBox = panel.getBoundingClientRect();
    const buttonBox = button.getBoundingClientRect();
    const target = document.elementFromPoint(
      buttonBox.left + buttonBox.width / 2,
      buttonBox.top + buttonBox.height / 2,
    );
    return {
      columnsSeparate: contextBox.right <= panelBox.left + 1,
      buttonClickable: target === button || button.contains(target),
      brandPanelWithinViewport: contextBox.left >= -1 &&
        contextBox.right <= window.innerWidth + 1,
      brandInkWithinViewport: [...context.querySelectorAll<HTMLElement>(
        '.et-auth-context__brand, .et-auth-context__headline, .et-auth-context__copy, .et-auth-context__footnote',
      )].every((node) => {
        const range = document.createRange();
        range.selectNodeContents(node);
        const lines = [...range.getClientRects()];
        return lines.length > 0 && lines.every((line) =>
          line.left >= 1 &&
          line.right <= contextBox.right - 1 &&
          line.width > 0
        );
      }),
      brandPosition: { left: contextBox.left, right: contextBox.right },
      brandContentBounded: [...context.querySelectorAll<HTMLElement>(
        '.et-auth-context__brand, .et-auth-context__headline, .et-auth-context__copy',
      )].every((node) => {
        const bounds = node.getBoundingClientRect();
        return bounds.left >= contextBox.left - 2 &&
          bounds.right <= contextBox.right + 2 &&
          node.scrollWidth <= node.clientWidth + 2;
      }),
    };
  });
  expect(desktopHitTest.columnsSeparate, 'desktop brand and form columns must not overlap').toBeTruthy();
  expect(desktopHitTest.buttonClickable, 'the sign-in button must receive pointer events').toBeTruthy();
  expect(desktopHitTest.brandContentBounded, 'desktop headline and brand boxes must fit inside their panel').toBeTruthy();
  expect(desktopHitTest.brandPanelWithinViewport, JSON.stringify(desktopHitTest.brandPosition)).toBeTruthy();
  expect(desktopHitTest.brandInkWithinViewport, 'the rendered brand text must not be cropped at the left screen edge').toBeTruthy();
  await expect(page.locator('input[type="email"]')).toBeVisible();
  await expect(page.locator('input[type="password"]')).toBeVisible();
  await testInfo.attach('aurora-login-desktop', {
    body: await page.screenshot({ fullPage: false }),
    contentType: 'image/png',
  });

  await page.setViewportSize({ width: 1024, height: 768 });
  await expect(page.locator('.et-auth-context')).toBeHidden();
  await expect(page.locator('.et-auth-panel')).toBeVisible();
  await expect(page.locator('.et-auth-form-wrap')).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)).toBeFalsy();

  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator('.et-auth-context')).toBeHidden();
  await expect(page.locator('.et-auth-mobile-brand')).toBeVisible();
  await expect(page.locator('.et-auth-form-wrap')).toHaveCSS('border-radius', '20px');
  expect(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1)).toBeFalsy();

  await page.locator('.et-auth-panel .language-switcher-toggle').click();
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');
  await expect(page.locator('.et-auth-form-wrap')).toBeVisible();

  const rtlPasswordIcons = await page.evaluate(() => {
    const input = document.querySelector<HTMLInputElement>('.et-auth-form input[type="password"]')!;
    const control = input.closest('.et-ui-control')!;
    const leading = control.querySelector<HTMLElement>('.et-ui-control__leading')!;
    const toggle = control.querySelector<HTMLButtonElement>('.et-ui-input-action')!;
    const fieldRect = input.getBoundingClientRect();
    const lockRect = leading.getBoundingClientRect();
    const toggleRect = toggle.getBoundingClientRect();
    const hit = document.elementFromPoint(
      toggleRect.left + toggleRect.width / 2,
      toggleRect.top + toggleRect.height / 2,
    );
    return {
      leadingOnRight: lockRect.left >= fieldRect.left + fieldRect.width * 0.55,
      toggleOnLeft: toggleRect.right <= fieldRect.left + fieldRect.width * 0.45,
      iconsDoNotOverlap: toggleRect.right + 8 < lockRect.left,
      revealClickable: hit === toggle || toggle.contains(hit),
      revealTargetHeight: toggleRect.height,
    };
  });
  expect(rtlPasswordIcons.leadingOnRight, 'RTL lock icon belongs at the right of the password field').toBeTruthy();
  expect(rtlPasswordIcons.toggleOnLeft, 'RTL reveal action belongs at the left of the password field').toBeTruthy();
  expect(rtlPasswordIcons.iconsDoNotOverlap, 'RTL reveal action and lock icon must not overlap').toBeTruthy();
  expect(rtlPasswordIcons.revealClickable, 'RTL reveal action must receive pointer events').toBeTruthy();
  expect(rtlPasswordIcons.revealTargetHeight).toBeGreaterThanOrEqual(44);
  await testInfo.attach('aurora-login-mobile-fa', {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});

test('Aurora login remains clickable in Persian RTL desktop layout @smoke @aurora @login @rtl @accessibility', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'fa'));
  await page.goto('/');
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');
  const signIn = page.locator('.et-auth-form button[type="submit"]');
  await expect(signIn).toBeVisible();

  const clickable = await signIn.evaluate((button) => {
    const rectangle = button.getBoundingClientRect();
    const hit = document.elementFromPoint(
      rectangle.left + rectangle.width / 2,
      rectangle.top + rectangle.height / 2,
    );
    return hit === button || button.contains(hit);
  });
  expect(clickable, 'RTL sign-in control must not be under the desktop brand panel').toBeTruthy();
  const brandFits = await page.locator('.et-auth-context').evaluate((aside) => {
    const bounds = aside.getBoundingClientRect();
    return [...aside.querySelectorAll<HTMLElement>(
      '.et-auth-context__brand, .et-auth-context__headline, .et-auth-context__copy',
    )].every((node) => {
      const rect = node.getBoundingClientRect();
      return rect.left >= bounds.left - 2 && rect.right <= bounds.right + 2 &&
        node.scrollWidth <= node.clientWidth + 2;
    });
  });
  expect(brandFits, 'RTL sign-in branding must not be cropped').toBeTruthy();
  const rtlBrandInkFits = await page.locator('.et-auth-context').evaluate((aside) => {
    const bounds = aside.getBoundingClientRect();
    return bounds.left >= -1 && [...aside.querySelectorAll<HTMLElement>(
      '.et-auth-context__brand, .et-auth-context__headline, .et-auth-context__copy, .et-auth-context__footnote',
    )].every((node) => {
      const range = document.createRange();
      range.selectNodeContents(node);
      const lines = [...range.getClientRects()];
      return lines.length > 0 && lines.every((line) =>
        line.left >= 1 && line.right <= bounds.right - 1 && line.width > 0
      );
    });
  });
  expect(rtlBrandInkFits, 'RTL rendered brand text must be fully inside the desktop panel').toBeTruthy();
  await page.locator('input[type="email"]').fill('e2e-manager-a@example.test');
  await page.locator('input[type="password"]').fill('e2e-password');
  await signIn.click();
  await expect(page).toHaveURL(/\/dashboard$/);
});

test('teacher assignment filters expose exactly one current choice @smoke @aurora @teacher @student @accessibility', async ({ page }) => {
  await signIn(page, 'e2e-teacher-a@example.test');
  await page.goto('/dashboard/assignments');
  const teacherFilters = page.locator('.et-teacher-assignments .et-assignment-filters');
  await expect(teacherFilters).toHaveAttribute('role', 'group');
  await expect(teacherFilters.getByRole('button')).toHaveCount(4);
  await expect(teacherFilters.locator('button[aria-pressed="true"]')).toHaveCount(1);
  await teacherFilters.getByRole('button').nth(1).click();
  await expect(teacherFilters.getByRole('button').nth(1)).toHaveAttribute('aria-pressed', 'true');


});

test('student assignment filter group and cards remain usable at mobile width @smoke @aurora @student @responsive', async ({ page }, testInfo: TestInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page, 'e2e-student-a@example.test');
  await page.goto('/dashboard/assignments');
  const filters = page.locator('.et-student-assignments .et-assignment-filters');
  await expect(filters).toHaveAttribute('role', 'group');
  await expect(filters.getByRole('button')).toHaveCount(5);
  await expect(filters.locator('button[aria-pressed="true"]')).toHaveCount(1);
  await expect(page.locator('.et-student-assignment-card').first()).toBeVisible();
  await filters.getByRole('button').nth(1).click();
  await expect(filters.getByRole('button').nth(1)).toHaveAttribute('aria-pressed', 'true');
  expect(await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1)).toBeFalsy();
  await testInfo.attach('aurora-student-assignments-mobile', {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});

test('parent academic guidance has accessible scoped controls, consent remains required @smoke @aurora @parent @accessibility', async ({ page }, testInfo: TestInfo) => {
  await signIn(page, 'e2e-parent-a@example.test');
  await page.goto('/dashboard/ai-chat');
  const workspace = page.locator('.et-parent-chat__workspace');
  await expect(workspace).toBeVisible();
  const childPicker = workspace.locator('select');
  await expect(childPicker).toBeVisible();
  await childPicker.selectOption({ index: 1 });
  const suggestions = workspace.locator('.et-parent-chat__suggestions');
  await expect(suggestions.getByRole('button')).toHaveCount(2);
  // An empty log is intentionally zero-height; the visible status explains
  // that no child-scoped answers have been generated yet.
  const history = workspace.locator('[role="log"]');
  await expect(history).toBeAttached();
  await expect(history).toHaveAttribute('aria-label', /.+/);
  await expect(workspace.locator('.et-parent-chat__empty')).toBeVisible();
  await expect(workspace.locator('textarea')).toBeVisible();
  const send = workspace.getByRole('button').last();
  await expect(send).toBeDisabled();
  await testInfo.attach('aurora-parent-chat', {
    body: await page.screenshot({ fullPage: true }),
    contentType: 'image/png',
  });
});

test('teacher create assignment fields are explicitly labeled, required, and keyboard focusable @smoke @aurora @teacher @forms @accessibility', async ({ page }) => {
  await signIn(page, 'e2e-teacher-a@example.test');
  await page.goto('/dashboard/assignments');
  await page.locator('.et-teacher-assignments button.btn-primary').first().click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const fields = [
    ['et-teacher-assignment-title', 'input'],
    ['et-teacher-assignment-class', 'select'],
    ['et-teacher-assignment-due', 'input'],
    ['et-teacher-assignment-instructions', 'textarea'],
  ] as const;
  for (const [id, element] of fields) {
    const control = dialog.locator(`${element}#${id}`);
    await expect(control).toBeVisible();
    await expect(dialog.locator(`label[for="${id}"]`)).toBeVisible();
    await expect(control).toHaveAttribute('aria-required', 'true');
    await control.focus();
    await expect(control).toBeFocused();
  }
});
