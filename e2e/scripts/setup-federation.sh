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
# FM_GATEWAY_API_ADDR is set in docker-compose.yml for the devtools service
gwcli() { $DC --profile setup run --rm devtools gateway-cli "$@"; }

retry() {
  local label="$1" max_attempts="$2"; shift 2
  echo "  Waiting for $label..."
  for i in $(seq 1 "$max_attempts"); do
    if output=$("$@" 2>&1); then
      echo "  $label ready."
      return 0
    fi
    if [ "$((i % 10))" -eq 0 ]; then
      echo "    (attempt $i/$max_attempts — last error: ${output:0:120})"
    fi
    sleep 2
  done
  echo "ERROR: $label did not become ready after $max_attempts attempts" >&2
  echo "  Last output: $output" >&2
  exit 1
}

# ── 1. Mine initial blocks ─────────────────────────────────────
echo "==> Creating bitcoind wallet and mining initial blocks"
btc_ createwallet "test" 2>/dev/null || btc_ loadwallet "test" 2>/dev/null || true
ADDR=$(btc getnewaddress)
btc generatetoaddress 200 "$ADDR" > /dev/null
echo "  Mined 200 blocks."

# ── 2. Wait for LND nodes to sync ──────────────────────────────
echo "==> Waiting for LND nodes to sync"
check_lnd_synced() {
  local node_cmd="$1"
  local info
  info=$($node_cmd getinfo 2>&1) || return 1
  echo "$info" | grep -q '"synced_to_chain": true'
}

retry "lnd-gateway sync" 60 check_lnd_synced lndg
retry "lnd-payer sync"   60 check_lnd_synced lndp

# ── 3. Fund both LND nodes ─────────────────────────────────────
echo "==> Funding LND nodes"
GW_ADDR=$(lndg newaddress p2wkh | grep -o '"address": "[^"]*"' | cut -d'"' -f4)
PAY_ADDR=$(lndp newaddress p2wkh | grep -o '"address": "[^"]*"' | cut -d'"' -f4)

btc sendtoaddress "$GW_ADDR" 10
btc sendtoaddress "$PAY_ADDR" 10
btc generatetoaddress 6 "$ADDR" > /dev/null
echo "  Funded both nodes with 10 BTC each."

# Wait for LND to see the confirmed balance
echo "==> Waiting for LND wallet balances"
check_lnd_funded() {
  local node_cmd="$1"
  local bal
  bal=$($node_cmd walletbalance 2>&1) || return 1
  echo "$bal" | grep -q '"confirmed_balance": "[1-9]'
}
retry "lnd-gateway balance" 30 check_lnd_funded lndg
retry "lnd-payer balance"   30 check_lnd_funded lndp

# ── 4. Connect peers and open channels ──────────────────────────
echo "==> Connecting LND peers and opening channels"
GW_PUBKEY=$(lndg getinfo | grep -o '"identity_pubkey": "[^"]*"' | cut -d'"' -f4)
PAY_PUBKEY=$(lndp getinfo | grep -o '"identity_pubkey": "[^"]*"' | cut -d'"' -f4)

lndg connect "${PAY_PUBKEY}@lnd-payer:9735" 2>/dev/null || true

# Open channels in both directions for bidirectional routing
lndg openchannel --node_key "$PAY_PUBKEY" --local_amt 5000000 --push_amt 0
btc generatetoaddress 6 "$ADDR" > /dev/null

lndp openchannel --node_key "$GW_PUBKEY" --local_amt 5000000 --push_amt 0
btc generatetoaddress 6 "$ADDR" > /dev/null

# Wait for channels to become active
check_channels_active() {
  local node_cmd="$1"
  local chans
  chans=$($node_cmd listchannels 2>&1) || return 1
  echo "$chans" | grep -q '"active": true'
}
retry "lnd-gateway channels active" 30 check_channels_active lndg
retry "lnd-payer channels active"   30 check_channels_active lndp
echo "  Channels open and active."

# ── 5. Run 1-of-1 federation DKG ───────────────────────────────
echo "==> Setting up 1-of-1 federation"

# Check if the federation is already running (idempotent)
if ! fmcli admin status 2>/dev/null | grep -q "ConsensusRunning"; then
  fmcli admin set-password
  fmcli admin set-config-gen-connections --our-name "guardian-0"
  fmcli admin run-dkg
  fmcli admin start-consensus
  echo "  DKG complete, consensus started."
else
  echo "  Federation already running."
fi

# Wait for consensus to be operational
retry "federation consensus" 30 fmcli admin status

# ── 6. Connect gateway to federation ───────────────────────────
echo "==> Connecting gateway to federation"
INVITE_CODE=$(fmcli dev invite-code | tr -d '"')

gwcli connect-fed "$INVITE_CODE" 2>/dev/null || true
retry "gateway federation" 30 gwcli info
echo "  Gateway connected."

# ── 7. Write invite code for Playwright ────────────────────────
mkdir -p "$E2E_DIR/.shared"
echo "$INVITE_CODE" > "$E2E_DIR/.shared/invite-code.txt"
echo "==> Setup complete. Invite code written to .shared/invite-code.txt"
echo "    $INVITE_CODE"
