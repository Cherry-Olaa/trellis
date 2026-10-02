@echo off
REM Hang mock: simulates a stellar binary that stalls indefinitely.
REM
REM Used by the timeout unit tests in rpc.rs to verify that invoke_once
REM returns within STELLAR_INVOKE_TIMEOUT_SECS rather than blocking forever.

timeout /t 9999 /nobreak >nul
