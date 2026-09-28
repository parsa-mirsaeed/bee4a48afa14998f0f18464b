import { expect, type Page } from '@playwright/test';

// Role identity belongs to the sidebar. A closed responsive drawer must be
// hidden from users and assistive technology; inspect it through its controls.
export async function expectSidebarRole(page: Page, role: string): Promise<void> {
  const menu = page.locator('.et-mobile-menu-button');
  const sidebar = page.locator('.et-sidebar');
  const responsive = await menu.isVisible();
  if (responsive) {
    await expect(sidebar).toBeHidden();
    await menu.click();
  }
  await expect(sidebar.locator('.et-user-role')).toHaveText(role);
  await expect(sidebar.locator('.et-user-role')).toBeVisible();
  if (responsive) {
    await sidebar.locator('.et-sidebar-mobile-close').click();
    await expect(sidebar).toBeHidden();
    await expect(menu).toBeFocused();
  }
}
