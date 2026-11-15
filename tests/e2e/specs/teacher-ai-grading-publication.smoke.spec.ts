import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

// Browser + real PostgreSQL/RLS positive Teacher decision boundary. The
// suggestion is a synthetic DB fixture, not a real provider result, and no
// inference, PDF dispatch or production school opt-in is enabled.
type Requests = Pick<APIRequestContext, 'post'>;
const origin = new URL(process.env.E2E_BASE_URL ?? 'http://127.0.0.1:8080');

function api(page: Page): Requests {
  return {
    post: async (path, options: any = {}) => {
      const url = new URL(path, origin);
      const headers = { ...options.headers };
      if (url.origin === origin.origin && origin.protocol === 'http:'
          && ['localhost', '127.0.0.1'].includes(origin.hostname)) {
        headers.cookie = (await page.context().cookies())
          .filter(cookie => cookie.domain === origin.hostname && url.pathname.startsWith(cookie.path))
          .map(cookie => `${cookie.name}=${cookie.value}`).join('; ');
      }
      return page.context().request.post(path, { ...options, headers, maxRedirects: 0 });
    },
  };
}
async function login(page: Page, email: string) {
  const response = await api(page).post('/api/auth/login', {
    data: { email, password: 'e2e-password' },
  });
  expect(response.ok(), `login failed for ${email}: ${response.status()}`).toBeTruthy();
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});
test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('teacher alone publishes synthetic AI suggestion; linked Parent and Student observe only accepted grades @smoke @final @teacher @student @parent @ai-grading @security', async ({ page }, info) => {
  test.setTimeout(120_000);
  const mobile = info.project.name.includes('mobile');
  const locale = mobile ? 'fa' : 'en';
  const suffix = mobile ? '601' : '501';
  const action = mobile ? 'reject' : 'accept';
  const customAssignment = `f1600000-0000-0000-0000-000000000${suffix}`;
  const assignmentTitle = `E2E AI Review ${suffix}`;
  await page.addInitScript(selected => {
    localStorage.setItem('edutalent_locale', selected);
  }, locale);

  // No pending AI artifact is an official grade, even with an approved rubric.
  await login(page, 'e2e-attachment-student@example.test');
  let answer = await api(page).post('/api/submissions/get_for_assignment', {
    data: { assignment_id: customAssignment },
  });
  expect(answer.ok()).toBeTruthy();
  expect((await answer.json()).grade).toBeNull();

  await login(page, 'e2e-attachment-parent@example.test');
  await page.goto('/dashboard/children');
  let child = page.getByText('E2E Attachment Student', { exact: true })
    .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
  await expect(child).toBeVisible();
  let gradeResponsePromise = page.waitForResponse(response =>
    new URL(response.url()).pathname === '/api/parent/scoped/child/grades'
  );
  await child.getByRole('button', {
    name: mobile ? 'مشاهده نمرات' : 'View Grades', exact: true,
  }).click();
  let parentGrades = await gradeResponsePromise;
  expect(parentGrades.ok()).toBeTruthy();
  expect((await parentGrades.json()).filter((row: { assignment_title: string }) =>
    row.assignment_title === assignmentTitle
  )).toHaveLength(0);
  await page.keyboard.press('Escape');

  // Teacher sees only an immutable rubric-bound suggestion. This click uses
  // the normal browser decision endpoint and real database permission checks.
  await login(page, 'e2e-attachment-teacher@example.test');
  await page.goto('/dashboard/submissions');
  const card = page.getByText(assignmentTitle, { exact: true }).first()
    .locator('xpath=ancestor::div[contains(@class, "overflow-hidden")][1]');
  await expect(card).toBeVisible();
  await card.getByRole('button').last().click();
  const dialog = page.getByRole('dialog');
  await expect(dialog).toBeVisible();
  const panel = dialog.locator('section').filter({
    hasText: mobile ? 'دستیار تصحیح هوشمند' : 'AI grading assistant',
  }).last();
  await expect(panel).toContainText('Synthetic AI model evidence: teacher review required.');
  await expect(panel).toContainText('18.00 / 20');
  await expect(panel.getByRole('button', {
    name: mobile ? 'تأیید و انتشار نمره' : 'Accept & publish grade',
  })).toBeVisible();

  const reviewResponsePromise = page.waitForResponse(response =>
    new URL(response.url()).pathname === '/api/ai-grading/suggestions/review'
  );
  await panel.getByRole('button', {
    name: action === 'accept'
      ? 'Accept & publish grade'
      : 'رد پیشنهاد',
  }).click();
  const review = await reviewResponsePromise;
  expect(review.ok(), `Teacher ${action} failed: ${review.status()}`).toBeTruthy();
  expect(await review.json()).toBe(true);

  await login(page, 'e2e-attachment-student@example.test');
  answer = await api(page).post('/api/submissions/get_for_assignment', {
    data: { assignment_id: customAssignment },
  });
  expect(answer.ok()).toBeTruthy();
  const student = await answer.json();
  if (action === 'accept') {
    expect(student.grade).toBe(18);
    expect(student.grade_scale).toBe(20);
  } else {
    expect(student.grade).toBeNull();
  }

  await login(page, 'e2e-attachment-parent@example.test');
  await page.goto('/dashboard/children');
  child = page.getByText('E2E Attachment Student', { exact: true })
    .locator('xpath=ancestor::div[contains(@class, "et-ui-card")][1]');
  await expect(child).toBeVisible();
  gradeResponsePromise = page.waitForResponse(response =>
    new URL(response.url()).pathname === '/api/parent/scoped/child/grades'
  );
  await child.getByRole('button', {
    name: mobile ? 'مشاهده نمرات' : 'View Grades', exact: true,
  }).click();
  parentGrades = await gradeResponsePromise;
  expect(parentGrades.ok()).toBeTruthy();
  const published = (await parentGrades.json()).filter(
    (row: { assignment_title: string }) => row.assignment_title === assignmentTitle
  );
  if (action === 'accept') {
    expect(published).toHaveLength(1);
    expect(published[0].points).toBe('18/20');
    await expect(page.getByRole('dialog')).toContainText(assignmentTitle);
    await expect(page.getByRole('dialog')).toContainText('18/20');
  } else {
    expect(published).toHaveLength(0);
    await expect(page.getByRole('dialog')).not.toContainText(assignmentTitle);
  }
});
