import { test, expect, type Page, type Locator } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const LINKED_PARENT = 'e2e-parent-a@example.test';
const EMPTY_PARENT = 'e2e-parent-empty@example.test';

async function openParentRoute(
  page: Page,
  email: string,
  locale: 'en' | 'fa',
  path: '/dashboard' | '/dashboard/children',
): Promise<void> {
  await page.addInitScript((selectedLocale) => {
    localStorage.setItem('edutalent_locale', selectedLocale);
  }, locale);

  const login = await page.context().request.post('/api/auth/login', {
    data: { email, password: PASSWORD },
  });
  expect(login.ok(), `session setup failed for ${email}`).toBeTruthy();

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

async function expectFinishedParentChrome(body: Locator): Promise<void> {
  await expect(body).not.toContainText('not enabled in this release');
  await expect(body).not.toContainText('Incomplete capabilities');
  await expect(body).not.toContainText(/parent\.[a-z0-9_.]+/i);
  await expect(body).not.toContainText(/T\d{2}:\d{2}:\d{2}/);
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
    dir: 'ltr',
    role: 'Parent',
    intro: 'Review your linked children, their classes, assignments, and recorded grades.',
    gradeMissing: 'Grade level not recorded',
    classCount: '1 class',
    viewGrades: 'View Grades',
    assignments: 'Assignments',
    graded: 'Graded',
    emptyTitle: 'No student is linked to this parent account yet',
    emptyDescription: 'School administration must link a student before academic information appears.',
  },
  {
    locale: 'fa' as const,
    dir: 'rtl',
    role: 'والد',
    intro: 'فرزندان متصل، کلاس‌ها، تکلیف‌ها و نمره‌های ثبت‌شده آن‌ها را مرور کنید.',
    gradeMissing: 'پایه تحصیلی ثبت نشده است',
    classCount: '1 کلاس',
    viewGrades: 'مشاهده نمرات',
    assignments: 'تکالیف',
    graded: 'نمره‌گذاری‌شده',
    emptyTitle: 'هنوز دانش‌آموزی به این حساب والد متصل نشده است',
    emptyDescription: 'مدیریت مدرسه باید یک دانش‌آموز را به این حساب متصل کند تا اطلاعات تحصیلی نمایش داده شود.',
  },
]) {
  test(`parent overview is finished EN/FA product chrome in ${scenario.locale} @smoke @final @parent @i18n @workflow-truth`, async ({ page }) => {
    await openParentRoute(page, LINKED_PARENT, scenario.locale, '/dashboard');
    const body = page.locator('body');

    await expect(page.getByText('E2E Student A', { exact: true })).toBeVisible();
    await expect(page.getByText(scenario.role, { exact: true }).first()).toBeVisible();
    await expect(page.getByText(scenario.intro, { exact: true })).toBeVisible();
    await expect(page.getByText(scenario.gradeMissing, { exact: true })).toBeVisible();
    await expect(page.getByText(scenario.classCount, { exact: true })).toBeVisible();
    await expectFinishedParentChrome(body);

    if (scenario.locale === 'fa') {
      await expect(body).not.toContainText('Grade not recorded');
      await expect(body).not.toContainText('Grade level not recorded');
    }
  });

  test(`parent children grades and assignments are localized in ${scenario.locale} @smoke @final @parent @i18n @workflow-truth`, async ({ page }) => {
    await openParentRoute(page, LINKED_PARENT, scenario.locale, '/dashboard/children');
    const body = page.locator('body');
    const childCard = page
      .getByText('E2E Student A', { exact: true })
      .locator('xpath=ancestor::div[contains(@class,"et-ui-card")][1]');

    await expect(childCard).toBeVisible();
    await expect(childCard).toContainText(scenario.gradeMissing);
    await expect(childCard.getByRole('button', { name: scenario.viewGrades, exact: true })).toBeVisible();
    await expect(childCard.getByRole('button', { name: scenario.assignments, exact: true })).toBeVisible();
    await expectFinishedParentChrome(body);

    const gradeResponsePromise = page.waitForResponse((response) =>
      new URL(response.url()).pathname === '/api/parent/scoped/child/grades',
    );
    await childCard.getByRole('button', { name: scenario.viewGrades, exact: true }).click();
    const gradeResponse = await gradeResponsePromise;
    expect(gradeResponse.ok()).toBeTruthy();
    const gradeRows = await gradeResponse.json();
    expect(Array.isArray(gradeRows)).toBeTruthy();
    const persistedGrade = gradeRows.find((row: { assignment_title: string }) =>
      row.assignment_title === 'E2E Assignment A1',
    );
    expect(persistedGrade).toMatchObject({ grade: 'A-', points: '18/20' });
    let dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('E2E Assignment A1');
    await expect(dialog).toContainText('E2E Class A1');
    await expect(dialog).toContainText('A-');
    await expect(dialog).toContainText('18/20');
    await expect(dialog.locator('bdi[dir="ltr"]', { hasText: '18/20' })).toBeVisible();
    await expect(dialog).not.toContainText(/T\d{2}:\d{2}:\d{2}/);
    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);

    const assignmentResponsePromise = page.waitForResponse((response) =>
      new URL(response.url()).pathname === '/api/parent/scoped/child/assignments',
    );
    await childCard.getByRole('button', { name: scenario.assignments, exact: true }).click();
    const assignmentResponse = await assignmentResponsePromise;
    expect(assignmentResponse.ok()).toBeTruthy();
    const assignmentRows = await assignmentResponse.json();
    expect(Array.isArray(assignmentRows)).toBeTruthy();
    const persistedAssignment = assignmentRows.find((row: { title: string }) =>
      row.title === 'E2E Assignment A1',
    );
    // The assignments API carries a grade even though this card currently only
    // renders status. Both authorized reads must describe the same persisted result.
    expect(persistedAssignment).toMatchObject({ status: 'Graded', grade: persistedGrade.grade });
    const request = assignmentResponse.request();
    const originalBody = request.postData();
    expect(originalBody).toContain('c0000000-0000-0000-0000-0000000000a3');
    const denied = await page.context().request.post(request.url(), {
      headers: { 'content-type': request.headers()['content-type'] },
      data: originalBody!.replace(
        'c0000000-0000-0000-0000-0000000000a3',
        'c0000000-0000-0000-0000-0000000000b3',
      ),
    });
    expect(denied.ok(), 'Parent A must not read School B child assignments').toBeFalsy();
    expect(await denied.text()).not.toContain('E2E Authorization Submission B');
    dialog = page.getByRole('dialog');
    await expect(dialog).toContainText('E2E Assignment A1');
    await expect(dialog).toContainText('E2E Class A1');
    await expect(dialog).toContainText(scenario.graded);
    await expect(dialog).not.toContainText(/T\d{2}:\d{2}:\d{2}/);
    await expect(dialog).not.toContainText(/parent\.[a-z0-9_.]+/i);
    if (scenario.locale === 'fa') {
      await expect(dialog).not.toContainText(/\b(?:Assigned|Pending|Overdue|Submitted|Graded)\b/);
      await expect(dialog).not.toContainText(/\b(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\b/);
    }
  });

  test(`parent no-child state is localized in ${scenario.locale} @smoke @final @parent @i18n @workflow-truth`, async ({ page }) => {
    await openParentRoute(page, EMPTY_PARENT, scenario.locale, '/dashboard/children');
    const body = page.locator('body');

    await expect(page.getByText(scenario.emptyTitle, { exact: true })).toBeVisible();
    await expect(page.getByText(scenario.emptyDescription, { exact: true })).toBeVisible();
    await expectFinishedParentChrome(body);
  });
}
