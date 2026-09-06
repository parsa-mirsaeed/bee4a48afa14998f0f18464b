// @smoke @final @auth — credentials cannot be lost before client hydration.
import { test, expect } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

for (const locale of ['en', 'fa']) {
  test(`login protects credentials until hydration in ${locale} @smoke @final @auth`, async ({ page }) => {
    await page.addInitScript(value => localStorage.setItem('edutalent_locale', value), locale);
    let releaseWasm!: () => void;
    const hydrationBarrier = new Promise<void>(resolve => { releaseWasm = resolve; });
    await page.route(/\.wasm(?:\?.*)?$/, async route => {
      await hydrationBarrier;
      await route.fallback();
    });
    try {
      await page.goto('/', { waitUntil: 'domcontentloaded' });
      const form = page.locator('form.et-auth-form');
      const email = page.getByLabel(/^(Email Address|آدرس ایمیل|ایمیل)(\s*\*)?$/i);
      const password = page.getByLabel(/^(Password|رمز عبور)(\s*\*)?$/i);
      const submit = page.getByRole('button', { name: /^(Sign In|ورود)$/i });
      await expect(form).toHaveAttribute('aria-busy', 'true');
      await expect(email).toBeDisabled();
      await expect(password).toBeDisabled();
      await expect(submit).toBeDisabled();

      releaseWasm();
      await expect(form).toHaveAttribute('aria-busy', 'false');
      await expect(email).toBeEnabled();
      await expect(password).toBeEnabled();
      await expect(submit).toBeEnabled();
      await email.fill('e2e-teacher-a@example.test');
      await password.fill('e2e-password');
      const login = page.waitForResponse(response =>
        response.url().endsWith('/api/auth/login') && response.request().method() === 'POST');
      await submit.click();
      expect((await login).ok()).toBeTruthy();
      await expect(page).toHaveURL(/\/dashboard$/);
    } finally {
      releaseWasm();
    }
  });
}
