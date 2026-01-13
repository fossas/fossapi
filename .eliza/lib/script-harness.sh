#!/bin/bash
# Eliza script harness - wraps user scripts with JSON Lines output protocol
# Version: 1.2.0

set -euo pipefail

# Emit JSON event
emit_json() {
    local type=$1
    shift
    local json="$@"
    echo "{\"type\":\"$type\",$json}"
}

# Emit log message
log() {
    local level=$1
    local message=$2
    emit_json "log" "\"level\":\"$level\",\"message\":\"$message\""
}

# Emit phase start
phase_start() {
    local phase_id=$1
    local phase_name=${2:-$phase_id}
    emit_json "phase_start" "\"phase_id\":\"$phase_id\",\"name\":\"$phase_name\""
}

# Emit phase complete
phase_complete() {
    local phase_id=$1
    emit_json "phase_complete" "\"phase_id\":\"$phase_id\""
}

# Emit error
error() {
    local message=$1
    local recoverable=${2:-false}
    emit_json "error" "\"message\":\"$message\",\"recoverable\":$recoverable"
}

# Emit artifact
artifact() {
    local path=$1
    local artifact_type=${2:-file}
    local size
    if [ -f "$path" ]; then
        size=$(stat -f%z "$path" 2>/dev/null || stat -c%s "$path" 2>/dev/null || echo 0)
        emit_json "artifact" "\"path\":\"$path\",\"artifact_type\":\"$artifact_type\",\"size\":$size"
    fi
}

# Emit progress
progress() {
    local current=$1
    local total=$2
    emit_json "progress" "\"current\":$current,\"total\":$total"
}

# Export functions for script use
export -f emit_json log phase_start phase_complete error artifact progress

# Only run wrapper logic when executed directly (not when sourced)
# When sourced: BASH_SOURCE[0] != $0
# When executed: BASH_SOURCE[0] == $0
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    # Get script path from args
    SCRIPT_PATH="${1:?Script path required}"
    WRAPPER_MODE="${ELIZA_WRAPPER_MODE:-auto}"

    if [ "$WRAPPER_MODE" = "manual" ]; then
        # Just execute script, no auto-wrapping
        exec "$SCRIPT_PATH"
    fi

    # Auto mode: wrap execution
    log "info" "Starting script: $ELIZA_SCRIPT_NAME"

    # Run script and capture output
    set +e
    if [ -n "${ELIZA_PHASE_ID:-}" ]; then
        phase_start "$ELIZA_PHASE_ID" "${ELIZA_PHASE_NAME:-$ELIZA_PHASE_ID}"
    fi

    # Execute the actual script
    "$SCRIPT_PATH"
    EXIT_CODE=$?

    if [ -n "${ELIZA_PHASE_ID:-}" ]; then
        if [ $EXIT_CODE -eq 0 ]; then
            phase_complete "$ELIZA_PHASE_ID"
        else
            error "Script failed with exit code $EXIT_CODE" false
        fi
    fi

    log "info" "Script completed: exit code $EXIT_CODE"
    exit $EXIT_CODE
fi
