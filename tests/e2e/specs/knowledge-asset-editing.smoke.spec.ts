// @smoke @workflow-truth — #34 focused browser evidence for lifecycle-aware
// knowledge asset editing/versioning. Uses the real Dioxus server functions,
// PostgreSQL RLS context, and local private-storage contract; application
// responses are not stubbed.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const FIXTURE_PASSWORD = 'e2e-password';
const MANAGER_EMAIL = 'e2e-manager-a@example.test';

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
  return form;
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
  const description = metadataForm.locator('textarea').nth(0);
  const cancel = metadataForm.getByRole('button', { name: 'Cancel', exact: true });
  const save = metadataForm.getByRole('button', { name: 'Save', exact: true });
  await expect(save).toBeDisabled();

  await description.fill('Unsaved presentation copy');
  await expect(metadataForm.getByText('Unsaved changes', { exact: true })).toBeVisible();
  await expect(save).toBeEnabled();

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
  const subject = metadataForm.locator('input[type="text"]').nth(2);
  await subject.fill('Physics');
  page.once('dialog', async (dialog) => {
    expect(dialog.message()).toMatch(/affects retrieval metadata/);
    await dialog.accept();
  });
  await metadataForm.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByRole('status')).toContainText(/Metadata saved/);

  metadataForm = await openAssetEditor(page, title);
  await expect(metadataForm.locator('input[type="text"]').nth(2)).toHaveValue('Physics');

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
  await expect(page.getByRole('status')).toContainText(/New PDF source revision registered/);

  await openAssetEditor(page, title);
  await expect(page.getByText(replacementName, { exact: true })).toBeVisible();
  const selectedCard = page
    .getByText(title, { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(selectedCard).toContainText(/Revision: [3-9][0-9]*/);
});

test('manager knowledge editing surface renders Persian controls @smoke @workflow-truth', async ({ page }) => {
  await signIn(page, 'fa');
  await actionWithIcon(page, 'upload_file').click();
  await expect(page.getByRole('heading', { name: 'ویرایش منابع دانشی' })).toBeVisible();
  await expect(page.getByText(/وضعیت چرخهٔ عمر فقط در سمت سرور تغییر می‌کند/)).toBeVisible();
});
