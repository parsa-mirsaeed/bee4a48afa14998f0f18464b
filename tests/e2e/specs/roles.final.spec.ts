// @final @roles @workflows — PR-12 final role and persisted core-workflow evidence.
//
// The production release deliberately disables unfinished attendance, timetable,
// reports, messaging, derived metrics, and synthetic-health domains. These
// journeys cover only enabled core school workflows against the deterministic
// two-school fixture and the real server-backed UI.
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const roles = [
  { name: 'platform admin', email: 'e2e-admin@example.test', alias: '/dashboard/platform-admin' },
  { name: 'school manager', email: 'e2e-manager-a@example.test', alias: '/dashboard/school-manager' },
  { name: 'teacher', email: 'e2e-teacher-a@example.test', alias: '/dashboard/teacher' },
  { name: 'student', email: 'e2e-student-a@example.test', alias: '/dashboard/student' },
  { name: 'parent', email: 'e2e-parent-a@example.test', alias: '/dashboard/parent' },
] as const;

async function signIn(page: Page, email: string): Promise<void> {
  await page.goto('/');
  await page.locator('input[type="email"]').fill(email);
  await page.locator('input[type="password"]').fill(PASSWORD);
  await page.getByRole('button', { name: /sign in|ورود/i }).click();
  await expect(page).toHaveURL(/\/dashboard$/);
}

async function signInEnglish(page: Page, email: string): Promise<void> {
  await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'en'));
  await signIn(page, email);
}

async function endSessionForRoleSwitch(page: Page): Promise<void> {
  // Logout/session termination has its own Tier-1 browser journey. This final
  // stateful workflow uses the real logout endpoint only to switch fixture
  // actors, avoiding coupling the student→teacher→student chain to whether the
  // responsive shell renders logout inline or behind its mobile profile menu.
  const response = await page.context().request.post('/api/auth/logout');
  expect(response.ok(), 'role-switch logout endpoint must succeed').toBeTruthy();
  await page.goto('/');
  await expect(page).toHaveURL(/\/$/);
}

async function establishSession(page: Page, email: string): Promise<void> {
  // Direct-route acceptance must begin from a valid authenticated browser
  // context, not by tearing down a live hydrated Dioxus document. The browser
  // context request client shares its cookie jar with pages in the context, so
  // the role alias below is the first authenticated document navigation.
  const response = await page.context().request.post('/api/auth/login', {
    data: { email, password: PASSWORD },
  });
  expect(response.ok(), `session setup failed for ${email}`).toBeTruthy();
}

async function navigateWithIcon(page: Page, icon: string): Promise<void> {
  const mobileMenu = page.locator('.et-mobile-menu-button');
  if (await mobileMenu.isVisible()) {
    await mobileMenu.click();
    await expect(page.locator('.et-sidebar')).toHaveClass(/et-sidebar--mobile-open/);
  }

  const item = page.locator('.et-ui-sidebar-nav__item', {
    has: page.locator('span.material-icons-outlined', {
      hasText: new RegExp(`^${icon}$`),
    }),
  }).first();
  await expect(item).toBeVisible();
  await item.click();
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

for (const role of roles) {
  test(`${role.name} reaches the canonical dashboard and role alias @final @roles`, async ({ page }) => {
    await establishSession(page, role.email);
    const response = await page.goto(role.alias);
    expect(response === null || response.status() < 400).toBeTruthy();
    await expect(page).toHaveURL(new RegExp(`${role.alias.replaceAll('/', '\\/')}$`));
  });
}

test('authenticated dashboard supports English/LTR @final @roles', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await expect.poll(() => page.evaluate(() => document.documentElement.lang)).toMatch(/^en/i);
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');
});

test('school manager reads only the authorized school user directory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await navigateWithIcon(page, 'groups');

  await expect(page.getByText('E2E Teacher A', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Student A', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Teacher B', { exact: true })).toHaveCount(0);
  await expect(page.getByText('E2E Student B', { exact: true })).toHaveCount(0);
});

test('school manager reads only the authorized class inventory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await navigateWithIcon(page, 'class');

  await expect(page.getByText('E2E Class A1', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Class B1', { exact: true })).toHaveCount(0);
});

test('school manager sees the governed school knowledge inventory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-manager-a@example.test');
  await page.goto('/dashboard/knowledge-submissions');
  await expect(page).toHaveURL(/\/dashboard\/knowledge-submissions$/);

  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});

test('teacher sees the persisted published assignment and governed knowledge asset @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-teacher-a@example.test');

  await page.goto('/dashboard/assignments');
  await expect(page).toHaveURL(/\/dashboard\/assignments$/);
  await expect(page.getByText('E2E Assignment A1', { exact: true })).toBeVisible();

  await page.goto('/dashboard/knowledge-assets');
  await expect(page).toHaveURL(/\/dashboard\/knowledge-assets$/);
  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});


for (const [locale, materialsLabel, emptyResources] of [
  ['en', 'Materials', 'No Materials Yet'],
  ['fa', 'منابع', 'منبعی وجود ندارد'],
] as const) {
  test(`teacher class resources omit internal migration copy in ${locale} @smoke @final @teacher @i18n`, async ({ page }) => {
    await page.addInitScript((selectedLocale) => localStorage.setItem('edutalent_locale', selectedLocale), locale);
    await signIn(page, 'e2e-teacher-a@example.test');
    await page.goto('/dashboard/classes');

    const classCard = page.getByText('E2E Class A1', { exact: true })
      .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
    await expect(classCard).toBeVisible();
    await classCard.getByRole('button', { name: materialsLabel, exact: true }).click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toContainText(emptyResources);
    await expect(dialog).not.toContainText('Teacher file uploads are retired');
    await expect(dialog).not.toContainText('Governed knowledge assets');
  });
}

test('student sees only persisted enrollment and assignment state @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-student-a@example.test');

  await expect(page.getByText('E2E Class A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Assignment A1', { exact: true }).first()).toBeVisible();
  await expect(page.getByText('E2E Class B1', { exact: true })).toHaveCount(0);
});

test('student submission is graded by the authorized teacher and appears in persisted grades @final @workflows', async ({ page }, testInfo) => {
  const assignmentTitle = testInfo.project.name === 'mobile-chromium'
    ? 'E2E Submission Journey Mobile'
    : 'E2E Submission Journey Desktop';
  const submittedWork = `Persisted ${testInfo.project.name} submission`;
  const feedback = `Verified ${testInfo.project.name} feedback`;

  // Student performs the contracted submission workflow against the real server.
  await signInEnglish(page, 'e2e-student-a@example.test');
  await navigateWithIcon(page, 'assignment');
  const assignmentCard = page
    .getByText(assignmentTitle, { exact: true })
    .locator('xpath=ancestor::article[1]');
  await expect(assignmentCard).toBeVisible();
  await assignmentCard.getByRole('button', { name: 'Start assignment', exact: true }).click();

  const detailsDialog = page.getByRole('dialog');
  await expect(detailsDialog).toContainText(assignmentTitle);
  await detailsDialog.getByRole('button', { name: 'Open my submission', exact: true }).click();

  const workEditor = page.getByRole('dialog').locator('textarea');
  await expect(workEditor).toBeVisible();
  await workEditor.fill(submittedWork);
  await page.getByRole('dialog').getByRole('button', { name: 'Submit work', exact: true }).click();
  await expect(assignmentCard).toContainText('Submitted');
  await endSessionForRoleSwitch(page);

  // The School A teacher sees that submission, records a grade, and persists feedback.
  await signInEnglish(page, 'e2e-teacher-a@example.test');
  await navigateWithIcon(page, 'grading');
  await expect(page.getByText(assignmentTitle, { exact: true })).toBeVisible();
  await expect(page.getByText(submittedWork, { exact: true })).toBeVisible();
  await expect(page.getByText('submissions.grade_btn', { exact: true })).toHaveCount(0);

  const submissionCard = page
    .getByText(assignmentTitle, { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"rounded-xl")][.//button[.//span[normalize-space()="grading"]]][1]');
  await submissionCard.evaluate((element) => element.scrollIntoView({ block: 'center', inline: 'nearest' }));
  await submissionCard.locator('xpath=.//button[.//span[normalize-space()="grading"]]').click();

  const gradingDialog = page.getByRole('dialog');
  await expect(gradingDialog).toBeVisible();
  const labelledBy = await gradingDialog.getAttribute('aria-labelledby');
  expect(labelledBy).toMatch(/^et-dialog-title-/);
  await expect(gradingDialog.locator(`#${labelledBy}`)).toContainText('Grade Submission');
  await gradingDialog.locator('input[type="number"]').fill('91');
  await gradingDialog.locator('textarea').fill(feedback);
  await gradingDialog.getByRole('button', { name: /Save Grade$/ }).click();
  await expect(gradingDialog).toHaveCount(0);
  await expect(page.getByText(assignmentTitle, { exact: true })).toHaveCount(0);
  await endSessionForRoleSwitch(page);

  // The student reads the persisted grade from the production grade view.
  await signInEnglish(page, 'e2e-student-a@example.test');
  await navigateWithIcon(page, 'grade');
  const classCard = page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"et-ui-card")][1]');
  await classCard.getByRole('button', { name: 'View Details', exact: true }).click();

  const gradesDialog = page.getByRole('dialog');
  await expect(gradesDialog).toContainText(assignmentTitle);
  await expect(gradesDialog).toContainText('91/100');
});

test('parent sees only the authorized child enrollment @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-parent-a@example.test');

  await page.goto('/dashboard/children');
  await expect(page).toHaveURL(/\/dashboard\/children$/);
  await expect(page.getByText('E2E Student A', { exact: true })).toBeVisible();
  await expect(page.getByText('E2E Student B', { exact: true })).toHaveCount(0);
});

test('student and parent see the same persisted twenty-point grade @smoke @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-student-a@example.test');
  await navigateWithIcon(page, 'grade');
  const classCard = page
    .getByText('E2E Class A1', { exact: true })
    .locator('xpath=ancestor::div[contains(@class,"et-ui-card")][1]');
  await classCard.getByRole('button', { name: 'View Details', exact: true }).click();
  const studentGrades = page.getByRole('dialog');
  await expect(studentGrades.getByText('E2E Assignment A1', { exact: true })).toBeVisible();
  await expect(studentGrades).toContainText('A-');
  await expect(studentGrades).toContainText('18/20');
  await endSessionForRoleSwitch(page);

  await establishSession(page, 'e2e-parent-a@example.test');
  await page.goto('/dashboard/children');
  await page.getByRole('button', { name: 'View Grades', exact: true }).click();
  const parentGrades = page.getByRole('dialog');
  await expect(parentGrades.getByText('E2E Assignment A1', { exact: true })).toBeVisible();
  await expect(parentGrades).toContainText('A-');
  await expect(parentGrades).toContainText('18/20');
  await expect(parentGrades).not.toContainText('18/100');
});

test('platform admin sees the governed published asset inventory @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-admin@example.test');

  await expect(page.getByText('E2E Published Asset', { exact: true })).toBeVisible();
});

test('platform admin updates verified OCR in an accessible prefilled dialog @final @workflows', async ({ page }) => {
  await signInEnglish(page, 'e2e-admin@example.test');

  const card = page
    .getByText('E2E Verified OCR Asset', { exact: true })
    .locator('xpath=ancestor::article[1]');
  const trigger = card.getByRole('button', { name: 'Update verified OCR', exact: true });
  await trigger.click();

  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  await expect(dialog).toHaveAccessibleName(/update verified ocr.*e2e verified ocr asset/i);
  await expect(dialog.getByLabel('Verified source text')).toHaveValue('E2E preverified OCR text');
  await expect(dialog).toBeFocused();

  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
});
