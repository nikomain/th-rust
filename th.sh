#!/bin/bash

# Teleport Helper Wrapper Function
# This function calls the Rust binary and then sources the credentials if they exist

th() {
    # Get the directory of this script
    local SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    
    # Call the Rust binary with all arguments
    "$SCRIPT_DIR/target/release/th" "$@"
    local EXIT_CODE=$?
    
    # If it was an AWS command (and not help), try to source the credentials
    if [[ ("$1" == "aws" || "$1" == "a") && "$2" != "-h" && "$2" != "--help" ]]; then
        # Look for the most recent tsh_proxy log file
        local LATEST_LOG=$(ls -t /tmp/tsh_proxy_*.log 2>/dev/null | head -1)
        if [[ -n "$LATEST_LOG" && -f "$LATEST_LOG" ]]; then
            source "$LATEST_LOG"
        fi
    fi
    
    # If it was a logout command, unset environment variables in current shell
    if [[ "$1" == "logout" || "$1" == "l" ]]; then
        unset AWS_ACCESS_KEY_ID
        unset AWS_SECRET_ACCESS_KEY
        unset AWS_CA_BUNDLE
        unset HTTPS_PROXY
        unset ACCOUNT
        unset ROLE
        unset AWS_DEFAULT_REGION
    fi
    
    return $EXIT_CODE
}