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

# Body on stdout with the HTTP status as the final line. `path` is relative to the network
# app's base path, i.e. exactly what the daemon's `UnifiClient::get` takes.

# api_key_get_path <base_path> <path>
api_key_get_path() {
  local base_path="$1" path="$2"
  curl_base -w '\n%{http_code}' \
    -H "X-API-KEY: ${UNIFI_API_KEY}" \
    "$(origin)${base_path}/${path}"
}

# local_admin_get_path <base_path> <login_path> <path>
local_admin_get_path() {
  local base_path="$1" login_path="$2" path="$3"
  # No RETURN trap: it would evaluate "$jar" after the local has gone out of scope, which
  # trips `set -u`. There is no early return here, so an explicit cleanup is enough.
  local jar out
  jar="$(mktemp)"

  curl_base -o /dev/null -c "$jar" \
    -H 'Content-Type: application/json' \
    -d "{\"username\":\"${UNIFI_USERNAME}\",\"password\":\"${UNIFI_PASSWORD}\"}" \
    "$(origin)${login_path}" || true

  out="$(curl_base -w '\n%{http_code}' -b "$jar" "$(origin)${base_path}/${path}")"
  rm -f "$jar"
  printf '%s\n' "$out"
}

# Site-scoped wrappers, mirroring `UnifiClient::get_site`.
api_key_get() {
  api_key_get_path "$1" "api/s/${UNIFI_SITE}/$2"
}
local_admin_get() {
  local_admin_get_path "$1" "$2" "api/s/${UNIFI_SITE}/$3"
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

  # The daemon validates the site name against the site *list* rather than by reading the
  # status code of a site-scoped call. That is not a stylistic choice: measured against UniFi
  # OS Server 5.1.21, an unknown site returns 401, which is indistinguishable from a rejected
  # credential — a typo'd site name would be reported to the user as a bad API key. So the
  # contract to verify here is that the non-site-scoped site list is reachable and names the
  # configured site.
  local response status body sites
  if [[ -n "$UNIFI_API_KEY" ]]; then
    response="$(api_key_get_path "/proxy/network" "api/self/sites")"
  else
    response="$(local_admin_get_path "/proxy/network" "/api/auth/login" "api/self/sites")"
  fi
  status="$(status_of "$response")"
  body="$(body_of "$response")"

  if [[ "$status" != "200" ]]; then
    warn "site list (api/self/sites) returned HTTP ${status} — the daemon will fall back to"
    warn "probing a site-scoped endpoint, so a bad site name may report as a bad credential"
    return
  fi

  if command -v jq >/dev/null 2>&1; then
    sites="$(jq -r '[.data[].name] | join(", ")' <<<"$body")"
    ok "site list reachable: ${sites}"
    if jq -e --arg s "$UNIFI_SITE" '[.data[].name] | index($s)' <<<"$body" >/dev/null; then
      ok "configured site '${UNIFI_SITE}' exists"
    else
      bad "configured site '${UNIFI_SITE}' is not in the list above"
    fi
  else
    ok "site list reachable (install jq to see the site names)"
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
