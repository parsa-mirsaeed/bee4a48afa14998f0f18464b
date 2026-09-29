import { execFileSync } from 'node:child_process';
import { test, expect, type Page, type Locator } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';
import { expectSidebarRole } from '../fixtures/dashboard-sidebar';

const PASSWORD = 'e2e-password';
const ADMIN = 'e2e-admin@example.test';
const SCHOOL_A_ID = 'a0000000-0000-0000-0000-0000000000a1';
const VERIFIED_ASSET_ID = 'f3000000-0000-0000-0000-0000000000a2';

function runFixtureSql(sql: string): string {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) {
    throw new Error('DATABASE_URL is required for the governed browser fixture');
  }
  return execFileSync(
    'psql',
    [databaseUrl, '-v', 'ON_ERROR_STOP=1', '-Atqc', sql],
    { encoding: 'utf8' },
  ).trim();
}

function resetVectorizationFixture(): void {
  runFixtureSql(`
    BEGIN;
    DELETE FROM knowledge_chunks WHERE asset_id = '${VERIFIED_ASSET_ID}';
    DELETE FROM ingestion_jobs WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed';

    -- Respect the production lifecycle trigger while returning the shared browser
    -- fixture to OCR-ready. An embedded asset may only re-enter OCR through the
    -- legal embedded -> embedding_pending -> ocr_ready recovery path.
    UPDATE knowledge_assets
    SET status = 'embedding_pending', failure_reason = NULL, reviewed_by = NULL, published_at = NULL
    WHERE id = '${VERIFIED_ASSET_ID}' AND status = 'embedded';

    UPDATE knowledge_assets
    SET status = 'ocr_ready', failure_reason = NULL, reviewed_by = NULL, published_at = NULL
    WHERE id = '${VERIFIED_ASSET_ID}' AND status = 'embedding_pending';
    COMMIT;
  `);
}

async function completeQueuedVectorizationFixture(): Promise<void> {
  await expect.poll(
    () =>
      runFixtureSql(`
        SELECT status::text
        FROM ingestion_jobs
        WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed'
        ORDER BY created_at DESC
        LIMIT 1
      `),
    { timeout: 15_000 },
  ).toBe('queued');

  runFixtureSql(`
    BEGIN;
    UPDATE ingestion_jobs
    SET status = 'running',
        attempts = attempts + 1,
        started_at = COALESCE(started_at, NOW()),
        locked_at = NOW(),
        heartbeat_at = NOW()
    WHERE id = (
      SELECT id
      FROM ingestion_jobs
      WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed' AND status = 'queued'
      ORDER BY created_at DESC
      LIMIT 1
    );

    DELETE FROM knowledge_chunks WHERE asset_id = '${VERIFIED_ASSET_ID}';

    INSERT INTO knowledge_chunks (
      asset_id, chunk_index, text, token_count, embedding_provider,
      embedding_model, vector_id, metadata_json
    )
    SELECT
      asset_id,
      0,
      'E2E deterministic vectorized text',
      4,
      embedding_provider,
      embedding_model,
      'knowledge:' || asset_id::text || ':0',
      jsonb_build_object(
        'embedding_profile', embedding_profile,
        'embedding_collection', embedding_collection,
        'embedding_dimensions', embedding_dimensions,
        'chunk_size', chunk_size,
        'chunk_overlap', chunk_overlap
      )
    FROM ingestion_jobs
    WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed' AND status = 'running'
    ORDER BY created_at DESC
    LIMIT 1;

    UPDATE knowledge_assets
    SET status = 'embedded', reviewed_by = 'b0000000-0000-0000-0000-0000000000a0'
    WHERE id = '${VERIFIED_ASSET_ID}' AND status = 'embedding_pending';

    UPDATE ingestion_jobs
    SET status = 'succeeded',
        finished_at = NOW(),
        locked_at = NULL,
        heartbeat_at = NULL
    WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed' AND status = 'running';
    COMMIT;
  `);
}

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
  resetVectorizationFixture();
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
    vectorTitle: 'Vectorization',
    vectorMethod: 'Vectorization method',
    vectorStatus: 'Vector status',
    vectorNotStarted: 'Not started',
    vectorStored: 'Stored successfully',
    publicationStage: 'Step 3 · Publication',
    startVectorization: 'Start vectorization',
    publish: 'Publish',
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
    vectorTitle: 'بردارسازی',
    vectorMethod: 'روش بردارسازی',
    vectorStatus: 'وضعیت بردار',
    vectorNotStarted: 'شروع نشده',
    vectorStored: 'با موفقیت ذخیره شد',
    publicationStage: 'مرحله ۳ · انتشار',
    startVectorization: 'شروع بردارسازی',
    publish: 'انتشار',
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

    await expectSidebarRole(page, scenario.role);
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
    await expect(verifiedCard.getByText(scenario.vectorTitle, { exact: true })).toBeVisible();
    await expect(verifiedCard.getByText(scenario.vectorMethod, { exact: true })).toBeVisible();
    await expect(verifiedCard.getByText(scenario.vectorStatus, { exact: true })).toBeVisible();
    await expect(
      verifiedCard.getByText(scenario.vectorNotStarted, { exact: true }).first(),
    ).toBeVisible();
    const vectorMethod = verifiedCard.locator('select[id^="vector-profile-"]');
    await expect(vectorMethod).toBeVisible();
    await expect(vectorMethod.locator('option')).toHaveCount(2);
    const startVectorization = verifiedCard.getByRole('button', {
      name: scenario.startVectorization,
      exact: true,
    });
    await expect(startVectorization).toBeVisible();
    await expect(startVectorization).toBeEnabled();
    const pendingPublish = verifiedCard.getByRole('button', {
      name: scenario.publish,
      exact: true,
    });
    await expect(pendingPublish).toBeVisible();
    await expect(pendingPublish).toBeDisabled();
    await expectNoRawAdminChrome(body);

    await verifiedCard.getByRole('button', { name: scenario.updateOcr, exact: true }).click();
    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText(scenario.provider, { exact: true })).toBeVisible();
    await expect(dialog.getByText(scenario.verifiedText, { exact: true })).toBeVisible();
    await expect(dialog.locator('#verified-ocr-text')).toHaveValue('E2E preverified OCR text');
    await expect(dialog.locator('#ocr-provider')).toHaveValue('e2e-manual-review');
    await expect(dialog).not.toContainText(/source review required/i);
    await expect(dialog).not.toContainText(/platform_admin\.[a-z0-9_.]+/i);
    await dialog.getByRole('button', { name: scenario.cancel, exact: true }).click();
    await expect(dialog).toHaveCount(0);

    // Reproduce the real regression path: the mounted card starts at ocr_ready,
    // queues vectorization through the product endpoint, and is then completed
    // deterministically in PostgreSQL because browser proof intentionally runs
    // without Qdrant/provider services. The existing client polling must update
    // the same card in place and enable its already-mounted Publish control.
    await startVectorization.click();
    await completeQueuedVectorizationFixture();
    await expect(
      verifiedCard.getByText(scenario.vectorStored, { exact: true }).first(),
    ).toBeVisible({ timeout: 15_000 });
    await expect(verifiedCard.getByText(scenario.publicationStage, { exact: true })).toBeVisible();
    await expect(pendingPublish).toBeEnabled();

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
      await expect(body).not.toContainText('Vectorization method');
      await expect(body).not.toContainText('Start vectorization');
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
