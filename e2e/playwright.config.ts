import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 180_000, // 3 min per test (federation join can be slow)
  retries: 1, // 1 retry for LN flakiness
  workers: 1, // sequential: shared federation state
  use: {
    baseURL: "http://localhost:8080",
    browserName: "chromium",
    headless: true,
    screenshot: "only-on-failure",
    trace: "on-first-retry",
    actionTimeout: 60_000,
  },
  reporter: [["html", { open: "never" }], ["list"]],
});
