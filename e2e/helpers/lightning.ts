import { exec as execCb } from "child_process";
import { promisify } from "util";
import { resolve } from "path";

const exec = promisify(execCb);

const E2E_DIR = resolve(__dirname, "..");

function dc(service: string, cmd: string): string {
  return `docker compose -f ${E2E_DIR}/docker-compose.yml exec -T ${service} ${cmd}`;
}

/**
 * Pay a BOLT11 invoice using the lnd-payer node.
 * Blocks until the payment settles or times out.
 */
export async function payInvoice(bolt11: string): Promise<void> {
  const cmd = dc(
    "lnd-payer",
    `lncli --network=regtest sendpayment --pay_req ${bolt11} --force`,
  );
  try {
    await exec(cmd, { timeout: 60_000 });
  } catch (err: any) {
    const msg = err.stderr || err.stdout || err.message || "unknown error";
    throw new Error(`Payment failed: ${msg}`);
  }
}

/**
 * Mine N regtest blocks.
 */
export async function mineBlocks(n: number): Promise<void> {
  const getAddr = dc(
    "bitcoind",
    `bitcoin-cli -regtest -rpcuser=bitcoin -rpcpassword=bitcoin getnewaddress`,
  );
  const { stdout: addr } = await exec(getAddr);
  const mine = dc(
    "bitcoind",
    `bitcoin-cli -regtest -rpcuser=bitcoin -rpcpassword=bitcoin generatetoaddress ${n} ${addr.trim()}`,
  );
  await exec(mine, { timeout: 30_000 });
}
