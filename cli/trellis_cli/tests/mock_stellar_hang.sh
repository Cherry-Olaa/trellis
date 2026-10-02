#!/usr/bin/env bash
#
# Hang mock: simulates a stellar binary that stalls indefinitely.
#
# Used by the timeout unit tests in rpc.rs to verify that invoke_once
# returns within STELLAR_INVOKE_TIMEOUT_SECS rather than blocking forever.
# The script sleeps for a very long time; the test kills it via the timeout.

sleep 9999
