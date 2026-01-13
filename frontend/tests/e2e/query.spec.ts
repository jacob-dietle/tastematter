import { test, expect } from '@playwright/test';

test.describe('Query Flow', () => {
  test('app loads and shows time selector', async ({ page }) => {
    await page.goto('/');

    // Header with title should be visible
    await expect(page.locator('h1')).toContainText('Tastematter');

    // Time selector buttons should be visible
    await expect(page.getByRole('button', { name: '7d' })).toBeVisible();
    await expect(page.getByRole('button', { name: '30d' })).toBeVisible();
    await expect(page.getByRole('button', { name: '90d' })).toBeVisible();
  });

  test('clicking time button changes selection', async ({ page }) => {
    await page.goto('/');

    const button30d = page.getByRole('button', { name: '30d' });
    await button30d.click();

    // Should have selected class
    await expect(button30d).toHaveClass(/selected/);
  });

  test('shows loading or error state after query', async ({ page }) => {
    await page.goto('/');

    // After load, should show either results, error, or loading
    // (depends on whether CLI is available)
    const content = page.locator('section.content');
    await expect(content).toBeVisible();
  });
});
