// @smoke @student @sky — Study Sky overview hero.
//
// Invariant-based on purpose: other specs in this suite mutate the shared
// student fixture (submissions, grading), so nothing here hard-codes how many
// assignments exist or which state they are in. It proves that the sky is a
// faithful, deterministic picture of the same data the assignment list shows,
// that its motion obeys the contract, and that it is accessible and RTL-safe.
import AxeBuilder from '@axe-core/playwright';
import { test, expect, type Page } from '@playwright/test';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors } from '../fixtures/console-guard';

const PASSWORD = 'e2e-password';
const STUDENT = 'e2e-student-a@example.test';
const STAR_CAP = 18;
const ALLOWED_ANIMATED_PROPERTIES = new Set([
  'transform',
  'opacity',
  'strokeDashoffset',
  'offset',
  'computedOffset',
  'easing',
  'composite',
]);

async function openOverview(page: Page, locale: 'en' | 'fa'): Promise<void> {
  await page.addInitScript((selected) => localStorage.setItem('edutalent_locale', selected), locale);
  const login = await page.context().request.post('/api/auth/login', {
    data: { email: STUDENT, password: PASSWORD },
  });
  expect(login.ok(), 'student session setup failed').toBeTruthy();
  const response = await page.goto('/dashboard');
  expect(response === null || response.status() < 400).toBeTruthy();
  await expect(page.locator('.ss-hero')).toBeVisible();
  await expect(page.locator('.ss-star').first()).toBeVisible();
}

async function settleAnimations(page: Page): Promise<void> {
  await page.evaluate(async () => {
    const hero = document.querySelector('.ss-hero');
    if (!hero) return;
    await Promise.all(hero.getAnimations({ subtree: true }).map((animation) => animation.finished));
  });
}

async function legendCounts(page: Page) {
  const read = async (modifier: string): Promise<number> => {
    const item = page.locator('.ss-legend__item', {
      has: page.locator(`.ss-swatch--${modifier}`),
    });
    return Number(await item.locator('.ss-legend__count').innerText());
  };
  const pending = await read('pending');
  const overdue = await read('overdue');
  const submitted = await read('submitted');
  const graded = await read('graded');
  return {
    pending,
    overdue,
    submitted,
    graded,
    total: pending + overdue + submitted + graded,
    lit: submitted + graded,
  };
}

test.beforeEach(async ({ page }) => {
  await enforceOfflineAllowlist(page);
  watchConsole(page);
});

test.afterEach(() => {
  assertNoUnexpectedOrigins();
  assertNoConsoleErrors();
});

test('sky is a faithful picture of the student assignment data @smoke @student @sky', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openOverview(page, 'en');
  const counts = await legendCounts(page);
  expect(counts.total).toBeGreaterThan(0);

  // One star per assignment, capped.
  await expect(page.locator('.ss-star')).toHaveCount(Math.min(counts.total, STAR_CAP));
  if (counts.total <= STAR_CAP) {
    await expect(page.locator('.ss-star--pending')).toHaveCount(counts.pending);
    await expect(page.locator('.ss-star--overdue')).toHaveCount(counts.overdue);
    await expect(page.locator('.ss-star--submitted')).toHaveCount(counts.submitted);
    await expect(page.locator('.ss-star--graded')).toHaveCount(counts.graded);
  }

  // The numeric summary, the progressbar and the stars tell the same story.
  await expect(page.locator('.ss-hero__stat-value')).toHaveText(`${counts.lit} / ${counts.total}`);
  const bar = page.getByRole('progressbar', { name: 'Assignment submission progress' });
  await expect(bar).toHaveAttribute('aria-valuenow', String(Math.floor((counts.lit * 100) / counts.total)));

  // Decorative scene is hidden from assistive tech and carries no assignment text.
  await expect(page.locator('.ss-scene')).toHaveAttribute('aria-hidden', 'true');
  await expect(page.locator('.ss-scene')).toHaveText('');
});

test('up next and the upcoming list come from one queue, so counts cannot disagree @smoke @student @sky', async ({ page }) => {
  await openOverview(page, 'en');
  const counts = await legendCounts(page);
  const toDo = counts.pending + counts.overdue;

  const rows = page.locator('.ss-queue .et-list-row');
  const next = page.locator('.ss-next');
  if (toDo === 0) {
    await expect(next).toHaveCount(0);
    await expect(page.locator('.ss-queue')).toHaveCount(0);
    return;
  }
  await expect(rows).toHaveCount(Math.min(toDo, 5));
  // "Up next" is the first overdue item, else the first pending item: exactly
  // the first row of the list. Finished work never appears in either place.
  const firstTitle = (await rows.first().locator('.et-list-title').innerText()).trim();
  await expect(next.locator('.ss-next__title')).toHaveText(firstTitle);
  await expect(next).toHaveClass(counts.overdue > 0 ? /ss-next--overdue/ : /ss-next--pending/);
  await expect(page.locator('.ss-hero')).not.toContainText('E2E Assignment A1');
});

test('motion contract: bounded properties, nothing loops, reduced motion removes it @smoke @student @sky @accessibility', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openOverview(page, 'en');

  const audit = await page.evaluate(() => {
    const hero = document.querySelector('.ss-hero')!;
    const animations = hero.getAnimations({ subtree: true });
    const properties = new Set<string>();
    let infinite = 0;
    for (const animation of animations) {
      const effect = animation.effect as KeyframeEffect | null;
      if (!effect) continue;
      if (effect.getComputedTiming().iterations === Infinity) infinite += 1;
      for (const frame of effect.getKeyframes()) {
        Object.keys(frame).forEach((key) => properties.add(key));
      }
    }
    return { infinite, properties: [...properties] };
  });
  expect(audit.infinite, 'no animation inside the hero may loop').toBe(0);
  for (const property of audit.properties) {
    expect(ALLOWED_ANIMATED_PROPERTIES.has(property), `unexpected animated property ${property}`).toBeTruthy();
  }

  // Defaults are the final state: once settled, every star is fully opaque.
  await settleAnimations(page);
  const opacities = await page.locator('.ss-star').evaluateAll((stars) =>
    stars.map((star) => getComputedStyle(star).opacity),
  );
  expect(opacities.every((value) => value === '1')).toBeTruthy();

  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.reload();
  await expect(page.locator('.ss-star').first()).toBeVisible();
  const running = await page.evaluate(
    () => document.querySelector('.ss-hero')!.getAnimations({ subtree: true }).length,
  );
  expect(running, 'reduced motion must leave no running animations in the hero').toBe(0);
});

test('depth is real on wide screens and flat on mobile @smoke @student @sky @responsive', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openOverview(page, 'en');
  await expect(page.locator('.ss-layer')).toHaveCount(3);
  await expect(page.locator('.ss-world')).toHaveCSS('transform-style', 'preserve-3d');
  await expect(page.locator('.ss-scene')).not.toHaveCSS('perspective', 'none');

  await page.setViewportSize({ width: 390, height: 844 });
  await expect(page.locator('.ss-scene')).toHaveCSS('perspective', 'none');
  await expect(page.locator('.ss-world')).toHaveCSS('transform-style', 'flat');
  for (const layer of ['far', 'mid', 'near']) {
    await expect(page.locator(`.ss-layer--${layer}`)).toHaveCSS('transform', 'none');
  }
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > window.innerWidth + 1);
  expect(overflow, 'mobile sky must not cause sideways page scroll').toBeFalsy();
});

test('English layout runs start to end and Persian mirrors it @smoke @student @sky @rtl', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openOverview(page, 'en');
  await settleAnimations(page);
  const readX = () => page.locator('.ss-star').evaluateAll((stars) => stars.map((star) => star.getBoundingClientRect().x));
  const ltr = await readX();
  if (ltr.length > 1) {
    expect(ltr.every((x, index) => index === 0 || x > ltr[index - 1]), 'LTR stars advance left to right').toBeTruthy();
  }
  await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('ltr');

  const fa = await page.context().newPage();
  await enforceOfflineAllowlist(fa);
  await fa.addInitScript(() => localStorage.setItem('edutalent_locale', 'fa'));
  await fa.setViewportSize({ width: 1440, height: 900 });
  await fa.goto('/dashboard');
  await expect(fa.locator('.ss-star').first()).toBeVisible();
  await expect.poll(() => fa.evaluate(() => document.documentElement.dir)).toBe('rtl');
  await settleAnimations(fa);
  const rtl = await fa.locator('.ss-star').evaluateAll((stars) => stars.map((star) => star.getBoundingClientRect().x));
  if (rtl.length > 1) {
    expect(rtl.every((x, index) => index === 0 || x < rtl[index - 1]), 'RTL stars advance right to left').toBeTruthy();
  }
  await expect(fa.locator('.ss-hero')).toContainText('آسمان مطالعه');
  await expect(fa.locator('.ss-hero')).not.toContainText(/\b(?:Pending|Overdue|Submitted|Graded)\b/);
  await expect(fa.locator('.ss-hero')).not.toContainText(/\b(?:student|play)\.[a-z0-9_.]+\b/);
  await fa.close();
});

test('hero has no automated WCAG A/AA violations in English and Persian @smoke @student @sky @accessibility', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  for (const locale of ['en', 'fa'] as const) {
    if (locale === 'fa') {
      await page.addInitScript(() => localStorage.setItem('edutalent_locale', 'fa'));
      await page.reload();
      await expect(page.locator('.ss-star').first()).toBeVisible();
      await expect.poll(() => page.evaluate(() => document.documentElement.dir)).toBe('rtl');
    } else {
      await openOverview(page, 'en');
    }
    await settleAnimations(page);
    const result = await new AxeBuilder({ page })
      .include('.ss-hero')
      .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
      .analyze();
    expect(
      result.violations,
      result.violations.map((violation) => `${locale} ${violation.id}: ${violation.help}`).join('\n'),
    ).toEqual([]);
  }
});

test('hero call to action is keyboard operable and keeps its accessible name @smoke @student @sky @accessibility', async ({ page }) => {
  await openOverview(page, 'en');
  const cta = page.getByRole('button', { name: 'Explore assignments' });
  await cta.focus();
  await expect(cta).toBeFocused();
  const outline = await cta.evaluate((element) => getComputedStyle(element).outlineStyle);
  expect(outline, 'focus indicator must be visible on the dark sky').not.toBe('none');
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/dashboard\/assignments$/);
});
