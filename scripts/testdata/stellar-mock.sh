#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--version" ]]; then
    echo "stellar mock"
    exit 0
fi

if [[ "${1:-}" == "contract" && "${2:-}" == "info" && "${3:-}" == "hash" ]]; then
    echo "${MOCK_WASM_HASH}"
    exit 0
fi

if [[ "${1:-}" == "contract" && "${2:-}" == "info" ]]; then
    echo "{\"wasm_hash\":\"${MOCK_WASM_HASH}\"}"
    exit 0
fi

if [[ "${1:-}" == "contract" && "${2:-}" == "invoke" ]]; then
    function_name="${!#}"

    if [[ "${MOCK_FAIL_SMOKE:-}" == "$function_name" ]]; then
        echo "mock failure for $function_name" >&2
        exit 1
    fi

    case "$function_name" in
        get_version)
            echo "2"
            ;;
        get_admin)
            echo "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            ;;
        get_pause_flags)
            echo "{lock_paused:false,release_paused:false,refund_paused:false}"
            ;;
        *)
            echo "ok"
            ;;
    esac
    exit 0
fi

echo "Unexpected stellar mock call: $*" >&2
exit 1
