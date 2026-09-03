import { test, expect, type Page, type Locator } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const ADMIN = 'e2e-admin@example.test';
const SCHOOL_A_ID = 'a0000000-0000-0000-0000-0000000000a1';
const VERIFIED_ASSET_ID = 'f3000000-0000-0000-0000-0000000000a2';

async function openAdminRoute(
  page: Page,
  locale: 'en' | 'fa',
  path: '/dashboard' | '/dashboard/knowledge-audit',
): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);

  const login = await page.context().request.post('/api/auth/login', {
    data: { email: ADMIN, password: PASSWORD },
  });
  expect(login.ok(), 'platform admin session setup failed').toBeTruthy();

  const response = await page.goto(path);
  expect(response === null || response.status() < 400).toBeTruthy();
  await expect(page).toHaveURL(new RegExp(`${path.replaceAll('/', '\\/')}$`));
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
    new RegExp(`^${locale}`, 'i'),
  );
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe(
    locale === 'fa' ? 'rtl' : 'ltr',
  );
}

async function expectNoRawAdminChrome(body: Locator): Promise<void> {
  await expect(body).not.toContainText(/platform_admin\.[a-z0-9_.]+/i);
  await expect(body).not.toContainText(/\b(?:ocr_ready|ocr_pending|embedding_pending)\b/);
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

for (const scenario of [
  {
    locale: 'en' as const,
    role: 'Platform Administrator',
    reviewTitle: 'Governed knowledge review',
    schoolLabel: 'School',
    languageLabel: 'Language',
    status: 'OCR verified',
    sourceReview: 'Review private PDF',
    updateOcr: 'Update verified OCR',
    provider: 'OCR provider / verification process',
    verifiedText: 'Verified source text',
    cancel: 'Cancel',
    withdrawArchive: 'Withdraw / archive',
    archiveDialog: 'Archive asset',
    auditTitle: 'Knowledge audit trail',
    time: 'Time',
    actor: 'Actor',
    action: 'Action',
    target: 'Target',
    details: 'Details',
    viewDetails: 'View details',
    detailTitle: 'Audit event details',
    exactUtc: 'Exact UTC timestamp',
    exactAction: 'Exact action code',
    exactTarget: 'Exact target ID',
  },
  {
    locale: 'fa' as const,
    role: 'مدیر سامانه',
    reviewTitle: 'بازبینی دانش کنترل‌شده',
    schoolLabel: 'مدرسه',
    languageLabel: 'زبان',
    status: 'OCR تأییدشده',
    sourceReview: 'بازبینی PDF خصوصی',
    updateOcr: 'به‌روزرسانی OCR تأییدشده',
    provider: 'ارائه‌دهنده OCR / فرایند تأیید',
    verifiedText: 'متن تأییدشده منبع',
    cancel: 'انصراف',
    withdrawArchive: 'خروج از استفاده / بایگانی',
    archiveDialog: 'بایگانی منبع',
    auditTitle: 'ردپای ممیزی دانش',
    time: 'زمان',
    actor: 'عامل',
    action: 'اقدام',
    target: 'هدف',
    details: 'جزئیات',
    viewDetails: 'مشاهده جزئیات',
    detailTitle: 'جزئیات رویداد ممیزی',
    exactUtc: 'زمان دقیق UTC',
    exactAction: 'کد دقیق اقدام',
    exactTarget: 'شناسه دقیق هدف',
  },
]) {
  test(`platform admin governance cards are readable and localized in ${scenario.locale} @smoke @final @platform-admin @i18n @workflow-truth`, async ({ page }) => {
    await openAdminRoute(page, scenario.locale, '/dashboard');
    const body = page.locator('body');

    await expect(page.getByText(scenario.role, { exact: true }).first()).toBeVisible();
    await expect(page.getByText(scenario.reviewTitle, { exact: true })).toBeVisible();

    const verifiedCard = page.locator('article').filter({
      has: page.getByText('E2E Verified OCR Asset', { exact: true }),
    });
    await expect(verifiedCard).toBeVisible();
    await expect(verifiedCard).toContainText('E2E School A');
    await expect(verifiedCard).toContainText(`${scenario.schoolLabel}:`);
    await expect(verifiedCard).toContainText(scenario.languageLabel);
    await expect(verifiedCard).toContainText(scenario.status);
    await expect(verifiedCard).not.toContainText(SCHOOL_A_ID);
    await expect(verifiedCard).not.toContainText('ocr_ready');
    await expect(verifiedCard.getByRole('link', { name: scenario.sourceReview, exact: true })).toBeVisible();
    await expectNoRawAdminChrome(body);

    await verifiedCard.getByRole('button', { name: scenario.updateOcr, exact: true }).click();
    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText(scenario.provider, { exact: true })).toBeVisible();
    await expect(dialog.getByText(scenario.verifiedText, { exact: true })).toBeVisible();
    await expect(dialog.locator('#verified-ocr-text')).toHaveValue('E2E preverified OCR text');
    await expect(dialog.locator('#ocr-provider')).toHaveValue('e2e-manual-review');
    await expect(dialog).not.toContainText(/platform_admin\.[a-z0-9_.]+/i);
    await dialog.getByRole('button', { name: scenario.cancel, exact: true }).click();
    await expect(dialog).toHaveCount(0);

    const publishedCard = page.locator('article').filter({
      has: page.getByText('E2E Published Asset', { exact: true }),
    });
    await publishedCard.getByRole('button', { name: scenario.withdrawArchive, exact: true }).click();
    dialog = page.getByRole('dialog');
    await expect(dialog).toContainText(scenario.archiveDialog);
    await dialog.getByRole('button', { name: scenario.cancel, exact: true }).click();
    await expect(dialog).toHaveCount(0);

    if (scenario.locale === 'fa') {
      await expect(body).not.toContainText('Governed knowledge review');
      await expect(body).not.toContainText('Review private PDF');
      await expect(body).not.toContainText('Update verified OCR');
      await expect(body).not.toContainText('Source document');
    }
  });

  test(`platform admin audit is readable first and technical on demand in ${scenario.locale} @smoke @final @platform-admin @i18n @workflow-truth`, async ({ page }) => {
    await openAdminRoute(page, scenario.locale, '/dashboard/knowledge-audit');
    const body = page.locator('body');

    await expect(page.getByText(scenario.auditTitle, { exact: true })).toBeVisible();
    const table = page.locator('table');
    await expect(table).toBeVisible();
    await expect(table.getByRole('columnheader', { name: scenario.time, exact: true })).toBeVisible();
    await expect(table.getByRole('columnheader', { name: scenario.actor, exact: true })).toBeVisible();
    await expect(table.getByRole('columnheader', { name: scenario.action, exact: true })).toBeVisible();
    await expect(table.getByRole('columnheader', { name: scenario.target, exact: true })).toBeVisible();
    await expect(table.getByRole('columnheader', { name: scenario.details, exact: true })).toBeVisible();

    await expect(table).not.toContainText(SCHOOL_A_ID);
    await expect(table).not.toContainText(VERIFIED_ASSET_ID);
    await expect(table).not.toContainText(/\bPlatformAdmin\b/);
    await expect(table).not.toContainText(/\bknowledge_[a-z_]+\.[a-z_]+\b/);
    await expect(table).not.toContainText(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
    await expectNoRawAdminChrome(body);

    const detailsButton = table.getByRole('button', { name: scenario.viewDetails, exact: true }).first();
    await expect(detailsButton).toBeVisible();
    await detailsButton.click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toContainText(scenario.detailTitle);
    await expect(dialog.getByText(scenario.exactUtc, { exact: true })).toBeVisible();
    await expect(dialog.getByText(scenario.exactAction, { exact: true })).toBeVisible();
    await expect(dialog.getByText(scenario.exactTarget, { exact: true })).toBeVisible();
    await expect(dialog).toContainText(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
    await expect(dialog).toContainText(/[a-f0-9]{8}-[a-f0-9-]{27,}/i);
    await expect(dialog).toContainText(/[a-z_]+\.[a-z_]+/i);
    await expect(dialog).not.toContainText(/platform_admin\.[a-z0-9_.]+/i);

    if (scenario.locale === 'fa') {
      await expect(body).not.toContainText('Knowledge audit trail');
      await expect(table).not.toContainText('View details');
    }
  });
}
