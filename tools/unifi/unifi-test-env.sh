#!/usr/bin/env bash
#
# UniFi controller test-environment driver.
#
# Exercises the same two auth transports and two API layouts the daemon integration does, so a
# green run here means the transport half of the integration is real — not inferred from docs.
#
# What this CANNOT validate: the adopted-device tables (port_table / lldp_table / mac_table /
# uplink / downlink_table). Those need real adopted hardware. A controller with no devices
# returns an empty `data` array, which proves the envelope and nothing about the device shapes.
#
# See UNIFI-TEST-ENV.md for provisioning the controller.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CAPTURE_DIR="${UNIFI_CAPTURE_DIR:-$SCRIPT_DIR/captures}"

: "${UNIFI_HOST:=}"
: "${UNIFI_PORT:=11443}"
: "${UNIFI_SITE:=default}"
: "${UNIFI_API_KEY:=}"
: "${UNIFI_USERNAME:=}"
: "${UNIFI_PASSWORD:=}"

BOLD=$'\033[1m'; RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; RESET=$'\033[0m'

die()  { echo "${RED}error:${RESET} $*" >&2; exit 1; }
info() { echo "${BOLD}==>${RESET} $*"; }
ok()   { echo "  ${GREEN}ok${RESET}    $*"; }
warn() { echo "  ${YELLOW}warn${RESET}  $*"; }
bad()  { echo "  ${RED}fail${RESET}  $*"; }

require_host() {
  [[ -n "$UNIFI_HOST" ]] || die "set UNIFI_HOST (see tools/unifi/UNIFI-TEST-ENV.md)"
}

origin() { echo "https://${UNIFI_HOST}:${UNIFI_PORT}"; }

# UniFi controllers ship self-signed certs, hence -k. This mirrors the daemon honouring
# `accept_invalid_scan_certs`; it is not a shortcut taken only by this script.
curl_base() { curl -sk --connect-timeout 5 --max-time 30 "$@"; }

# ---------------------------------------------------------------------------------------------
# Transport probes
# ---------------------------------------------------------------------------------------------

# api_key_get <base_path> <endpoint> -> body on stdout, HTTP status on fd 3
api_key_get() {
  local base_path="$1" endpoint="$2"
  curl_base -w '\n%{http_code}' \
    -H "X-API-KEY: ${UNIFI_API_KEY}" \
    "$(origin)${base_path}/api/s/${UNIFI_SITE}/${endpoint}"
}

# local_admin_get <base_path> <login_path> <endpoint>
local_admin_get() {
  local base_path="$1" login_path="$2" endpoint="$3"
  local jar; jar="$(mktemp)"
  trap 'rm -f "$jar"' RETURN

  curl_base -o /dev/null -c "$jar" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"${UNIFI_USERNAME}\",\"password\":\"${UNIFI_PASSWORD}\"}" \
    "$(origin)${login_path}" || true

  curl_base -w '\n%{http_code}' -b "$jar" \
    "$(origin)${base_path}/api/s/${UNIFI_SITE}/${endpoint}"
}

status_of() { tail -n1 <<<"$1"; }
body_of()   { sed '$d' <<<"$1"; }

# Try both API layouts for one transport; echo the layout that worked.
detect_layout() {
  local transport="$1" endpoint="${2:-stat/sysinfo}"
  local response status

  for layout in unifi-os legacy; do
    case "$layout" in
      unifi-os) base_path="/proxy/network"; login_path="/api/auth/login" ;;
      legacy)   base_path="";               login_path="/api/login" ;;
    esac

    if [[ "$transport" == "api-key" ]]; then
      response="$(api_key_get "$base_path" "$endpoint")"
    else
      response="$(local_admin_get "$base_path" "$login_path" "$endpoint")"
    fi
    status="$(status_of "$response")"

    if [[ "$status" == "200" ]]; then
      echo "$layout"
      return 0
    fi
  done
  return 1
}

# ---------------------------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------------------------

cmd_status() {
  require_host
  info "UniFi controller at $(origin), site '${UNIFI_SITE}'"

  if ! curl_base -o /dev/null "$(origin)" 2>/dev/null; then
    bad "controller is not reachable"
    exit 1
  fi
  ok "controller is reachable"

  local any=0

  if [[ -n "$UNIFI_API_KEY" ]]; then
    if layout="$(detect_layout api-key)"; then
      ok "API key accepted (${layout} layout)"
      any=1
    else
      bad "API key rejected on both layouts"
      warn "expected on a legacy self-hosted Network Application (8443): Ubiquiti"
      warn "does not support API keys there — use a local admin instead"
    fi
  else
    warn "UNIFI_API_KEY unset, skipping the API-key transport"
  fi

  if [[ -n "$UNIFI_USERNAME" && -n "$UNIFI_PASSWORD" ]]; then
    if layout="$(detect_layout local-admin)"; then
      ok "local admin accepted (${layout} layout)"
      any=1
    else
      bad "local admin rejected on both layouts"
    fi
  else
    warn "UNIFI_USERNAME/UNIFI_PASSWORD unset, skipping the local-admin transport"
  fi

  [[ "$any" == "1" ]] || die "no transport authenticated"

  # A bad site name is the most common misconfiguration and returns 404, not 401 — the daemon
  # reports these differently, so check the distinction actually holds.
  local response status
  if [[ -n "$UNIFI_API_KEY" ]]; then
    response="$(UNIFI_SITE=definitely-not-a-site api_key_get "/proxy/network" "stat/sysinfo")"
  else
    response="$(UNIFI_SITE=definitely-not-a-site local_admin_get "/proxy/network" "/api/auth/login" "stat/sysinfo")"
  fi
  status="$(status_of "$response")"
  if [[ "$status" == "404" ]]; then
    ok "unknown site returns 404 (distinguishable from a rejected credential)"
  else
    warn "unknown site returned HTTP ${status}, not 404 — the daemon's error message for"
    warn "a bad site name may be misleading on this controller version"
  fi
}

cmd_capture() {
  require_host
  mkdir -p "$CAPTURE_DIR"

  local transport layout base_path login_path
  if [[ -n "$UNIFI_API_KEY" ]]; then transport=api-key; else transport=local-admin; fi
  layout="$(detect_layout "$transport")" || die "could not authenticate"
  case "$layout" in
    unifi-os) base_path="/proxy/network"; login_path="/api/auth/login" ;;
    legacy)   base_path="";               login_path="/api/login" ;;
  esac
  info "capturing via ${transport} (${layout} layout)"

  for endpoint in stat/sysinfo stat/device; do
    local response status body out
    if [[ "$transport" == "api-key" ]]; then
      response="$(api_key_get "$base_path" "$endpoint")"
    else
      response="$(local_admin_get "$base_path" "$login_path" "$endpoint")"
    fi
    status="$(status_of "$response")"
    body="$(body_of "$response")"
    [[ "$status" == "200" ]] || { bad "${endpoint} -> HTTP ${status}"; continue; }

    out="$CAPTURE_DIR/${endpoint//\//_}.json"
    if command -v jq >/dev/null 2>&1; then
      jq '.' <<<"$body" > "$out"
    else
      printf '%s\n' "$body" > "$out"
    fi
    ok "${endpoint} -> ${out}"

    if [[ "$endpoint" == "stat/device" ]] && command -v jq >/dev/null 2>&1; then
      local count with_ports
      count="$(jq '.data | length' <<<"$body")"
      with_ports="$(jq '[.data[] | select((.port_table // []) | length > 0)] | length' <<<"$body")"
      echo "        devices: ${count}, with a port_table: ${with_ports}"
      if [[ "$with_ports" == "0" ]]; then
        warn "no device reports a port_table — this capture validates the envelope and"
        warn "auth only. The port/LLDP/FDB mappings stay unvalidated until a controller"
        warn "with adopted hardware is captured."
      fi
    fi
  done

  echo
  echo "${BOLD}Captures are raw controller output and may contain MACs, IPs and device names.${RESET}"
  echo "Redact values before sharing, but do not reshape the JSON — the field names and"
  echo "nesting are the whole point."
}

case "${1:-status}" in
  status)  cmd_status ;;
  capture) cmd_capture ;;
  *) die "usage: $(basename "$0") {status|capture}" ;;
esac
