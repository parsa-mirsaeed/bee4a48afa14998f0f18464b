import { execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors, allowHttpResponse } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const TEACHER = 'e2e-teacher-a@example.test';

async function establishSession(page: Page, locale: 'en' | 'fa', email = TEACHER): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);

  // Authentication itself is covered by auth.smoke.spec.ts. This localization
  // journey needs a real authenticated session, but making the assignment route
  // the first hydrated Dioxus document avoids an unrelated full-document teardown
  // race that can surface as WASM `unreachable` and can stall mobile navigation.
  const response = await page.request.post('/api/auth/login', {
    data: { email, password: PASSWORD },
  });
  expect(response.ok(), `teacher session setup failed with HTTP ${response.status()}`).toBeTruthy();
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
    create: 'Create assignment',
    viewDetails: 'View details',
    detailsTitle: 'Assignment details',
    titleLabel: 'Title',
    classLabel: 'Class',
    dueLabel: 'Due date',
    instructionsLabel: 'Instructions',
    createDraft: 'Create draft',
    classMaterials: 'Class materials (optional)',
    schoolKnowledge: 'School knowledge for AI generation',
    status: 'Published',
  },
  {
    locale: 'fa' as const,
    create: 'ایجاد تکلیف',
    viewDetails: 'مشاهده جزئیات',
    detailsTitle: 'جزئیات تکلیف',
    titleLabel: 'عنوان',
    classLabel: 'کلاس',
    dueLabel: 'تاریخ مهلت',
    instructionsLabel: 'دستورالعمل‌ها',
    createDraft: 'ایجاد پیش‌نویس',
    classMaterials: 'منابع کلاس (اختیاری)',
    schoolKnowledge: 'دانش مدرسه برای تولید با هوش مصنوعی',
    status: 'منتشرشده',
  },
]) {
  test(`teacher assignment create/detail chrome is localized in ${scenario.locale} @smoke @final @teacher @i18n @workflow-truth`, async ({ page }) => {
    await establishSession(page, scenario.locale);
    await page.goto('/dashboard/assignments');
    await expect(page).toHaveURL(/\/dashboard\/assignments$/);
    await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(
      new RegExp(`^${scenario.locale}`, 'i'),
    );

    const pageBody = page.locator('body');
    await expect(pageBody).not.toContainText('teacher.assignments.');

    const assignmentCard = page
      .getByText('E2E Assignment A1', { exact: true })
      .locator('xpath=ancestor::article[1]');
    await expect(assignmentCard).toBeVisible();
    await expect(assignmentCard).toContainText(scenario.status);

    if (scenario.locale === 'fa') {
      await expect(assignmentCard).not.toContainText('Published');
      await expect(assignmentCard).not.toContainText('Due ');
      await expect(assignmentCard).not.toContainText(' submitted');
      await expect(assignmentCard).not.toContainText('View details');
      await expect(assignmentCard).not.toContainText('Delete');
      await expect(assignmentCard).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }

    await page.getByRole('button', { name: scenario.create, exact: true }).click();
    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAccessibleName(scenario.create);
    await expect(dialog).toContainText(scenario.titleLabel);
    await expect(dialog).toContainText(scenario.classLabel);
    await expect(dialog).toContainText(scenario.dueLabel);
    await expect(dialog).toContainText(scenario.instructionsLabel);
    await expect(dialog).toContainText(scenario.schoolKnowledge);
    await expect(dialog).not.toContainText('Governed class materials (optional)');
    await expect(dialog.getByRole('button', { name: scenario.createDraft, exact: true })).toBeVisible();
    await expect(dialog).not.toContainText('teacher.assignments.');
    const classSelect = dialog.locator('select').first();
    await classSelect.selectOption({ index: 1 });
    await expect(dialog).toContainText(scenario.classMaterials);
    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText('Create draft');
      await expect(dialog).not.toContainText('Select one of your classes');
      await expect(dialog).not.toContainText('Instructions');
    }
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);

    await assignmentCard.getByRole('button', { name: scenario.viewDetails, exact: true }).click();
    dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAccessibleName(scenario.detailsTitle);
    await expect(dialog).toContainText(scenario.status);
    await expect(dialog).not.toContainText('teacher.assignments.');
    await expect(dialog).not.toContainText(/T\d{2}:\d{2}:\d{2}/);
    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText('Status:');
      await expect(dialog).not.toContainText('Published');
      await expect(dialog).not.toContainText('Due ');
      await expect(dialog).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }
  });
}

// Read the existing synthetic failed-job fixture; do not modify queue state.
function latestFailedJob(teacherId: string): string {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) throw new Error('DATABASE_URL is required for the teacher support-reference fixture');
  return execFileSync('psql', [databaseUrl, '-v', 'ON_ERROR_STOP=1', '-Atqc', `
    SELECT job.id FROM assignment_personalization_jobs job
    WHERE job.requested_by = '${teacherId}' AND job.status = 'failed'
      AND job.school_id = (SELECT school_id FROM users WHERE id = '${teacherId}')
      AND NOT EXISTS (
        SELECT 1 FROM assignment_personalization_jobs newer
        WHERE newer.school_id = job.school_id AND newer.requested_by = job.requested_by
          AND newer.assignment_id = job.assignment_id AND newer.student_id = job.student_id
          AND newer.profile_name = job.profile_name AND newer.profile_version = job.profile_version
          AND newer.execution_generation > job.execution_generation
      )
    ORDER BY job.completed_at DESC NULLS LAST, job.id DESC LIMIT 1
  `], { encoding: 'utf8' }).trim();
}

for (const locale of ['en', 'fa'] as const) {
  test(`teacher sees only a safe failed-job support reference in ${locale} @smoke @teacher @personalization @i18n`, async ({ page }) => {
    const expected = latestFailedJob('b0000000-0000-0000-0000-0000000000a2');
    expect(expected).toMatch(/^[0-9a-f-]{36}$/);
    await establishSession(page, locale);
    await page.goto('/dashboard/assignments');
    const panel = page.getByTestId('teacher-personalization-status');
    const reference = panel.getByTestId('teacher-personalization-support-reference');
    await expect(panel.getByRole('button', {name: locale === 'en' ? 'Refresh status' : 'به‌روزرسانی وضعیت', exact: true})).toBeVisible();
    await expect(reference).toBeHidden();
    await panel.getByText(locale === 'en' ? 'View details' : 'دریافت راهنمایی', {exact: true}).click();
    await expect(reference).toContainText(locale === 'en' ? 'Support reference' : 'شناسه پیگیری پشتیبانی');
    await expect(reference.locator('bdi')).toHaveText(expected.slice(0, 8) + '…');
    await expect(panel).not.toContainText(expected);
    await page.context().grantPermissions(['clipboard-read', 'clipboard-write']);
    await reference.getByRole('button', {name: locale === 'en' ? 'Copy support reference' : 'کپی شناسه پیگیری', exact:true}).click();
    await expect.poll(() => page.evaluate(() => navigator.clipboard.readText())).toBe(expected);
    await expect(panel).not.toContainText(/gateway_unavailable|provider_unconfigured|model_mismatch|Gateway failure history|تاریخچه خطاهای دروازه/);
    for (const width of [320, 375, 430, 1024]) {
      await page.setViewportSize({width, height: 844});
      for (const dark of [false, true]) {
        await page.evaluate(dark => document.documentElement.classList.toggle('dark', dark), dark);
        await expect.poll(() => panel.evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBeTruthy();
        const copy = reference.getByRole('button', {name: locale === 'en' ? 'Copy support reference' : 'کپی شناسه پیگیری', exact:true});
        await expect(copy).toBeVisible();
        expect((await copy.boundingBox())!.height).toBeGreaterThanOrEqual(44);
        await copy.focus();
        await expect(copy).toBeFocused();
      }
    }
  });
}

test('teacher in another school cannot see the failed-job support reference @smoke @teacher @personalization', async ({ page }) => {
  const schoolAReference = latestFailedJob('b0000000-0000-0000-0000-0000000000a2');
  expect(schoolAReference).toMatch(/^[0-9a-f-]{36}$/);
  await establishSession(page, 'en', 'e2e-teacher-b@example.test');
  await page.goto('/dashboard/assignments');
  const panel = page.getByTestId('teacher-personalization-status');
  await expect(panel).toBeVisible();
  await expect(panel).not.toContainText(/Loading|loading/);
  await expect(panel).not.toContainText(schoolAReference);
  await expect(panel.getByTestId('teacher-personalization-support-reference')).toHaveCount(0);
});

test('teacher status read error is distinct from an administrator job retry @smoke @final @teacher @personalization', async ({ page }) => {
  await establishSession(page, 'en');
  const path = '/api/assignments/personalization_status';
  await page.goto('/dashboard/assignments');
  const panel = page.getByTestId('teacher-personalization-status');
  const refresh = panel.getByRole('button', {name:'Refresh status', exact:true});
  await expect(refresh).toBeVisible();
  // Fail a single status read, not a job command or any other API.
  allowHttpResponse(path, 503);
  const match = (url: URL) => url.pathname === path;
  await page.route(match, route => route.fulfill({status:503, contentType:'text/plain', body:'Synthetic status read failure'}), {times:1});
  await refresh.click();
  await expect(panel.getByRole('alert')).toContainText('Could not load personalization status');
  await expect(panel.getByTestId('teacher-personalization-support-reference')).toHaveCount(0);
  await panel.getByRole('button', {name:'Retry', exact:true}).click();
  await expect(refresh).toBeVisible();
  await expect(panel.getByRole('alert')).toHaveCount(0);
});

function fixtureSql(sql: string): string {
  const databaseUrl = process.env.DATABASE_URL;
  if (!databaseUrl) throw new Error('DATABASE_URL is required for teacher recovery fixture');
  return execFileSync('psql', [databaseUrl, '-v', 'ON_ERROR_STOP=1', '-Atqc', sql], {encoding:'utf8'}).trim();
}

test('admin retry and offline worker completion clear teacher current failure @smoke @final @teacher @platform-admin @personalization', async ({ page }) => {
  const assignment = randomUUID();
  const custom = randomUUID();
  const title = 'E2E Teacher Recovery ' + assignment.slice(0, 8);
  const teacherB = 'e2e-teacher-b@example.test';
  // Create only a synthetic, isolated assignment; never modify shared failure fixtures.
  fixtureSql(`
    BEGIN;
    INSERT INTO assignments (id, teacher_id, class_section_id, subject_id, title, body, due_at, status, published_at)
    VALUES ('${assignment}', 'c0000000-0000-0000-0000-0000000000b2', 'e0000000-0000-0000-0000-0000000000b1', 'd0000000-0000-0000-0000-0000000000a1', '${title}', 'Synthetic recovery fixture', NOW()+INTERVAL '2 days', 'Published', NOW());
    INSERT INTO custom_assignments (id, assignment_id, student_id, due_at, status)
    VALUES ('${custom}', '${assignment}', 'c0000000-0000-0000-0000-0000000000b3', NOW()+INTERVAL '2 days', 'Assigned');
    UPDATE assignment_personalization_jobs SET status='failed', processing_stage='failed', completed_at=NOW(),
      last_error_code='gateway_unavailable', last_error_summary='Synthetic recovery failure'
    WHERE assignment_id='${assignment}';
    COMMIT;
  `);
  try {
    const job = fixtureSql(`SELECT id FROM assignment_personalization_jobs WHERE assignment_id='${assignment}'`);
    expect(job).toMatch(/^[0-9a-f-]{36}$/);
    await establishSession(page, 'en', teacherB);
    await page.goto('/dashboard/assignments');
    const panel = page.getByTestId('teacher-personalization-status');
    await expect(panel).toContainText('1 need attention');
    await panel.getByText('View details', {exact:true}).click();
    await expect(panel.getByTestId('teacher-personalization-support-reference').locator('bdi')).toHaveText(job.slice(0,8)+'…');

    await establishSession(page, 'en', 'e2e-admin@example.test');
    await page.goto('/dashboard/personalization');
    const card = page.locator('article').filter({has:page.getByText(title,{exact:true})});
    await expect(card).toBeVisible();
    const retryPath = '/api/admin/personalization/retry-job';
    // Forward the actual admin command. Freeze only this synthetic fixture
    // after the real successful response, before offline worker completion.
    // The worker remains enabled; its lease cannot complete a replaced claim.
    const retryMatch = (url: URL) => url.pathname === retryPath;
    await page.route(retryMatch, async route => {
      const response = await route.fetch();
      if (response.ok()) {
        fixtureSql(`UPDATE assignment_personalization_jobs
          SET status='queued', processing_stage='queued', available_at=NOW()+INTERVAL '1 day',
              lease_owner=NULL, heartbeat_at=NULL, completed_at=NULL,
              last_error_code=NULL, last_error_summary=NULL
          WHERE id='${job}'`);
      }
      await route.fulfill({response});
    }, {times:1});
    const retryResponse = page.waitForResponse(response => new URL(response.url()).pathname === retryPath);
    await card.getByRole('button', {name:'Retry', exact:true}).click();
    expect((await retryResponse).ok()).toBeTruthy();
    expect(fixtureSql(`SELECT status FROM assignment_personalization_jobs WHERE id='${job}'`)).toBe('queued');

    await establishSession(page, 'en', teacherB);
    await page.goto('/dashboard/assignments');
    await expect(panel.getByTestId('teacher-personalization-support-reference')).toHaveCount(0);
    await expect(panel).toContainText('1 queued');
    // Deterministic offline completion of this fixture, not provider inference.
    fixtureSql(`
      BEGIN;
      UPDATE custom_assignments SET prompt_ctx=jsonb_build_object('personalized_assignment',
        jsonb_build_object('title','${title}','body','Synthetic completed content',
          'scope',jsonb_build_object('type','default','deliverables','[]'::jsonb),
          'estimated_difficulty','medium','personalization_notes','Offline recovery fixture'))
      WHERE id='${custom}';
      UPDATE assignment_personalization_jobs SET status='succeeded', processing_stage='ready',
        completed_at=NOW(), lease_owner=NULL, heartbeat_at=NULL, last_error_code=NULL,last_error_summary=NULL
      WHERE id='${job}';
      COMMIT;
    `);
    await expect(panel).toContainText('0 queued', {timeout:15_000});
    await expect(panel.getByTestId('teacher-personalization-support-reference')).toHaveCount(0);
    await expect(panel).not.toContainText('need attention');
    await panel.getByRole('button',{name:'Refresh status',exact:true}).click();
    await expect(panel).toContainText('0 queued');
    await expect(panel).toContainText('Last completed');
  } finally {
    fixtureSql(`DELETE FROM assignments WHERE id='${assignment}'`);
  }
});
