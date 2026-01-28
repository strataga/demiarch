import { test, expect } from "@playwright/test";
test.describe("Playwright Todo", () => {
  test("add, complete, clear, and delete items", async ({ page }) => {
    await page.goto("/");

    // Add two todos
    await page.getByPlaceholder("Add a task...").fill("Write docs");
    await page.getByRole("button", { name: "Add" }).click();
    await page.getByPlaceholder("Add a task...").fill("Ship feature");
    await page.getByRole("button", { name: "Add" }).click();

    const items = page.locator("li");
    await expect(items).toHaveCount(2);
    await expect(page.getByText("2 items")).toBeVisible();

    // Complete the first item
    await items.nth(0).getByRole("checkbox").check();
    await expect(items.nth(0)).toHaveClass(/done/);

    // Clear completed (removes the done item)
    await page.getByRole("button", { name: "Clear completed" }).click();
    await expect(items).toHaveCount(1);
    await expect(page.getByText("1 item")).toBeVisible();

    // Delete the remaining item
    await items.nth(0).getByRole("button", { name: "✕" }).click();
    await expect(items).toHaveCount(0);
    await expect(page.getByText("0 items")).toBeVisible();
  });
});
