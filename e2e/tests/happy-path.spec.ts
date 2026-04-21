import { test, expect } from "@playwright/test";
import { readInviteCode } from "../helpers/federation.js";
import { payInvoice } from "../helpers/lightning.js";

test("full issuance flow: connect, pay, mint, download PDF", async ({
  page,
}) => {
  const inviteCode = readInviteCode();

  // ── Landing ──────────────────────────────────────────────────
  await page.goto("/");
  await page.getByRole("button", { name: "Issue Paper Ecash" }).click();

  // ── Federation step ──────────────────────────────────────────
  await page.getByRole("button", { name: "Enter code manually" }).click();
  await page.locator("textarea#invite-code").fill(inviteCode);
  await page.getByRole("button", { name: "Next" }).click();

  // ── Denomination step ────────────────────────────────────────
  // Select the smallest denomination (1024 msat = "1.02 ksat")
  await page.getByRole("button", { name: "1.02 ksat" }).click();
  await page.getByRole("button", { name: "Next" }).click();

  // ── Count step ───────────────────────────────────────────────
  await page.locator("input#count-input").fill("2");
  await page.getByRole("button", { name: "Next" }).click();

  // ── Deposit step ─────────────────────────────────────────────
  // Wait for the invoice to appear (federation join + invoice creation)
  const invoiceTextarea = page.locator("textarea[readonly]");
  await invoiceTextarea.waitFor({ state: "visible", timeout: 120_000 });

  // Read the BOLT11 invoice and pay it via lnd-payer
  const bolt11 = await invoiceTextarea.inputValue();
  expect(bolt11).toBeTruthy();

  await payInvoice(bolt11);

  // Wait for payment confirmation and auto-advance to Issue step
  await page.getByText("Payment received!").waitFor({ timeout: 60_000 });

  // ── Issue step ───────────────────────────────────────────────
  await page
    .getByText(/All \d+ notes issued/)
    .waitFor({ timeout: 120_000 });

  // ── Design step ──────────────────────────────────────────────
  // Wait for designs to load and select the first one (should be auto-selected)
  await page.getByText("Design").first().waitFor();
  // Click Next (design should already be selected by default)
  await page.getByRole("button", { name: "Next" }).click();

  // ── PDF step ─────────────────────────────────────────────────
  await page.getByText("Generate PDF").first().waitFor();

  // Trigger download and verify the PDF
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("button", { name: "Generate & Download PDF" }).click(),
  ]);

  const filePath = await download.path();
  expect(filePath).toBeTruthy();

  // Verify it downloaded with a reasonable size
  const fs = await import("fs");
  const stat = fs.statSync(filePath!);
  expect(stat.size).toBeGreaterThan(10_000);

  // ── Done ─────────────────────────────────────────────────────
  await page.getByRole("button", { name: "Done" }).click();
  // Should be back on the landing page
  await page
    .getByRole("button", { name: "Issue Paper Ecash" })
    .waitFor();
});
