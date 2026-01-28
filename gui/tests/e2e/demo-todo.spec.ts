import { test, expect } from "@playwright/test";

test("demo todo flow", async ({ page }) => {
  await page.goto("/demo/todo");

  // add two todos
  await page.getByTestId("todo-input").fill("Write docs");
  await page.getByTestId("todo-add").click();
  await page.getByTestId("todo-input").fill("Ship feature");
  await page.getByTestId("todo-add").click();

  const items = page.locator("[data-testid^='todo-checkbox-']");
  await expect(items).toHaveCount(2);
  await expect(page.getByTestId("todo-counter")).toHaveText("2 items");

  // complete first
  await items.nth(0).check();
  await page.getByTestId("todo-clear-completed").click();
  await expect(items).toHaveCount(1);
  await expect(page.getByTestId("todo-counter")).toHaveText("1 item");

  // delete remaining
  const deleteBtn = page.locator("[data-testid^='todo-delete-']").first();
  await deleteBtn.click();
  await expect(items).toHaveCount(0);
  await expect(page.getByTestId("todo-counter")).toHaveText("0 items");
});
