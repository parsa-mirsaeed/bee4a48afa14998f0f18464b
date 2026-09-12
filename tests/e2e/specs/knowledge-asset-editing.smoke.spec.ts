import AxeBuilder from '@axe-core/playwright';
// @smoke @workflow-truth — #34 focused browser evidence for lifecycle-aware
// knowledge asset editing/versioning. Uses the real Dioxus server functions,
// PostgreSQL RLS context, and local private-storage contract; application
// responses are not stubbed.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const FIXTURE_PASSWORD = 'e2e-password';
const MANAGER_EMAIL = 'e2e-manager-a@example.test';
const TABLET_VIEWPORT = { width: 1024, height: 768 };

async function signIn(page: Page, locale: 'en' | 'fa' = 'en'): Promise<void> {
  await page.addInitScript((value) => localStorage.setItem('edutalent_locale', value), locale);
  await page.goto('/');
  await page.locator('input[type="email"]').fill(MANAGER_EMAIL);
  await page.locator('input[type="password"]').fill(FIXTURE_PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

function actionWithIcon(page: Page, icon: string) {
  return page.locator('button', {
    has: page.locator('span.material-icons-outlined', {
      hasText: new RegExp(`^${icon}$`),
    }),
  }).first();
}

function minimalPdf(marker: string): Buffer {
  return Buffer.from(`%PDF-1.4\n% ${marker}\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n`);
}

async function openKnowledgeManager(page: Page): Promise<void> {
  await actionWithIcon(page, 'upload_file').click();
  await expect(page.getByText(/Private knowledge storage is ready/i)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Manage existing assets' })).toBeVisible();
}

async function openKnowledgeManagerResponsive(page: Page, locale: 'en' | 'fa'): Promise<void> {
  const navigationAction = page.getByRole('button', { name: locale === 'fa' ? 'ارسال منابع دانشی' : 'Knowledge submissions', exact: true });
  if (await page.locator('.et-mobile-menu-button').isVisible()) {
    await page.locator('.et-mobile-menu-button').click();
    await expect(navigationAction).toBeVisible();
  }
  await navigationAction.click();
  await expect(
    page.getByRole('heading', {
      name: locale === 'fa' ? 'ویرایش منابع دانشی' : 'Manage existing assets',
    }),
  ).toBeVisible();
}

async function openAssetEditor(page: Page, title: string) {
  const card = page
    .getByText(title, { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(card).toBeVisible();
  await card.getByRole('button', { name: 'Edit', exact: true }).click();
  const form = page
    .getByRole('heading', { name: 'Asset details', exact: true })
    .locator('xpath=ancestor::form[1]');
  await expect(form).toBeVisible();
  await expect(page.getByRole('dialog')).toBeFocused();
  return form;
}

async function openLocalizedFixtureEditor(page: Page, locale: 'en' | 'fa') {
  const card = page
    .getByText('E2E Published Asset', { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(card).toBeVisible();
  await card
    .getByRole('button', { name: locale === 'fa' ? 'ویرایش' : 'Edit', exact: true })
    .click();
  const form = page
    .getByRole('heading', {
      name: locale === 'fa' ? 'جزئیات منبع' : 'Asset details',
      exact: true,
    })
    .locator('xpath=ancestor::form[1]');
  await expect(form).toBeVisible();
  await expect(page.getByRole('dialog')).toBeFocused();
  return form;
}

async function assertLocaleDirection(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
    new RegExp(`^${locale}`, 'i'),
  );
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe(
    locale === 'fa' ? 'rtl' : 'ltr',
  );
}

async function assertNoHorizontalOverflow(page: Page): Promise<void> {
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth > document.documentElement.clientWidth,
  );
  expect(overflow, 'knowledge editor must not introduce page-level horizontal overflow').toBeFalsy();
}

async function assertEditorAccessibility(page: Page, locale: 'en' | 'fa'): Promise<void> {
  const dialog = page.getByRole('dialog');
  await expect(dialog).toHaveAccessibleName(new RegExp('E2E Published Asset'));
  const title = dialog.getByLabel(locale === 'fa' ? 'عنوان *' : 'Title *', { exact: true });
  await expect(title).toHaveValue('E2E Published Asset');
  const save = dialog.getByRole('button', { name: locale === 'fa' ? 'ذخیره' : 'Save', exact: true });
  await save.scrollIntoViewIfNeeded();
  await expect(save).toBeInViewport();
  const result = await new AxeBuilder({ page }).include('[role="dialog"]')
    .withTags(['wcag2a', 'wcag2aa', 'wcag21aa', 'wcag22aa']).analyze();
  expect(result.violations.filter(v => v.impact === 'serious' || v.impact === 'critical')).toEqual([]);
  await title.focus();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  const card = page.getByRole('article').filter({ has: page.getByRole('heading', { name: 'E2E Published Asset', exact: true }) });
  await expect(card.getByRole('button', { name: locale === 'fa' ? 'ویرایش' : 'Edit', exact: true })).toBeFocused();
}

test.beforeEach(async ({ page, request }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
  const ready = await request.post('http://127.0.0.1:9100/__e2e/storage-mode', {
    data: { mode: 'ready' },
  });
  expect(ready.ok()).toBeTruthy();
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('manager edits same asset with dirty-state guard and replaces immutable source @smoke @workflow-truth', async ({ page }) => {
  test.setTimeout(120_000);
  await signIn(page);
  await openKnowledgeManager(page);

  const unique = Date.now().toString();
  const title = `E2E Versioned Knowledge ${unique}`;
  const uploadForm = page.locator('#manager-knowledge-upload-form');
  await uploadForm.locator('input[name="title"]').fill(title);
  await uploadForm.locator('input[name="subject"]').fill('Mathematics');
  await uploadForm.locator('input[name="grade"]').fill('8');
  await uploadForm.locator('input[type="file"]').setInputFiles({
    name: `version-1-${unique}.pdf`,
    mimeType: 'application/pdf',
    buffer: minimalPdf(`version-1-${unique}`),
  });
  await uploadForm.getByRole('button', { name: 'Upload for review', exact: true }).click();
  await expect(page.getByRole('status')).toContainText(/uploaded and registered with status submitted/i);

  let metadataForm = await openAssetEditor(page, title);
  const description = metadataForm.getByLabel('Description', { exact: true });
  const cancel = metadataForm.getByRole('button', { name: 'Cancel', exact: true });
  const save = metadataForm.getByRole('button', { name: 'Save', exact: true });
  await expect(save).toBeDisabled();

  await description.fill('Unsaved presentation copy');
  await expect(metadataForm.getByText('Unsaved changes', { exact: true })).toBeVisible();
  await expect(save).toBeEnabled();

  const reloadPrompt = page.waitForEvent('dialog');
  await page.evaluate(() => setTimeout(() => location.reload(), 0));
  const unloadDialog = await reloadPrompt;
  expect(unloadDialog.type()).toBe('beforeunload');
  await unloadDialog.dismiss();
  await expect(description).toHaveValue('Unsaved presentation copy');

  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toMatch(/Discard your unsaved changes/);
    await dialog.dismiss();
  });
  await cancel.click();
  await expect(metadataForm).toBeVisible();
  await expect(description).toHaveValue('Unsaved presentation copy');

  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toMatch(/Discard your unsaved changes/);
    await dialog.accept();
  });
  await cancel.click();
  await expect(page.getByRole('heading', { name: 'Asset details', exact: true })).toHaveCount(0);

  metadataForm = await openAssetEditor(page, title);
  const subject = metadataForm.getByLabel('Subject', { exact: true });
  await subject.fill('Physics');
  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toMatch(/affects retrieval metadata/);
    await dialog.accept();
  });
  await metadataForm.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /Metadata saved/ })).toBeVisible();

  metadataForm = await openAssetEditor(page, title);
  await expect(metadataForm.getByLabel('Subject', { exact: true })).toHaveValue('Physics');

  const sourceForm = page
    .getByRole('heading', { name: 'Source document', exact: true })
    .locator('xpath=ancestor::form[1]');
  const replacementName = `version-2-${unique}.pdf`;
  await sourceForm.locator('input[type="file"]').setInputFiles({
    name: replacementName,
    mimeType: 'application/pdf',
    buffer: minimalPdf(`version-2-${unique}`),
  });
  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toMatch(/new immutable source revision/);
    await dialog.accept();
  });
  await sourceForm.getByRole('button', { name: 'Replace source document', exact: true }).click();
  await expect(page.getByRole('status').filter({ hasText: /New PDF source revision registered/ })).toBeVisible();

  await openAssetEditor(page, title);
  await expect(page.getByText(replacementName, { exact: true })).toBeVisible();
  const selectedCard = page
    .getByText(title, { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(selectedCard).toContainText(/Revision: [3-9][0-9]*/);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);

  const adminLogin = await page.context().request.post('/api/auth/login', {
    data: { email: 'e2e-admin@example.test', password: FIXTURE_PASSWORD },
  });
  expect(adminLogin.ok()).toBeTruthy();
  await page.goto('/dashboard/knowledge-audit');
  const auditRows = page.getByRole('row').filter({ hasText: title });
  await expect(auditRows.filter({ hasText: 'Asset details updated' })).toHaveCount(1);
  const replaced = auditRows.filter({ hasText: 'Source document replaced' });
  await expect(replaced).toHaveCount(1);
  await replaced.getByRole('button', { name: 'View details', exact: true }).click();
  await expect(page.getByRole('dialog')).toContainText('knowledge_asset.source_replaced');
  await expect(page.getByRole('dialog')).toContainText(replacementName);
});

test('manager knowledge editing surface renders Persian controls @smoke @workflow-truth', async ({ page }) => {
  await signIn(page, 'fa');
  await actionWithIcon(page, 'upload_file').click();
  await expect(page.getByRole('heading', { name: 'ویرایش منابع دانشی' })).toBeVisible();
  await expect(page.getByText(/تاریخچهٔ نسخه‌های قبلی حفظ می‌شود/)).toBeVisible();
  await assertLocaleDirection(page, 'fa');
});

for (const locale of ['en', 'fa'] as const) {
  test(`manager knowledge editor is usable at tablet viewport in ${locale} @smoke @workflow-truth @tablet`, async ({ page }) => {
    await page.setViewportSize(TABLET_VIEWPORT);
    await signIn(page, locale);
    await assertLocaleDirection(page, locale);
    await openKnowledgeManagerResponsive(page, locale);
    const form = await openLocalizedFixtureEditor(page, locale);
    await expect(page.getByRole('dialog')).toBeInViewport({ ratio: 0.9 });
    await expect(
      form.getByRole('button', { name: locale === 'fa' ? 'ذخیره' : 'Save', exact: true }),
    ).toBeVisible();
    await assertEditorAccessibility(page, locale);
    await assertNoHorizontalOverflow(page);
  });

  test(`manager knowledge editor is usable on mobile in ${locale} @smoke @workflow-truth @mobile`, async ({ page }) => {
    test.skip(test.info().project.name !== 'mobile-chromium', 'mobile evidence runs in the mobile Chromium project');
    await signIn(page, locale);
    await assertLocaleDirection(page, locale);
    await openKnowledgeManagerResponsive(page, locale);
    const form = await openLocalizedFixtureEditor(page, locale);
    await expect(form).toBeVisible();
    await expect(
      form.getByRole('button', { name: locale === 'fa' ? 'ذخیره' : 'Save', exact: true }),
    ).toBeVisible();
    await assertEditorAccessibility(page, locale);
    await assertNoHorizontalOverflow(page);
  });
}
