#!/bin/bash

# Script to create 150 hosts with 2 services each for pagination testing
# Rate limit: 20 requests/minute, so we do 18/minute to be safe (3.3s between requests)

API_KEY="scp_u_yfxREZbNOE1UXFFanOFj58AcOkipqw3r"
NETWORK_ID="b0000000-0000-0000-0000-000000000001"
BASE_URL="http://localhost:60072/api/v1/hosts"
TOTAL_HOSTS=150
DELAY_SECONDS=3.5  # ~17 requests per minute, under the 20/min limit

echo "Creating $TOTAL_HOSTS hosts with 2 services each..."
echo "Rate limit: 20/min, using ${DELAY_SECONDS}s delay between requests"
echo "Estimated time: ~9 minutes"
echo ""

success_count=0
fail_count=0

for i in $(seq 1 $TOTAL_HOSTS); do
    HOST_NAME="pagination-test-host-$(printf '%03d' $i)"
    SERVICE1_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
    SERVICE2_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

    # Create host with 2 services
    RESPONSE=$(curl -s -w "\n%{http_code}" -X POST \
        -H "Authorization: Bearer $API_KEY" \
        -H "Content-Type: application/json" \
        -d '{
            "name": "'"$HOST_NAME"'",
            "network_id": "'"$NETWORK_ID"'",
            "tags": [],
            "services": [
                {
                    "id": "'"$SERVICE1_ID"'",
                    "name": "'"$HOST_NAME"'-svc-1",
                    "service_definition": "HTTP"
                },
                {
                    "id": "'"$SERVICE2_ID"'",
                    "name": "'"$HOST_NAME"'-svc-2",
                    "service_definition": "PostgreSQL"
                }
            ]
        }' \
        "$BASE_URL")

    # Extract HTTP status code (last line)
    HTTP_CODE=$(echo "$RESPONSE" | tail -1)

    if [ "$HTTP_CODE" = "200" ]; then
        ((success_count++))
        echo "[$i/$TOTAL_HOSTS] Created: $HOST_NAME"
    else
        ((fail_count++))
        echo "[$i/$TOTAL_HOSTS] FAILED ($HTTP_CODE): $HOST_NAME"
        # If rate limited (429), wait longer
        if [ "$HTTP_CODE" = "429" ]; then
            echo "Rate limited, waiting 60s..."
            sleep 60
        fi
    fi

    # Progress update every 10 hosts
    if [ $((i % 10)) -eq 0 ]; then
        echo "--- Progress: $i/$TOTAL_HOSTS ($success_count succeeded, $fail_count failed) ---"
    fi

    # Wait before next request (except for the last one)
    if [ $i -lt $TOTAL_HOSTS ]; then
        sleep $DELAY_SECONDS
    fi
done

echo ""
echo "========================================"
echo "Complete! Created $success_count hosts, $fail_count failures"
echo "========================================"
