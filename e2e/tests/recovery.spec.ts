import { test, expect } from "@playwright/test";
import { readInviteCode } from "../helpers/federation.js";
import { payInvoice } from "../helpers/lightning.js";

test("recover issuance after page reload mid-minting", async ({ page }) => {
  const inviteCode = readInviteCode();

  await page.goto("/");
  await page.getByRole("button", { name: "Issue Paper Ecash" }).click();

  // ── Federation step ──────────────────────────────────────────
  await page.getByRole("button", { name: "Enter code manually" }).click();
  await page.locator("textarea#invite-code").fill(inviteCode);
  await page.getByRole("button", { name: "Next" }).click();

  // ── Denomination step ────────────────────────────────────────
  await page.getByRole("button", { name: "1.02 sat" }).click();
  await page.getByRole("button", { name: "Next" }).click();

  // ── Count step: 3 notes to give time for mid-issuance reload ─
  await page.locator("input#count-input").fill("3");
  await page.getByRole("button", { name: "Next" }).click();

  // ── Deposit step ─────────────────────────────────────────────
  const invoiceTextarea = page.locator("textarea[readonly]");
  await invoiceTextarea.waitFor({ state: "visible", timeout: 120_000 });

  const bolt11 = await invoiceTextarea.inputValue();
  expect(bolt11).toBeTruthy();

  await payInvoice(bolt11);
  await page.getByText("Payment received!").waitFor({ timeout: 60_000 });

  // ── Issue step: wait for at least one note, then reload ──────
  await page
    .getByText(/Issuing note [12] of 3/)
    .waitFor({ timeout: 60_000 });

  // Force reload mid-issuance
  await page.reload();

  // ── Landing page after reload ────────────────────────────────
  // The app should show the Issuances link since we have an in-progress one
  // Navigate back to the issuance via the Issuances page
  await page.getByRole("link", { name: "Issuances" }).click();

  // Find the in-progress issuance and click to resume
  const issuanceRow = page.locator("tr, [class*='issuance']").first();
  await issuanceRow.click();

  // ── Resumed: Issue step should recover and finish ────────────
  // The app reconnects, runs recover_issued_notes(), and mints remaining
  await page
    .getByText(/All \d+ notes issued/)
    .waitFor({ timeout: 120_000 });

  // ── Design step ──────────────────────────────────────────────
  await page.getByText("Design").first().waitFor();
  await page.getByRole("button", { name: "Next" }).click();

  // ── PDF step ─────────────────────────────────────────────────
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
