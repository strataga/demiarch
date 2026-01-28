import { test, expect } from "@playwright/test";

const MOCK_PRD = `# Product Requirements Document: Demo Todo

## Executive Summary
Simple todo list app for testing Playwright flows.`;

test("create project via UI new project flow", async ({ page }) => {
  // Stub OpenRouter to return a PRD immediately
  await page.route("https://openrouter.ai/api/v1/chat/completions", async (route) => {
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        choices: [{ message: { content: MOCK_PRD } }],
      }),
    });
  });

  // Seed a mock API key so UI thinks we're configured
  await page.addInitScript(() => {
    localStorage.setItem("openrouter_api_key", "test-key");
    localStorage.setItem("openrouter_model", "mock-model");
    localStorage.removeItem("demiarch_projects");
  });

  await page.goto("/projects");

  // Open modal
  await page.getByRole("button", { name: "New Project" }).click();
  await expect(page.getByRole("heading", { name: "New Project" })).toBeVisible();

  // Provide description and send (Enter triggers handleSend)
  await page.getByPlaceholder("Type your response...").fill("Build a todo list app for testing");
  await page.keyboard.press("Enter");

  // Wait for PRD and creation button
  await expect(
    page.getByRole("button", { name: "Create Project with this PRD" })
  ).toBeVisible();

  await page.getByRole("button", { name: "Create Project with this PRD" }).click();

  // We should land on the new project's detail page with the PRD visible
  await expect(page.getByRole("heading", { name: /Product Requirements Document/i })).toBeVisible();
});
