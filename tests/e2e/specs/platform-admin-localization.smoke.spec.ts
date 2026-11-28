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

// Synthetic fixture setup/cleanup uses the same transaction-scoped actor
// contract enforced by the production teacher override trigger. Never disable it.
function runAdminFixtureMutation(sql: string): void {
  runFixtureSql(`
    BEGIN;
    SELECT set_config('app.user_id', 'b0000000-0000-0000-0000-0000000000a0', true),
           set_config('app.user_role', 'PlatformAdmin', true),
           set_config('app.school_id', '${SCHOOL_A_ID}', true),
           set_config('app.elevated_operation', 'false', true);
    ${sql};
    COMMIT;
  `);
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
    WHERE id = '${VERIFIED_ASSET_ID}' AND status IN ('embedded','failed');

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
  path: '/dashboard' | '/dashboard/knowledge-audit' | '/dashboard/personalization',
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
    vectorQueued: 'Queued',
    vectorRunning: 'Running',
    vectorFailed: 'Failed',
    vectorRetry: 'Retry vectorization',
    vectorDetails: 'Vectorization details',
    vectorChunks: 'Stored chunks',
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
    vectorQueued: 'در صف',
    vectorRunning: 'در حال اجرا',
    vectorFailed: 'ناموفق',
    vectorRetry: 'تلاش دوباره برای بردارسازی',
    vectorDetails: 'جزئیات بردارسازی',
    vectorChunks: 'قطعه‌های ذخیره‌شده',
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
    await expect(vectorMethod.locator('option:disabled')).toHaveCount(1);
    const startVectorization = verifiedCard.getByRole('button', {
      name: scenario.startVectorization,
      exact: true,
    });
    await expect(startVectorization).toBeVisible();
    await expect(startVectorization).toBeEnabled();
    // Visibility alone did not catch white text on a transparent button.
    // Assert the actual production CSS, without selecting its text.
    const originalViewport = page.viewportSize();
    const originalDark = await page.evaluate(() => document.documentElement.classList.contains('dark'));
    for (const width of [320, 375, 430, 1024]) {
      await page.setViewportSize({ width, height: 768 });
      for (const dark of [false, true]) {
        await page.evaluate(value => document.documentElement.classList.toggle('dark', value), dark);
        await expect(startVectorization).toBeVisible();
        const readAppearance = () => startVectorization.evaluate(button => {
          const css = getComputedStyle(button);
          const rgb = (value: string) => value.match(/[\d.]+/g)?.map(Number) ?? [];
          const luminance = (value: number[]) => value.slice(0,3).map(v => {
            const n = v / 255;
            return n <= 0.04045 ? n / 12.92 : ((n + 0.055) / 1.055) ** 2.4;
          }).reduce((sum, channel, index) => sum + channel * [0.2126,0.7152,0.0722][index],0);
          const background = rgb(css.backgroundColor);
          const foreground = rgb(getComputedStyle(button.querySelector('span:last-child') ?? button).color);
          const values = [luminance(background),luminance(foreground)].sort((a,b)=>b-a);
          return { opaque: background.length === 3 || background[3] === 1, contrast: (values[0]+0.05)/(values[1]+0.05), height: button.getBoundingClientRect().height, colors: [css.backgroundColor, getComputedStyle(button.querySelector('span:last-child') ?? button).color], opacity: css.opacity };
        });
        const verifyAppearance = async (state: string) => {
          const appearance = await readAppearance();
          const context = `${scenario.locale} ${width}px ${dark ? 'dark' : 'light'} ${state}: ${appearance.colors.join(' / ')}`;
          expect(appearance.opaque, context + ' CTA must not be transparent').toBeTruthy();
          expect(appearance.contrast, context + ' CTA text contrast').toBeGreaterThanOrEqual(4.5);
          expect(appearance.opacity, context + ' CTA must not be faded').toBe('1');
          expect(appearance.height, context + ' CTA touch target').toBeGreaterThanOrEqual(44);
        };
        await page.mouse.move(0, 0);
        await verifyAppearance('normal');
        await startVectorization.hover();
        await verifyAppearance('hover');
        await page.mouse.move(0, 0);
        await startVectorization.focus();
        await expect(startVectorization).toBeFocused();
        await verifyAppearance('focus');
        await startVectorization.evaluate(button => (button as HTMLButtonElement).blur());
      }
    }
    await page.evaluate(value => document.documentElement.classList.toggle('dark', value), originalDark);
    if (originalViewport) await page.setViewportSize(originalViewport);
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
    let queueRequests = 0;
    let releaseQueue!: () => void;
    let queueReached!: () => void;
    const queueHeld = new Promise<void>(resolve => { releaseQueue = resolve; });
    const queueSeen = new Promise<void>(resolve => { queueReached = resolve; });
    const queueRoute = '**/api/admin/knowledge-assets/embed-configured';
    await page.route(queueRoute, async route => {
      queueRequests += 1;
      queueReached();
      await queueHeld;
      await route.continue();
    });
    try {
      await startVectorization.click();
      await queueSeen;
      await expect(startVectorization).toBeDisabled();
      await expect(startVectorization).toHaveAttribute('aria-busy', 'true');
      await expect(startVectorization).toHaveCSS('opacity', '1');
      // A second click at the same actual control cannot enqueue again.
      await startVectorization.evaluate(button => (button as HTMLButtonElement).click());
      expect(queueRequests).toBe(1);
    } finally {
      releaseQueue();
    }
    const vectorPanel = verifiedCard.getByRole('region', {name: scenario.vectorTitle, exact: true});
    await expect(vectorPanel.getByRole('status')).toHaveText(scenario.vectorQueued);
    expect(queueRequests).toBe(1);
    await page.unroute(queueRoute);
    await expect(pendingPublish).toBeDisabled();
    expect(runFixtureSql(`SELECT count(*) FROM ingestion_jobs WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed'`)).toBe('1');
    // Exercise the terminal failure and explicit retry using the same
    // database transitions as the repository. No external vector provider.
    runFixtureSql(`
      UPDATE ingestion_jobs SET status='running', attempts=1, started_at=NOW(), locked_at=NOW(), heartbeat_at=NOW()
      WHERE asset_id='${VERIFIED_ASSET_ID}' AND stage='embed' AND status='queued'
    `);
    await expect(vectorPanel.getByRole('status')).toHaveText(scenario.vectorRunning);
    await expect(pendingPublish).toBeDisabled();
    runFixtureSql(`
      BEGIN;
      UPDATE ingestion_jobs SET status='failed', error_message='Synthetic worker failure', finished_at=NOW(), locked_at=NULL, heartbeat_at=NULL
      WHERE asset_id='${VERIFIED_ASSET_ID}' AND stage='embed' AND status='running';
      UPDATE knowledge_assets SET status='failed', failure_reason='Synthetic worker failure'
      WHERE id='${VERIFIED_ASSET_ID}' AND status='embedding_pending';
      COMMIT;
    `);
    await expect(vectorPanel.getByRole('status')).toHaveText(scenario.vectorFailed, {timeout: 15_000});
    await expect(pendingPublish).toBeDisabled();
    const retryVectorization = vectorPanel.getByRole('button', {name:scenario.vectorRetry, exact:true});
    await expect(retryVectorization).toBeEnabled();
    await retryVectorization.click();
    await completeQueuedVectorizationFixture();
    expect(runFixtureSql(`SELECT count(*) FROM ingestion_jobs WHERE asset_id = '${VERIFIED_ASSET_ID}' AND stage = 'embed'`)).toBe('2');
    await expect(
      verifiedCard.getByText(scenario.vectorStored, { exact: true }).first(),
    ).toBeVisible({ timeout: 15_000 });
    await expect(verifiedCard.getByText(scenario.publicationStage, { exact: true })).toBeVisible();
    await expect(pendingPublish).toBeEnabled();
    await vectorPanel.getByText(scenario.vectorDetails, {exact:true}).click();
    await expect(vectorPanel.getByText(scenario.vectorChunks, {exact:true}).locator('..')).toContainText('1');

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

  test(`platform admin personalization operations are governed and localized in ${scenario.locale} @smoke @final @platform-admin @personalization @i18n @workflow-truth`, async ({ page }) => {
    await openAdminRoute(page, scenario.locale, '/dashboard/personalization');
    const body = page.locator('body');
    const copy =
      scenario.locale === 'fa'
        ? {
            title: 'مرکز کنترل شخصی‌سازی هوشمند',
            capability: 'قابلیت هوش مصنوعی',
            schoolPolicy: 'سیاست مدرسه',
            requirePersonalized: 'محتوای شخصی‌سازی‌شده الزامی است',
            specializationInstructions: 'راهنمای تخصصی‌سازی تکلیف',
            specializationExample: 'برای نمایش دموی فارسی، مراحل کوتاه و مثال‌های روشن را ترجیح بده.',
            savePolicy: 'ذخیره سیاست',
            manageTeachers: 'مدیریت استثناهای معلم',
            teacherPolicy: 'سیاست معلم',
            recentJobs: 'کارهای اخیر شخصی‌سازی',
            queueSummary: 'صف شخصی‌سازی',
            retryFailedScope: 'تلاش دوباره خطاهای این محدوده',
            regenerateCurrent: 'تلاش مجدد با تنظیمات فعلی هوش مصنوعی',
            cancel: 'انصراف',
            statusLabel: 'وضعیت',
            running: 'در حال اجرا',
            ready: 'آماده',
            retrieving: 'در حال بازیابی زمینه آموزشی',
            queuedAt: 'زمان ورود به صف',
            failed: 'ناموفق',
            failure: 'دروازه هوش مصنوعی موقتاً در دسترس نیست',
          }
        : {
            title: 'AI personalization control center',
            capability: 'AI capability',
            schoolPolicy: 'School policy',
            requirePersonalized: 'Require personalized content',
            specializationInstructions: 'Specialization instructions',
            specializationExample: 'Prefer short scaffolded steps and concrete examples for the demo.',
            savePolicy: 'Save policy',
            manageTeachers: 'Manage teacher overrides',
            teacherPolicy: 'Teacher policy',
            recentJobs: 'Recent personalization jobs',
            queueSummary: 'Personalization queue',
            retryFailedScope: 'Retry failed in this scope',
            regenerateCurrent: 'Retry with current AI settings',
            cancel: 'Cancel',
            statusLabel: 'Status',
            running: 'Running',
            ready: 'Ready',
            retrieving: 'Retrieving learning context',
            queuedAt: 'Queued at',
            failed: 'Failed',
            failure: 'AI gateway is temporarily unavailable',
          };

    await expectSidebarRole(page, scenario.role);
    await expect(page.getByText(copy.title, { exact: true })).toBeVisible();
    await expect(page.getByText(copy.capability, { exact: true })).toBeVisible();
    await expect(page.getByText(copy.queueSummary, { exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: copy.retryFailedScope, exact: true }).first()).toBeVisible();
    await expect(page.getByText(copy.schoolPolicy, { exact: true })).toBeVisible();

    const schoolCard = page.locator('article').filter({
      has: page.getByText('E2E School A', { exact: true }),
    });
    await expect(schoolCard).toBeVisible();
    await expect(schoolCard).toContainText('deepseek-chat-v1');
    await expect(schoolCard).toContainText(copy.requirePersonalized);
    const specializationInstructions = schoolCard.getByLabel(copy.specializationInstructions);
    await expect(specializationInstructions).toBeVisible();
    await specializationInstructions.fill(copy.specializationExample);
    await schoolCard.getByRole('button', { name: copy.savePolicy, exact: true }).click();
    await expect(schoolCard.getByRole('button', { name: copy.savePolicy, exact: true })).toBeVisible();
    await expect(specializationInstructions).toHaveValue(copy.specializationExample);
    await expect(page.getByRole('status')).toContainText(/v[0-9]+/);
    await expect(schoolCard.getByRole('button', { name: copy.retryFailedScope, exact: true })).toBeVisible();
    // New-policy regeneration is intentionally distinct from retrying the old
    // snapshotted provider. Explicit confirmation is required even for school
    // scope and dismissing it must be non-mutating.
    const regenerate = schoolCard.getByRole('button', {
      name: copy.regenerateCurrent, exact: true,
    });
    await expect(regenerate).toBeVisible();
    await regenerate.click();
    const confirmation = schoolCard.getByRole('alertdialog');
    await expect(confirmation).toBeVisible();
    await expect(confirmation).toContainText(/new jobs|کارهای جدید/);
    await confirmation.getByRole('button', { name: copy.cancel }).click();
    await expect(confirmation).toHaveCount(0);
    await schoolCard.getByRole('button', { name: copy.manageTeachers, exact: true }).click();

    await expect(
      page.locator('h2').filter({ hasText: copy.teacherPolicy }).first(),
    ).toBeVisible();
    const teacherCard = page.locator('article').filter({
      has: page.getByText('E2E Teacher A', { exact: true }),
    });
    await expect(teacherCard).toBeVisible();
    await expect(teacherCard).toContainText('deepseek-chat-v1');
    await expect(teacherCard.getByRole('button', { name: copy.retryFailedScope, exact: true })).toBeVisible();

    await expect(page.getByText(copy.recentJobs, { exact: true })).toBeVisible();
    const failedJob = page.locator('article').filter({
      has: page.getByText('E2E Personalization Unavailable', { exact: true }),
    });
    await expect(failedJob).toBeVisible();
    await expect(failedJob).toContainText(copy.failed);
    await expect(failedJob).toContainText(copy.queuedAt);
    await expect(failedJob).toContainText(copy.failure);
    const diagnostics = failedJob.locator('details').filter({ has: page.locator('summary').filter({ hasText: /Technical diagnostic details|جزئیات فنی خطا/ }) });
    await expect(diagnostics).toBeVisible();
    await diagnostics.locator('summary').click();
    await expect(diagnostics).toContainText('gateway_unavailable');
    await expect(diagnostics).toContainText(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/);
    await expect(failedJob.getByRole('button', { name: copy.retryFailedScope, exact: true })).toBeVisible();

    const runningJob = page.locator('article').filter({
      has: page.getByText('E2E Personalization Running', { exact: true }),
    });
    await expect(runningJob).toBeVisible();
    await expect(
      runningJob.getByText(`${copy.statusLabel}: ${copy.running}`, { exact: true }),
    ).toBeVisible();
    await expect(runningJob).toContainText(copy.retrieving);

    const readyJob = page.locator('article').filter({
      has: page.getByText('E2E Personalization Source Ready', { exact: true }),
    });
    await expect(readyJob).toBeVisible();
    await expect(readyJob).toContainText(copy.ready);

    await expect(body).not.toContainText(/platform_admin\.[a-z0-9_.]+/i);
    await expect(body).not.toContainText(/LLM_API_KEY|OPENAI_API_KEY|Authorization:\s*Bearer|prompt_ctx/i);
    if (scenario.locale === 'fa') {
      await expect(body).not.toContainText('AI personalization control center');
      await expect(body).not.toContainText('School policy');
      await expect(body).not.toContainText('Recent personalization jobs');
      await expect(body).not.toContainText('AI gateway is temporarily unavailable');
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

for (const locale of ['en', 'fa'] as const) {
  test(`admin distinguishes recorded gateway and upstream failures in ${locale} @smoke @platform-admin @personalization @i18n`, async ({ page }) => {
    const requestId = locale === 'en' ? 'd1000000-0000-0000-0000-000000000001' : 'd1000000-0000-0000-0000-000000000002';
    const timeoutId = locale === 'en' ? 'd1000000-0000-0000-0000-000000000004' : 'd1000000-0000-0000-0000-000000000005';
    try {
      runFixtureSql(`
        INSERT INTO assignment_personalization_gateway_failures (
          request_id, job_id, lease_owner, execution_generation, attempt_number, stage,
          diagnostic_code, gateway_http_status, gateway_elapsed_ms,
          provider_dispatch_state, provider_failure_reason, provider_transport_attempts, upstream_http_status
        )
        SELECT '${requestId}', j.id, 'd1000000-0000-0000-0000-000000000003', j.execution_generation, 1,
               'provider', 'invalid_provider_response', 502, 26327, 'responded', 'model_mismatch', 1, 200
        FROM assignment_personalization_jobs j JOIN assignments a ON a.id = j.assignment_id
        WHERE a.title = 'E2E Personalization Unavailable' AND j.school_id = '${SCHOOL_A_ID}'
        ORDER BY j.id LIMIT 1;
        INSERT INTO assignment_personalization_gateway_failures (
          request_id, job_id, lease_owner, execution_generation, attempt_number, stage,
          diagnostic_code, gateway_http_status, gateway_elapsed_ms, provider_dispatch_state
        )
        SELECT '${timeoutId}', job_id, 'd1000000-0000-0000-0000-000000000006', execution_generation, 2,
               'ai_gateway', 'gateway_transport_timeout', NULL, 400, 'unknown'
        FROM assignment_personalization_gateway_failures WHERE request_id = '${requestId}'
      `);
      await openAdminRoute(page, locale, '/dashboard/personalization');
      const card = page.locator('article').filter({ has: page.getByText('E2E Personalization Unavailable', { exact: true }) });
      const history = card.locator('details').filter({ has: page.locator('summary').filter({ hasText: /Gateway failure history|تاریخچه خطاهای دروازه/ }) });
      await expect(history).toBeVisible();
      await history.locator('summary').click();
      const observed = history.getByTestId('gateway-failure-record').filter({ hasText: requestId });
      const unknown = history.getByTestId('gateway-failure-record').filter({ hasText: timeoutId });
      await expect(observed).toContainText('model_mismatch');
      await expect(observed).toContainText(/Internal gateway HTTP status|وضعیت HTTP دروازه داخلی/);
      await expect(observed).toContainText(/Observed upstream HTTP status|وضعیت HTTP مشاهده‌شده ارائه‌دهنده/);
      await expect(observed).toContainText('502');
      await expect(observed).toContainText('200');
      await expect(unknown).toContainText('gateway_transport_timeout');
      await expect(unknown.locator('dt').filter({ hasText: /Internal gateway HTTP status|وضعیت HTTP دروازه داخلی/ }).locator('..').locator('dd')).toHaveText(/^(Unknown|نامشخص)$/);
      await expect(unknown).toContainText(/Unknown|نامشخص/);
      await expect(history).toContainText(/Assignment personalization chat|گفت‌وگوی شخصی‌سازی تکلیف/);
      await expect(page.getByText(/Last validated provider response|آخرین پاسخ معتبر ارائه‌دهنده/)).toBeVisible();
      await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
      await observed.getByRole('button', { name: /Copy request reference|کپی شناسه پیگیری درخواست/ }).click();
      await expect(observed.getByRole('status')).toHaveText(/Request reference copied|شناسه پیگیری درخواست کپی شد/);
      expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(requestId);
    } finally {
      runFixtureSql(`DELETE FROM assignment_personalization_gateway_failures WHERE request_id IN ('${requestId}', '${timeoutId}')`);
    }
  });
}

for (const locale of ['en', 'fa'] as const) {
  test(`saved non-default personalization profile is selected after reload in ${locale} @smoke @platform-admin @personalization @i18n`, async ({ page }) => {
    const original = runFixtureSql(`SELECT llm_profile_id FROM assignment_personalization_school_policies WHERE school_id = '${SCHOOL_A_ID}'`);
    expect(original).toMatch(/^(deepseek-chat-v1|deepseek-v41-flash-v1|custom-v1:[A-Za-z0-9][A-Za-z0-9._:/-]{0,95})$/);
    try {
      for (const profile of ['deepseek-v41-flash-v1', 'custom-v1:gpt-6-luna']) {
        runFixtureSql(`UPDATE assignment_personalization_school_policies SET llm_profile_id = '${profile}', policy_version = policy_version + 1 WHERE school_id = '${SCHOOL_A_ID}'`);
        await openAdminRoute(page, locale, '/dashboard/personalization');
        const card = page.locator('article').filter({ has: page.getByText('E2E School A', { exact: true }) });
        const profileSelect = card.locator('select').first();
        await expect(card.getByTestId('saved-school-profile')).toContainText(profile);
        await expect(profileSelect).toHaveValue(profile);
        await expect(profileSelect.locator('option:checked')).toHaveAttribute('value', profile);
        await page.reload();
        await expect(profileSelect).toHaveValue(profile);
        // Opening the view must not silently rewrite a saved custom profile.
        expect(runFixtureSql(`SELECT llm_profile_id FROM assignment_personalization_school_policies WHERE school_id = '${SCHOOL_A_ID}'`)).toBe(profile);
      }
      const card = page.locator('article').filter({ has: page.getByText('E2E School A', { exact: true }) });
      await card.locator('select').first().selectOption('deepseek-chat-v1');
      const save = locale === 'fa' ? 'ذخیره سیاست' : 'Save policy';
      await card.getByRole('button', { name: save, exact: true }).click();
      await expect(card.getByTestId('saved-school-profile')).toContainText('deepseek-chat-v1');
      await expect.poll(() => runFixtureSql(`SELECT llm_profile_id FROM assignment_personalization_school_policies WHERE school_id = '${SCHOOL_A_ID}'`)).toBe('deepseek-chat-v1');
      await page.reload();
      await expect(card.locator('select').first()).toHaveValue('deepseek-chat-v1');
    } finally {
      runFixtureSql(`UPDATE assignment_personalization_school_policies SET llm_profile_id = '${original}', policy_version = policy_version + 1 WHERE school_id = '${SCHOOL_A_ID}'`);
    }
  });

  test(`teacher override retains saved and effective AI profile in ${locale} @smoke @platform-admin @personalization @i18n`, async ({ page }) => {
    const teacherId = 'c0000000-0000-0000-0000-0000000000a2';
    const table = 'public.assignment_personalization_teacher_overrides';
    const original = runFixtureSql(`SELECT to_jsonb(o)::text FROM ${table} o WHERE teacher_id = '${teacherId}'`);
    const savedProfile = 'custom-v1:gpt-6-luna';
    const manage = locale === 'fa' ? 'مدیریت استثناهای معلم' : 'Manage teacher overrides';
    const save = locale === 'fa' ? 'ذخیره سیاست' : 'Save policy';
    try {
      runAdminFixtureMutation(`
        INSERT INTO ${table} AS current_override (teacher_id, school_id, mode, llm_profile_id_override)
        VALUES ('${teacherId}', '${SCHOOL_A_ID}', 'override', '${savedProfile}')
        ON CONFLICT (teacher_id) DO UPDATE
          SET mode = 'override', llm_profile_id_override = EXCLUDED.llm_profile_id_override,
              override_version = current_override.override_version + 1
      `);
      await openAdminRoute(page, locale, '/dashboard/personalization');
      const schoolCard = page.locator('article').filter({ has: page.getByText('E2E School A', { exact: true }) });
      const teacherCard = page.locator('article').filter({ has: page.getByText('E2E Teacher A', { exact: true }) });
      await schoolCard.getByRole('button', { name: manage, exact: true }).click();
      const mode = teacherCard.locator('select').nth(0);
      const profile = teacherCard.locator('select').nth(1);
      await expect(mode).toHaveValue('override');
      await expect(profile).toHaveValue(savedProfile);
      await expect(teacherCard.getByTestId('effective-teacher-policy')).toContainText(savedProfile);
      await page.reload();
      await schoolCard.getByRole('button', { name: manage, exact: true }).click();
      await expect(profile).toHaveValue(savedProfile);
      expect(runFixtureSql(`SELECT llm_profile_id_override FROM ${table} WHERE teacher_id = '${teacherId}'`)).toBe(savedProfile);

      await profile.selectOption('deepseek-chat-v1');
      // A draft selection must not change the persisted/effective label before save.
      await expect(teacherCard.getByTestId('effective-teacher-policy')).toContainText(savedProfile);
      await teacherCard.getByRole('button', { name: save, exact: true }).click();
      await expect.poll(() => runFixtureSql(`SELECT llm_profile_id_override FROM ${table} WHERE teacher_id = '${teacherId}'`)).toBe('deepseek-chat-v1');
      await expect(teacherCard.getByTestId('effective-teacher-policy')).toContainText('deepseek-chat-v1');
      await page.reload();
      await schoolCard.getByRole('button', { name: manage, exact: true }).click();
      await expect(profile).toHaveValue('deepseek-chat-v1');
      await expect(teacherCard.getByTestId('effective-teacher-policy')).toContainText('deepseek-chat-v1');
    } finally {
      // Only the synthetic fixture is restored; normal saves above use the application.
      if (original) {
        const escaped = original.replaceAll("'", "''");
        runAdminFixtureMutation(`
          UPDATE ${table} o
          SET mode = prior.mode, enabled_override = prior.enabled_override,
              paused_override = prior.paused_override,
              llm_profile_id_override = prior.llm_profile_id_override,
              delivery_policy_override = prior.delivery_policy_override,
              updated_by = prior.updated_by, override_version = o.override_version + 1
          FROM jsonb_populate_record(NULL::${table}, '${escaped}'::jsonb) prior
          WHERE o.teacher_id = '${teacherId}'
        `);
      } else {
        runAdminFixtureMutation(`DELETE FROM ${table} WHERE teacher_id = '${teacherId}'`);
      }
    }
  });
}
