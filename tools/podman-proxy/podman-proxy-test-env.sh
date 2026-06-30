#!/bin/bash
set -euo pipefail

# Podman Proxy Test Environment — exposes the local Podman socket on TCP via
# nginx, mirroring tools/docker-proxy/docker-proxy-test-env.sh. Lets you exercise
# the Podman *proxy* transport (with optional mTLS), and the `workload`
# subcommand seeds a pod + published-port containers so discovery has something
# to find.
#
# Usage: tools/podman-proxy/podman-proxy-test-env.sh up [--tls] [--port PORT]
#        tools/podman-proxy/podman-proxy-test-env.sh down [--clean]
#        tools/podman-proxy/podman-proxy-test-env.sh status
#        tools/podman-proxy/podman-proxy-test-env.sh workload [up|down]
#
# macOS note: Podman runs inside a VM (`podman machine`). The script resolves the
# host-side socket from `podman machine inspect`; start the machine first with
# `podman machine init && podman machine start`.

WORK_DIR=/tmp/podman-proxy-test-env
PID_FILE="$WORK_DIR/nginx.pid"
CONF_FILE="$WORK_DIR/nginx.conf"
CERT_DIR="$WORK_DIR/certs"
STATE_FILE="$WORK_DIR/state"
# Distinct from docker-proxy's 2376 so both harnesses can run side by side.
DEFAULT_PORT=2378

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# Resolve the host-side Podman socket path. Honors PODMAN_SOCK override, then the
# `podman machine` socket (macOS / VM setups), then the rootful and rootless
# Linux defaults. Echoes empty string if none found.
resolve_podman_sock() {
    if [ -n "${PODMAN_SOCK:-}" ] && [ -S "${PODMAN_SOCK}" ]; then
        echo "$PODMAN_SOCK"
        return
    fi
    if command -v podman &>/dev/null; then
        local p
        p=$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}' 2>/dev/null | head -1 || true)
        if [ -n "$p" ] && [ -S "$p" ]; then
            echo "$p"
            return
        fi
    fi
    if [ -S /run/podman/podman.sock ]; then
        echo /run/podman/podman.sock
        return
    fi
    if [ -n "${XDG_RUNTIME_DIR:-}" ] && [ -S "$XDG_RUNTIME_DIR/podman/podman.sock" ]; then
        echo "$XDG_RUNTIME_DIR/podman/podman.sock"
        return
    fi
    echo ""
}

check_deps() {
    local missing=()

    if ! command -v nginx &>/dev/null; then
        missing+=("nginx")
    fi

    if ! command -v openssl &>/dev/null; then
        missing+=("openssl")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        printf "${RED}Missing dependencies:${NC} %s\n" "${missing[*]}"
        echo ""
        for dep in "${missing[@]}"; do
            case "$dep" in
                nginx)   echo "  brew install nginx    # macOS" ;;
                openssl) echo "  brew install openssl  # macOS" ;;
            esac
        done
        exit 1
    fi

    PODMAN_SOCK_RESOLVED=$(resolve_podman_sock)
    if [ -z "$PODMAN_SOCK_RESOLVED" ]; then
        printf "${RED}Podman socket not found${NC}\n"
        echo "Checked: \$PODMAN_SOCK, 'podman machine inspect', /run/podman/podman.sock,"
        echo "         \$XDG_RUNTIME_DIR/podman/podman.sock"
        echo ""
        echo "Start Podman first:"
        echo "  macOS:  podman machine init && podman machine start"
        echo "  Linux:  systemctl --user start podman.socket   # rootless"
        echo "          sudo systemctl start podman.socket      # rootful"
        exit 1
    fi
}

generate_certs() {
    mkdir -p "$CERT_DIR"

    echo "Generating TLS certificates..."

    # CA
    openssl genrsa -out "$CERT_DIR/ca-key.pem" 4096 2>/dev/null
    openssl req -new -x509 -days 365 -key "$CERT_DIR/ca-key.pem" \
        -sha256 -out "$CERT_DIR/ca.pem" \
        -subj "/CN=Podman Proxy Test CA" 2>/dev/null

    # Server cert
    openssl genrsa -out "$CERT_DIR/server-key.pem" 4096 2>/dev/null
    openssl req -new -key "$CERT_DIR/server-key.pem" \
        -subj "/CN=localhost" -out "$CERT_DIR/server.csr" 2>/dev/null

    cat > "$CERT_DIR/server-ext.cnf" <<EOF
subjectAltName = DNS:localhost,IP:127.0.0.1
extendedKeyUsage = serverAuth
EOF

    openssl x509 -req -days 365 -sha256 \
        -in "$CERT_DIR/server.csr" \
        -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial \
        -out "$CERT_DIR/server-cert.pem" \
        -extfile "$CERT_DIR/server-ext.cnf" 2>/dev/null

    # Client cert
    openssl genrsa -out "$CERT_DIR/client-key.pem" 4096 2>/dev/null
    openssl req -new -key "$CERT_DIR/client-key.pem" \
        -subj "/CN=Podman Proxy Test Client" -out "$CERT_DIR/client.csr" 2>/dev/null

    cat > "$CERT_DIR/client-ext.cnf" <<EOF
extendedKeyUsage = clientAuth
EOF

    openssl x509 -req -days 365 -sha256 \
        -in "$CERT_DIR/client.csr" \
        -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial \
        -out "$CERT_DIR/client-cert.pem" \
        -extfile "$CERT_DIR/client-ext.cnf" 2>/dev/null

    # Clean up CSRs and temp files
    rm -f "$CERT_DIR"/*.csr "$CERT_DIR"/*.cnf "$CERT_DIR"/*.srl

    printf "  ${GREEN}✓${NC} CA cert:      %s\n" "$CERT_DIR/ca.pem"
    printf "  ${GREEN}✓${NC} Server cert:  %s\n" "$CERT_DIR/server-cert.pem"
    printf "  ${GREEN}✓${NC} Server key:   %s\n" "$CERT_DIR/server-key.pem"
    printf "  ${GREEN}✓${NC} Client cert:  %s\n" "$CERT_DIR/client-cert.pem"
    printf "  ${GREEN}✓${NC} Client key:   %s\n" "$CERT_DIR/client-key.pem"
}

generate_nginx_conf() {
    local port="$1"
    local tls="$2"
    local sock="$3"

    cat > "$CONF_FILE" <<EOF
worker_processes 1;
pid $PID_FILE;
error_log $WORK_DIR/error.log warn;

events {
    worker_connections 64;
}

http {
    access_log $WORK_DIR/access.log;

    upstream podman {
        server unix:$sock;
    }

    server {
        listen 127.0.0.1:${port}$([ "$tls" = "true" ] && echo " ssl");
EOF

    if [ "$tls" = "true" ]; then
        cat >> "$CONF_FILE" <<EOF

        ssl_certificate $CERT_DIR/server-cert.pem;
        ssl_certificate_key $CERT_DIR/server-key.pem;
        ssl_client_certificate $CERT_DIR/ca.pem;
        ssl_verify_client on;

        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;
EOF
    fi

    cat >> "$CONF_FILE" <<EOF

        location / {
            proxy_pass http://podman;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_read_timeout 300;
            proxy_connect_timeout 10;

            # Required for streaming endpoints (logs, events, attach)
            proxy_buffering off;
            chunked_transfer_encoding on;
        }
    }
}
EOF
}

print_pem_contents() {
    echo ""
    printf "${BOLD}Certificate PEM contents (copy-paste into Scanopy):${NC}\n"

    echo ""
    printf "${CYAN}── CA Certificate ──${NC}\n"
    cat "$CERT_DIR/ca.pem"

    echo ""
    printf "${CYAN}── Client Certificate ──${NC}\n"
    cat "$CERT_DIR/client-cert.pem"

    echo ""
    printf "${CYAN}── Client Key ──${NC}\n"
    cat "$CERT_DIR/client-key.pem"
}

cmd_up() {
    local port="$DEFAULT_PORT"
    local tls=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tls) tls=true; shift ;;
            --port) port="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    check_deps

    # Check if already running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        printf "${YELLOW}Proxy already running${NC} (PID $(cat "$PID_FILE"))\n"
        echo "Run '$0 down' first."
        exit 1
    fi

    mkdir -p "$WORK_DIR"

    echo ""
    printf "${BOLD}Podman Proxy Test Environment${NC}\n"
    echo "=============================="
    printf "Podman socket: %s\n" "$PODMAN_SOCK_RESOLVED"

    # Generate TLS certs if needed
    if [ "$tls" = true ]; then
        generate_certs
        echo ""
    fi

    # Generate nginx config
    generate_nginx_conf "$port" "$tls" "$PODMAN_SOCK_RESOLVED"

    # Start nginx
    echo "Starting nginx proxy..."
    nginx -c "$CONF_FILE"

    # Wait briefly for nginx to start
    sleep 1

    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        local pid
        pid=$(cat "$PID_FILE")
        printf "  ${GREEN}✓${NC} nginx started (PID %s)\n" "$pid"
    else
        printf "  ${RED}✗${NC} nginx failed to start\n"
        echo "Check logs: $WORK_DIR/error.log"
        exit 1
    fi

    # Save state for status command
    echo "port=$port" > "$STATE_FILE"
    echo "tls=$tls" >> "$STATE_FILE"
    echo "sock=$PODMAN_SOCK_RESOLVED" >> "$STATE_FILE"

    # Print summary
    local scheme="http"
    [ "$tls" = true ] && scheme="https"

    echo ""
    printf "${BOLD}Proxy URL:${NC} %s://127.0.0.1:%s\n" "$scheme" "$port"
    echo ""

    if [ "$tls" = true ]; then
        print_pem_contents
        echo ""
        printf "${BOLD}Verify with:${NC}\n"
        printf "  curl --cacert %s --cert %s --key %s https://127.0.0.1:%s/version\n" \
            "$CERT_DIR/ca.pem" "$CERT_DIR/client-cert.pem" "$CERT_DIR/client-key.pem" "$port"
    else
        printf "${BOLD}Verify with:${NC}\n"
        printf "  curl http://127.0.0.1:%s/version\n" "$port"
    fi

    echo ""
    printf "${GREEN}Podman proxy is ready.${NC}\n"
    printf "Create a workload to discover: %s workload up\n" "$0"
}

cmd_down() {
    local clean=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --clean) clean=true; shift ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    echo "Stopping Podman proxy..."

    if [ -f "$PID_FILE" ]; then
        local pid
        pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            nginx -c "$CONF_FILE" -s stop 2>/dev/null || kill "$pid"
            printf "  ${GREEN}✓${NC} nginx stopped (PID %s)\n" "$pid"
        else
            echo "  nginx was not running"
        fi
    else
        echo "  No PID file found"
    fi

    # Clean up config and logs
    rm -f "$CONF_FILE" "$PID_FILE" "$STATE_FILE"
    rm -f "$WORK_DIR"/access.log "$WORK_DIR"/error.log

    if [ "$clean" = true ]; then
        echo "Cleaning up certificates..."
        rm -rf "$CERT_DIR"
    fi

    # Remove work dir if empty
    rmdir "$WORK_DIR" 2>/dev/null || true

    echo ""
    printf "${GREEN}Podman proxy stopped.${NC}\n"
    if [ "$clean" = false ] && [ -d "$CERT_DIR" ]; then
        echo "Certificates kept in $CERT_DIR (use --clean to remove)"
    fi
}

cmd_status() {
    echo "Podman Proxy Test Environment"
    echo "=============================="

    if [ ! -f "$PID_FILE" ] || ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        printf "Status:  ${RED}stopped${NC}\n"
        if [ -d "$CERT_DIR" ]; then
            printf "Certs:   ${YELLOW}present${NC} (%s)\n" "$CERT_DIR"
        fi
        return
    fi

    local pid
    pid=$(cat "$PID_FILE")
    local port="$DEFAULT_PORT"
    local tls="false"
    local sock=""

    if [ -f "$STATE_FILE" ]; then
        # shellcheck source=/dev/null
        source "$STATE_FILE"
    fi

    local scheme="http"
    [ "$tls" = "true" ] && scheme="https"

    printf "Status:  ${GREEN}running${NC} (PID %s)\n" "$pid"
    printf "URL:     %s://127.0.0.1:%s\n" "$scheme" "$port"
    printf "TLS:     %s\n" "$tls"
    printf "Socket:  %s\n" "$sock"

    if [ -d "$CERT_DIR" ]; then
        printf "Certs:   %s\n" "$CERT_DIR"
    fi
}

# Seed/tear-down a discoverable workload: a pod with a published-port nginx
# container plus a standalone published-port container. Gives discovery
# containers, ports, and (rootful) a bridge subnet to find.
cmd_workload() {
    if ! command -v podman &>/dev/null; then
        printf "${RED}podman CLI not found${NC}\n"
        exit 1
    fi

    case "${1:-up}" in
        up)
            echo "Creating test workload..."
            # Pod publishes nginx (80) and a fingerprintable Grafana (3000). Both run as pod
            # members (shared netns) so discovery must resolve their interfaces from the pod's
            # infra container, and Grafana exercises HTTP service detection via an existing def.
            podman pod create --name scanopy-test-pod -p 8088:80 -p 8090:3000 >/dev/null 2>&1 || true
            podman run -d --pod scanopy-test-pod --name scanopy-test-web \
                docker.io/library/nginx:alpine >/dev/null
            podman run -d --pod scanopy-test-pod --name scanopy-test-grafana \
                docker.io/grafana/grafana >/dev/null
            podman run -d --name scanopy-test-standalone -p 8089:80 \
                docker.io/library/nginx:alpine >/dev/null
            echo ""
            printf "  ${GREEN}✓${NC} pod 'scanopy-test-pod' (nginx on host port 8088, Grafana on host port 8090)\n"
            printf "  ${GREEN}✓${NC} container 'scanopy-test-standalone' (nginx on host port 8089)\n"
            echo ""
            podman ps --filter "name=scanopy-test" --format "table {{.Names}}\t{{.Ports}}\t{{.Status}}"
            ;;
        down)
            echo "Removing test workload..."
            podman rm -f scanopy-test-standalone >/dev/null 2>&1 || true
            podman pod rm -f scanopy-test-pod >/dev/null 2>&1 || true
            printf "  ${GREEN}✓${NC} workload removed\n"
            ;;
        *)
            echo "Usage: $0 workload {up|down}"
            exit 1
            ;;
    esac
}

case "${1:-}" in
    up)
        shift
        cmd_up "$@"
        ;;
    down)
        shift
        cmd_down "$@"
        ;;
    status)
        cmd_status
        ;;
    workload)
        shift
        cmd_workload "$@"
        ;;
    *)
        echo "Usage: $0 {up|down|status|workload}"
        echo ""
        echo "  up [--tls] [--port PORT]  — Start Podman API proxy via nginx"
        echo "  down [--clean]            — Stop proxy (--clean removes certs too)"
        echo "  status                    — Show current proxy state"
        echo "  workload [up|down]        — Create/remove a discoverable pod + containers"
        echo ""
        echo "Examples:"
        echo "  $0 up                     # HTTP proxy on port $DEFAULT_PORT"
        echo "  $0 up --tls               # HTTPS with mTLS on port $DEFAULT_PORT"
        echo "  $0 workload up            # Seed a pod + published-port containers"
        echo "  $0 down --clean           # Stop, remove certs"
        exit 1
        ;;
esac
