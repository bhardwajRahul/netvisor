#!/usr/bin/env bash
# Generate schema-constrained JSON with the Claude API and expose it as the
# step output `structured_output`, for consumption via fromJSON() in the
# workflow. Callers read fields off that object rather than parsing prose.
#
# Inputs (environment):
#   ANTHROPIC_API_KEY  required
#   PROMPT             required — the full instruction, including any context
#   JSON_SCHEMA        required — JSON Schema the reply must satisfy; needs
#                      "required" and "additionalProperties": false
#   MAX_TOKENS         optional, default 16000. Caps thinking + output
#                      together, so leave generous headroom.
#   MODEL              optional, default claude-opus-5
set -euo pipefail

: "${ANTHROPIC_API_KEY:?ANTHROPIC_API_KEY is not set}"
: "${PROMPT:?PROMPT is not set}"
: "${JSON_SCHEMA:?JSON_SCHEMA is not set}"

max_tokens="${MAX_TOKENS:-16000}"

if ! printf '%s' "$JSON_SCHEMA" | jq empty 2>/dev/null; then
  echo "JSON_SCHEMA is not valid JSON:" >&2
  printf '%s\n' "$JSON_SCHEMA" >&2
  exit 1
fi

# jq builds the payload so prompts containing quotes, newlines, backticks or
# dollar signs cannot break out and corrupt the request.
body=$(jq -n \
  --arg model "${MODEL:-claude-opus-5}" \
  --argjson max_tokens "$max_tokens" \
  --arg prompt "$PROMPT" \
  --argjson schema "$JSON_SCHEMA" \
  '{model: $model,
    max_tokens: $max_tokens,
    output_config: {format: {type: "json_schema", schema: $schema}},
    messages: [{role: "user", content: $prompt}]}')

response_file=$(mktemp)
trap 'rm -f "$response_file"' EXIT

http_status=$(curl -sS --retry 3 --retry-connrefused \
  -o "$response_file" -w '%{http_code}' \
  https://api.anthropic.com/v1/messages \
  -H "content-type: application/json" \
  -H "anthropic-version: 2023-06-01" \
  -H "x-api-key: $ANTHROPIC_API_KEY" \
  --data-binary @- <<<"$body")

reply=$(cat "$response_file")

if [ "$http_status" != "200" ]; then
  echo "Claude API returned HTTP $http_status:" >&2
  # Surface the API's own error message; it names the offending field.
  jq -r '.error.message // .' <<<"$reply" >&2 2>/dev/null || printf '%s\n' "$reply" >&2
  exit 1
fi

stop_reason=$(jq -r '.stop_reason // "unknown"' <<<"$reply")
case "$stop_reason" in
  refusal)
    echo "Claude declined the request: $(jq -r '.stop_details.explanation // "no explanation given"' <<<"$reply")" >&2
    exit 1
    ;;
  max_tokens)
    # Truncated output is not valid JSON, so fail loudly rather than let a
    # half-written changelog reach the pull request.
    echo "Response hit max_tokens ($max_tokens) and is truncated. Raise MAX_TOKENS." >&2
    exit 1
    ;;
esac

# output_config.format guarantees the text block is JSON matching the schema.
text=$(jq -r '[.content[] | select(.type == "text") | .text] | join("")' <<<"$reply")
if [ -z "$text" ] || ! printf '%s' "$text" | jq empty 2>/dev/null; then
  echo "Expected schema-conforming JSON, got:" >&2
  printf '%s\n' "${text:-<empty>}" >&2
  exit 1
fi

# Compact to one line so it survives GITHUB_OUTPUT without a heredoc.
compact=$(jq -c . <<<"$text")

echo "structured_output=$compact" >> "$GITHUB_OUTPUT"
echo "Generated $(printf '%s' "$compact" | wc -c) bytes of structured output."
