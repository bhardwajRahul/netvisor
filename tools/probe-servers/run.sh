#!/bin/bash
set -uo pipefail

# ══════════════════════════════════════════════════════════════════════
# Reference servers for the application probes.
#
# `live_servers.rs` runs every probe against a real implementation, because a
# parser test cannot catch a request built wrongly: a malformed request draws no
# reply, the parser is handed nothing, and `NoAnswer` is indistinguishable from
# "no server there". That is not hypothetical — it hid a wrong connect-data offset
# in the Oracle probe, which a real listener answered the moment it was corrected.
#
#   ./run.sh up      start everything on its real port on loopback
#   ./run.sh down    stop only what this script started
#
# Then:
#   cd backend && cargo test --lib -- --ignored live_servers
#   cd backend && SCANOPY_LIVE_UDP_PORTS=500,1194 cargo test --lib -- --ignored named_udp
#
# Most servers are public images. The seven built here are the ones with no usable
# arm64 image, or none that runs without hardware: a KDC, a directory, a NUT daemon
# (which exits without a UPS attached), a Check_MK agent (the published image is the
# monitoring *server*, which listens on nothing), xrdp, strongSwan and OpenVPN.
# ══════════════════════════════════════════════════════════════════════

DIR="$(cd "$(dirname "$0")" && pwd)"
STATE="${TMPDIR:-/tmp}/scanopy-probe-servers.cids"

# name  host:container  image  [extra docker args...]
PUBLIC_SERVERS=(
  "ssh        22:2222      lscr.io/linuxserver/openssh-server|-e|USER_NAME=probe"
  "ftp        21:21        delfer/alpine-ftp-server"
  "dns        53:53        ubuntu/bind9"
  "smb        445:445      dperson/samba"
  "rtsp       554:8554     bluenviron/mediamtx:latest"
  "mssql      1433:1433    mcr.microsoft.com/azure-sql-edge:latest|-e|ACCEPT_EULA=1|-e|MSSQL_SA_PASSWORD=Probe_12345"
  "mqtt       1883:1883    eclipse-mosquitto:2"
  "mysql      3306:3306    mysql:8|-e|MYSQL_ROOT_PASSWORD=probe"
  "opcua      4840:4840    open62541/open62541"
  "sip        5060:5060    ghcr.io/kamailio/kamailio-ci:5.8-alpine"
  "postgres   5432:5432    postgres:16-alpine|-e|POSTGRES_PASSWORD=probe"
  "amqp       5672:5672    rabbitmq:3-alpine"
  "redis      6379:6379    redis:alpine"
  "cassandra  9042:9042    cassandra:4"
  "kafka      9092:9092    apache/kafka:latest"
  "zabbix     10050:10050  zabbix/zabbix-agent:alpine-7.0-latest|-e|ZBX_PASSIVESERVERS=0.0.0.0/0|-e|ZBX_ACTIVESERVERS=127.0.0.1"
  "oracle     1521:1521    gvenzl/oracle-free:slim|-e|ORACLE_PASSWORD=probe"
  "mongodb    27017:27017  mongo:7"
  # Needs a real public key or the agent exits; the probe reads its banner and never authenticates.
  "beszel     45876:45876  henrygd/beszel-agent|-e|KEY=ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINRaenOMiDdJNWfIyv2c+gy/rINJPQUX7VALCON2DM97 scanopy-probe-reference-server|-e|LISTEN=45876"
)

# The ones built here. RDP and the two VPNs need extra privileges. The list grew because the runner
# passes docker *flags* rather than a container command, so anything needing a custom command (the
# busybox telnetd) has to be an image too.
BUILT_SERVERS=(
  "kerberos   88:88        probe-kerberos"
  "telnet     23:23        probe-telnet"
  "nfs        2049:2049    probe-nfs"
  "unbound    8953:8953    probe-unbound|-p|127.0.0.1:5353:53/udp"
  "salt       4505:4505    probe-salt|-p|127.0.0.1:4506:4506"
  "bacula     9101:9101    probe-bacula"
  "ldap       389:389      probe-ldap"
  "nut        3493:3493    probe-nut"
  "checkmk    6556:6556    probe-checkmk"
  "rdp        3389:3389    probe-rdp"
  "ike        500:500/udp  probe-ike|--privileged|-p|127.0.0.1:4500:4500/udp"
  "openvpn    1194:1194/udp probe-openvpn|--cap-add=NET_ADMIN|--device|/dev/net/tun"
)

start_one() {
    local name=$1 mapping=$2 spec=$3
    local IFS='|'; read -ra parts <<< "$spec"; unset IFS
    local image="${parts[0]}"; local extra=("${parts[@]:1}")

    local host="${mapping%%:*}" rest="${mapping#*:}"
    local publish="127.0.0.1:${host}:${rest}"

    local cid
    # `${extra[@]+...}` rather than a bare `"${extra[@]}"`: under `set -u` bash 3.2 treats an
    # empty array as unset and aborts, which silently failed every server taking no extra args.
    cid=$(docker run -d --rm -p "$publish" ${extra[@]+"${extra[@]}"} "$image" 2>&1 | tail -1)
    if docker inspect "$cid" >/dev/null 2>&1; then
        echo "$cid" >> "$STATE"
        printf '  %-10s %s\n' "$name" "started"
    else
        printf '  %-10s %s\n' "$name" "FAILED: $cid"
    fi
}

case "${1:-up}" in
  up)
    : > "$STATE"
    echo "=== Building the servers with no usable public image ==="
    for f in "$DIR"/Dockerfile.*; do
        n="${f##*Dockerfile.}"
        printf '  %-10s ' "$n"
        docker build -q -t "probe-$n" -f "$f" "$DIR" >/dev/null 2>&1 && echo built || echo FAILED
    done

    echo "=== Starting ==="
    for entry in "${PUBLIC_SERVERS[@]}" "${BUILT_SERVERS[@]}"; do
        read -r name mapping spec <<< "$entry"
        start_one "$name" "$mapping" "$spec"
    done

    echo
    echo 'Databases take a minute or two to accept connections. `live_servers` reports'
    echo 'which ports had nothing listening, so re-run it rather than guessing.'
    ;;
  down)
    [ -f "$STATE" ] || { echo "nothing recorded as started"; exit 0; }
    while read -r cid; do docker rm -f "$cid" >/dev/null 2>&1; done < "$STATE"
    rm -f "$STATE"
    echo "stopped everything this script started; anything else on the host is untouched"
    ;;
  *)
    echo "usage: $0 [up|down]" >&2; exit 2 ;;
esac
