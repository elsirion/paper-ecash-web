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
# Use the native binaries inside each container.
# v0.10.0 uses FM_API_URL env var instead of --url flag.
fmcli() { $DC exec -T -e FM_PASSWORD=testpass -e FM_API_URL=ws://127.0.0.1:18174 fedimintd fedimint-cli --data-dir /tmp/fm-client "$@"; }
gwcli() { $DC exec -T gatewayd gateway-cli --address http://127.0.0.1:8175 --rpcpassword testpass "$@"; }

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

fmsetup() { fmcli admin setup ws://127.0.0.1:18174 "$@"; }

if ! fmcli admin status 2>/dev/null | grep -qi "consensus"; then
  # v0.10.0 DKG: set-local-params returns the invite/connection code
  INVITE_CODE=$(fmsetup set-local-params --federation-name "e2e-test" "guardian-0" 2>/dev/null | tr -d '"')
  fmsetup start-dkg
  echo "  DKG complete, consensus started."
else
  echo "  Federation already running."
fi

# Give fedimintd a moment to start consensus after DKG
sleep 5
echo "  Federation should now be running."

# ── 6. Configure gateway mnemonic ──────────────────────────────
echo "==> Configuring gateway mnemonic"
# Check if gateway-cli auth works at all
echo "  Testing gateway-cli info:"
gwcli info 2>&1 | head -5 || true
# Now try to set mnemonic
echo "  Calling cfg set-mnemonic..."
gwcli cfg set-mnemonic 2>&1 || true
# Also check gatewayd logs for the mnemonic status
echo "  Gatewayd logs after set-mnemonic:"
$DC logs --tail=5 gatewayd 2>&1 | grep -v "^$" || true

# Wait for gateway to enter Running state
echo "==> Waiting for gateway to start"
for i in $(seq 1 30); do
  GW_STATUS=$(gwcli info 2>&1 || true)
  if echo "$GW_STATUS" | grep -qi "version\|running\|gateway_id"; then
    echo "  Gateway running (attempt $i)."
    break
  fi
  [ "$i" -eq 30 ] && { echo "ERROR: Gateway did not start" >&2; echo "$GW_STATUS" >&2; exit 1; }
  sleep 2
done

# ── 7. Connect gateway and get proper invite code ─────────────
echo "==> Connecting gateway to federation"
gwcli connect-fed "$INVITE_CODE" 2>/dev/null || true

# Now generate the proper fed1... invite code for the WASM client.
# Use the guardian's data-dir which has the federation config.
# Extract the fed1 invite code from the gateway (it knows the federation)
# The consensus API has a federation_invite_code endpoint.
# Call it via fedimint-cli or directly via the JSON-RPC API.
echo "  Fetching invite code from federation API..."

# The fedimint JSON-RPC API uses the same WS endpoint.
# fedimint-cli admin can call it if we use the right data-dir.
# But admin commands need auth. Let's try calling the API directly.
#
# The 'config' API endpoint returns the client config without auth.
# We can use fedimint-cli --data-dir /data to call admin methods.
CLIENT_INVITE=$(fmcli admin invite-code 2>&1 || true)
echo "  admin invite-code: ${CLIENT_INVITE:0:80}"

# If admin invite-code doesn't exist, try fetching via the public API
if [[ "$CLIENT_INVITE" != fed1* ]]; then
  # Call the JSON-RPC endpoint directly using wget
  CLIENT_INVITE=$($DC exec -T fedimintd sh -c '
    wget -qO- --post-data "{\"jsonrpc\":\"2.0\",\"method\":\"federation_invite_code\",\"params\":[],\"id\":1}" \
      --header="Content-Type: application/json" \
      http://127.0.0.1:18174 2>/dev/null
  ' | grep -oE 'fed1[a-z0-9]+' | head -1 || true)
  echo "  JSON-RPC result: ${CLIENT_INVITE:0:80}"
fi

if [[ "$CLIENT_INVITE" == fed1* ]]; then
  # Replace Docker hostname with localhost for browser access
  # (The invite code is bech32m so we can't do string replacement.
  # But since the API URL is ws://fedimintd:18174 and the runner has
  # "127.0.0.1 fedimintd" in /etc/hosts, the browser CAN resolve it.)
  INVITE_CODE="$CLIENT_INVITE"
  echo "  Got fed1 invite code: ${INVITE_CODE:0:60}..."
else
  echo "  Could not get fed1 invite code."
  echo "  Using DKG string (may not work): ${INVITE_CODE:0:60}..."
fi

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
