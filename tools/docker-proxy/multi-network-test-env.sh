#!/bin/bash
set -euo pipefail

# Multi-Subnet Container Test Environment
#
# Creates Docker networks (discovered as container bridge subnets) and containers with a mix of
# single and multiple attachments, to exercise container→subnet membership in discovery and in
# the L3 Logical topology view.
#
# Fixture:
#   scanopy-test-proxy / scanopy-test-db / scanopy-test-mgmt  — bridge networks
#   sc-test-api       nginx        proxy + db          multi-attach, exposed ports
#   sc-test-worker    alpine       proxy + db + mgmt   multi-attach, NO exposed ports
#   sc-test-edge      nginx        proxy               published host port (18080)
#   sc-test-db        postgres     db                  control: single attachment
#   sc-test-hostmode  alpine       host networking     control: host-mode path
#
# Usage: tools/docker-proxy/multi-network-test-env.sh up
#        tools/docker-proxy/multi-network-test-env.sh down [--clean]
#        tools/docker-proxy/multi-network-test-env.sh status

NETWORKS=(scanopy-test-proxy scanopy-test-db scanopy-test-mgmt)
CONTAINERS=(sc-test-api sc-test-worker sc-test-edge sc-test-db sc-test-hostmode)
PUBLISHED_PORT=18080

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

check_deps() {
    if ! command -v docker &>/dev/null; then
        printf "${RED}docker not found${NC}\n"
        exit 1
    fi

    if ! docker info &>/dev/null; then
        printf "${RED}Cannot talk to the Docker daemon${NC}\n"
        echo "Is Docker running?"
        exit 1
    fi
}

cmd_up() {
    check_deps

    printf "${BOLD}Creating networks...${NC}\n"
    for net in "${NETWORKS[@]}"; do
        if docker network inspect "$net" &>/dev/null; then
            printf "  %-22s ${YELLOW}exists${NC}\n" "$net"
        else
            docker network create "$net" >/dev/null
            printf "  %-22s ${GREEN}created${NC}\n" "$net"
        fi
    done

    printf "\n${BOLD}Starting containers...${NC}\n"

    # Recreate from a known state — re-running `up` should be safe.
    for name in "${CONTAINERS[@]}"; do
        if docker container inspect "$name" &>/dev/null; then
            docker rm -f "$name" >/dev/null
        fi
    done

    # Multi-attach with exposed ports — the core case. nginx exposes 80 in its image, so the
    # container reports a private port on every endpoint it holds.
    docker run -d --name sc-test-api --network scanopy-test-proxy nginx:alpine >/dev/null
    docker network connect scanopy-test-db sc-test-api
    printf "  %-22s ${GREEN}up${NC}  proxy + db\n" "sc-test-api"

    # Multi-attach with no exposed ports — exercises the IP-address binding path.
    docker run -d --name sc-test-worker --network scanopy-test-proxy \
        alpine:latest sleep infinity >/dev/null
    docker network connect scanopy-test-db sc-test-worker
    docker network connect scanopy-test-mgmt sc-test-worker
    printf "  %-22s ${GREEN}up${NC}  proxy + db + mgmt\n" "sc-test-worker"

    # Published host port — exercises host-port binding alongside the bridge endpoint.
    docker run -d --name sc-test-edge --network scanopy-test-proxy \
        -p "${PUBLISHED_PORT}:80" nginx:alpine >/dev/null
    printf "  %-22s ${GREEN}up${NC}  proxy (published :%s)\n" "sc-test-edge" "$PUBLISHED_PORT"

    # Control: single attachment must not regress.
    docker run -d --name sc-test-db --network scanopy-test-db \
        -e POSTGRES_PASSWORD=scanopy-test postgres:16-alpine >/dev/null
    printf "  %-22s ${GREEN}up${NC}  db\n" "sc-test-db"

    # Control: host networking takes a different code path entirely.
    docker run -d --name sc-test-hostmode --network host \
        alpine:latest sleep infinity >/dev/null
    printf "  %-22s ${GREEN}up${NC}  host networking\n" "sc-test-hostmode"

    echo ""
    cmd_status

    echo ""
    printf "${BOLD}Next:${NC} run a discovery scan of this host, then in the topology's\n"
    printf "L3 Logical view remove the ${CYAN}Container Bridges${NC} grouping rule to see one\n"
    printf "box per subnet. sc-test-api should appear inside both proxy and db;\n"
    printf "sc-test-worker inside all three.\n"
}

cmd_down() {
    check_deps

    local clean="false"
    [ "${1:-}" = "--clean" ] && clean="true"

    printf "${BOLD}Removing containers...${NC}\n"
    for name in "${CONTAINERS[@]}"; do
        if docker container inspect "$name" &>/dev/null; then
            docker rm -f "$name" >/dev/null
            printf "  %-22s ${GREEN}removed${NC}\n" "$name"
        fi
    done

    if [ "$clean" = "true" ]; then
        printf "\n${BOLD}Removing networks...${NC}\n"
        for net in "${NETWORKS[@]}"; do
            if docker network inspect "$net" &>/dev/null; then
                docker network rm "$net" >/dev/null
                printf "  %-22s ${GREEN}removed${NC}\n" "$net"
            fi
        done
    else
        printf "\nNetworks kept. Use ${CYAN}down --clean${NC} to remove them too.\n"
    fi
}

cmd_status() {
    check_deps

    printf "${BOLD}Multi-Subnet Test Environment${NC}\n"
    echo "============================="

    for net in "${NETWORKS[@]}"; do
        if ! docker network inspect "$net" &>/dev/null; then
            printf "%-22s ${RED}absent${NC}\n" "$net"
            continue
        fi
        local cidr members
        cidr=$(docker network inspect "$net" --format '{{range .IPAM.Config}}{{.Subnet}}{{end}}')
        members=$(docker network inspect "$net" --format '{{range .Containers}}{{.Name}} {{end}}')
        printf "%-22s ${GREEN}%s${NC}\n" "$net" "${cidr:-no subnet}"
        printf "  members: %s\n" "${members:-none}"
    done

    echo ""
    for name in "${CONTAINERS[@]}"; do
        if ! docker container inspect "$name" &>/dev/null; then
            printf "%-22s ${RED}absent${NC}\n" "$name"
            continue
        fi
        local attachments
        attachments=$(docker container inspect "$name" \
            --format '{{range $net, $cfg := .NetworkSettings.Networks}}{{$net}}={{$cfg.IPAddress}} {{end}}')
        printf "%-22s ${GREEN}running${NC}\n" "$name"
        printf "  %s\n" "${attachments:-host networking}"
    done
}

case "${1:-}" in
    up)
        cmd_up
        ;;
    down)
        shift
        cmd_down "$@"
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "Usage: $0 {up|down|status}"
        echo ""
        echo "  up              — Create test networks and containers"
        echo "  down [--clean]  — Remove containers (--clean removes networks too)"
        echo "  status          — Show networks, CIDRs, and container attachments"
        exit 1
        ;;
esac
