#!/usr/bin/env bash
#
# Bootstrap a 1-of-1 Fedimint federation with two funded LND nodes
# and a connected Lightning gateway.
#
# Run from the e2e/ directory AFTER `docker compose up -d --wait`.
# Writes the invite code to e2e/.shared/invite-code.txt for Playwright.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
E2E_DIR="$(dirname "$SCRIPT_DIR")"
cd "$E2E_DIR"

DC="docker compose"

# ── helpers ─────────────────────────────────────────────────────
btc_()  { $DC exec -T bitcoind     bitcoin-cli -regtest -rpcuser=bitcoin -rpcpassword=bitcoin "$@"; }
btc()   { btc_ -rpcwallet=test "$@"; }
lndg()  { $DC exec -T lnd-gateway  lncli --network=regtest "$@"; }
lndp()  { $DC exec -T lnd-payer    lncli --network=regtest "$@"; }
fmcli() { $DC --profile setup run --rm devtools fedimint-cli --url ws://fedimintd:18174 "$@"; }
gwcli() { $DC --profile setup run --rm devtools gateway-cli "$@"; }

# ── 1. Mine initial blocks ─────────────────────────────────────
echo "==> Creating bitcoind wallet and mining initial blocks"
btc_ createwallet "test" 2>/dev/null || btc_ loadwallet "test" 2>/dev/null || true
ADDR=$(btc getnewaddress)
btc generatetoaddress 200 "$ADDR" > /dev/null
echo "  Mined 200 blocks."

# ── 2. Wait for LND nodes to sync ──────────────────────────────
echo "==> Waiting for LND nodes to sync"
for i in $(seq 1 90); do
  GW_SYNCED=$(lndg getinfo 2>&1 | grep -c '"synced_to_chain":.*true' || true)
  PAY_SYNCED=$(lndp getinfo 2>&1 | grep -c '"synced_to_chain":.*true' || true)
  if [ "$GW_SYNCED" -ge 1 ] && [ "$PAY_SYNCED" -ge 1 ]; then
    echo "  Both LND nodes synced (attempt $i)."
    break
  fi
  if [ "$i" -eq 90 ]; then
    echo "ERROR: LND nodes did not sync after 90 attempts" >&2
    echo "  lnd-gateway synced: $GW_SYNCED, lnd-payer synced: $PAY_SYNCED" >&2
    echo "  lnd-gateway getinfo:" >&2
    lndg getinfo 2>&1 | grep -i sync >&2 || true
    echo "  lnd-payer getinfo:" >&2
    lndp getinfo 2>&1 | grep -i sync >&2 || true
    exit 1
  fi
  [ "$((i % 15))" -eq 0 ] && echo "  (attempt $i/90 — gw=$GW_SYNCED pay=$PAY_SYNCED)"
  sleep 2
done

# ── 3. Fund both LND nodes ─────────────────────────────────────
echo "==> Funding LND nodes"
# LND v0.18 uses "address":  "..." (double space in JSON output)
GW_ADDR=$(lndg newaddress p2wkh | grep -o '"address":.*"[^"]*"' | grep -o '"[^"]*"$' | tr -d '"')
PAY_ADDR=$(lndp newaddress p2wkh | grep -o '"address":.*"[^"]*"' | grep -o '"[^"]*"$' | tr -d '"')
echo "  Gateway address: $GW_ADDR"
echo "  Payer address: $PAY_ADDR"

btc sendtoaddress "$GW_ADDR" 10
btc sendtoaddress "$PAY_ADDR" 10
btc generatetoaddress 6 "$ADDR" > /dev/null
echo "  Funded both nodes with 10 BTC each."

echo "==> Waiting for LND wallet balances"
for i in $(seq 1 30); do
  GW_BAL=$(lndg walletbalance 2>&1 | grep '"confirmed_balance"' || true)
  PAY_BAL=$(lndp walletbalance 2>&1 | grep '"confirmed_balance"' || true)
  if echo "$GW_BAL" | grep -qE '[1-9]' && echo "$PAY_BAL" | grep -qE '[1-9]'; then
    echo "  Balances confirmed (attempt $i)."
    break
  fi
  [ "$i" -eq 30 ] && { echo "ERROR: LND balances not confirmed" >&2; exit 1; }
  sleep 2
done

# ── 4. Connect peers and open channels ──────────────────────────
echo "==> Connecting LND peers and opening channels"
# Extract pubkeys (LND JSON: "identity_pubkey":  "abc123",)
GW_PUBKEY=$(lndg getinfo | sed -n 's/.*"identity_pubkey":[[:space:]]*"\([^"]*\)".*/\1/p')
PAY_PUBKEY=$(lndp getinfo | sed -n 's/.*"identity_pubkey":[[:space:]]*"\([^"]*\)".*/\1/p')
echo "  Gateway pubkey: $GW_PUBKEY"
echo "  Payer pubkey: $PAY_PUBKEY"

lndg connect "${PAY_PUBKEY}@lnd-payer:9735" 2>/dev/null || true

lndg openchannel --node_key "$PAY_PUBKEY" --local_amt 5000000 --push_amt 0
btc generatetoaddress 6 "$ADDR" > /dev/null

lndp openchannel --node_key "$GW_PUBKEY" --local_amt 5000000 --push_amt 0
btc generatetoaddress 6 "$ADDR" > /dev/null

echo "==> Waiting for channels to become active"
for i in $(seq 1 60); do
  GW_ACTIVE=$(lndg listchannels 2>&1 | grep -c '"active":.*true' || true)
  PAY_ACTIVE=$(lndp listchannels 2>&1 | grep -c '"active":.*true' || true)
  if [ "$GW_ACTIVE" -ge 1 ] && [ "$PAY_ACTIVE" -ge 1 ]; then
    echo "  Channels active (attempt $i)."
    break
  fi
  [ "$i" -eq 60 ] && { echo "ERROR: Channels not active" >&2; exit 1; }
  sleep 2
done

# ── 5. Run 1-of-1 federation DKG ───────────────────────────────
echo "==> Setting up 1-of-1 federation"

if ! fmcli admin status 2>/dev/null | grep -q "ConsensusRunning"; then
  fmcli admin set-password
  fmcli admin set-config-gen-connections --our-name "guardian-0"
  fmcli admin run-dkg
  fmcli admin start-consensus
  echo "  DKG complete, consensus started."
else
  echo "  Federation already running."
fi

echo "==> Waiting for federation consensus"
for i in $(seq 1 30); do
  if fmcli admin status 2>&1 | grep -q "ConsensusRunning"; then
    echo "  Consensus running (attempt $i)."
    break
  fi
  [ "$i" -eq 30 ] && { echo "ERROR: Federation consensus not running" >&2; exit 1; }
  sleep 2
done

# ── 6. Connect gateway to federation ───────────────────────────
echo "==> Connecting gateway to federation"
INVITE_CODE=$(fmcli dev invite-code | tr -d '"')

gwcli connect-fed "$INVITE_CODE" 2>/dev/null || true

echo "==> Waiting for gateway to connect"
for i in $(seq 1 30); do
  if gwcli info 2>&1 | grep -q "federation"; then
    echo "  Gateway connected (attempt $i)."
    break
  fi
  [ "$i" -eq 30 ] && { echo "ERROR: Gateway did not connect" >&2; exit 1; }
  sleep 2
done

# ── 7. Write invite code for Playwright ────────────────────────
mkdir -p "$E2E_DIR/.shared"
echo "$INVITE_CODE" > "$E2E_DIR/.shared/invite-code.txt"
echo "==> Setup complete. Invite code written to .shared/invite-code.txt"
echo "    $INVITE_CODE"
