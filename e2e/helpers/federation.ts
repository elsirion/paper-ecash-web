import { readFileSync } from "fs";
import { resolve } from "path";

const DEFAULT_PATH = resolve(__dirname, "../.shared/invite-code.txt");

/**
 * Read the federation invite code written by the setup container.
 */
export function readInviteCode(): string {
  const path = process.env.INVITE_CODE_FILE
    ? resolve(process.env.INVITE_CODE_FILE)
    : DEFAULT_PATH;
  return readFileSync(path, "utf-8").trim();
}
