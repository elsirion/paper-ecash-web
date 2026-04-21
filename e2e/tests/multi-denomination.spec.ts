import { test, expect } from "@playwright/test";
import { readInviteCode } from "../helpers/federation.js";
import { payInvoice } from "../helpers/lightning.js";

test("multi-denomination note is issued as a single OOBNotes envelope", async ({
  page,
}) => {
  const inviteCode = readInviteCode();

  await page.goto("/");
  await page.getByRole("button", { name: "Issue Paper Ecash" }).click();

  // ── Federation step ──────────────────────────────────────────
  await page.getByRole("button", { name: "Enter code manually" }).click();
  await page.locator("textarea#invite-code").fill(inviteCode);
  await page.getByRole("button", { name: "Next" }).click();

  // ── Denomination step: pick two denominations ────────────────
  // 1024 msat = "1.02 ksat", 2048 msat = "2.05 ksat"
  await page.getByRole("button", { name: "1.02 ksat" }).click();
  await page.getByRole("button", { name: "2.05 ksat" }).click();

  // Verify the combined note value is shown
  await expect(page.getByText("Note value:")).toBeVisible();
  await expect(page.getByText("3.07 ksat")).toBeVisible();

  await page.getByRole("button", { name: "Next" }).click();

  // ── Count step: just 1 note ──────────────────────────────────
  await page.locator("input#count-input").fill("1");
  await page.getByRole("button", { name: "Next" }).click();

  // ── Deposit step ─────────────────────────────────────────────
  const invoiceTextarea = page.locator("textarea[readonly]");
  await invoiceTextarea.waitFor({ state: "visible", timeout: 120_000 });

  const bolt11 = await invoiceTextarea.inputValue();
  expect(bolt11).toBeTruthy();

  await payInvoice(bolt11);
  await page.getByText("Payment received!").waitFor({ timeout: 60_000 });

  // ── Issue step ───────────────────────────────────────────────
  await page
    .getByText(/All \d+ notes issued/)
    .waitFor({ timeout: 120_000 });

  // ── Design step ──────────────────────────────────────────────
  await page.getByText("Design").first().waitFor();
  await page.getByRole("button", { name: "Next" }).click();

  // ── PDF step ─────────────────────────────────────────────────
  await page.getByText("Generate PDF").first().waitFor();

  // Verify the summary shows both denominations
  await expect(page.getByText("1.02 ksat")).toBeVisible();
  await expect(page.getByText("2.05 ksat")).toBeVisible();

  // Generate and verify PDF
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("button", { name: "Generate & Download PDF" }).click(),
  ]);

  const filePath = await download.path();
  expect(filePath).toBeTruthy();

  const fs = await import("fs");
  const stat = fs.statSync(filePath!);
  expect(stat.size).toBeGreaterThan(10_000);
});
