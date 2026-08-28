@echo off
rem Start formicaria on Windows with the console VISIBLE.
rem
rem "Start formicaria.vbs" beside this is the normal way in - it runs the same server with no
rem window at all. This one exists for the two cases that need the output: script hosting disabled
rem by policy, and diagnosing a launch that did not work. fm-serve prints the keep-this-window-open
rem warning itself, so this file does not repeat it.
setlocal
set "HERE=%~dp0"
set "FM_VAULT=%HERE%vault"
set "FM_VAULTS=%HERE%vaults.json"
set "FM_OPEN=1"
set "FM_AUTO_SHUTDOWN=1"
if not exist "%FM_VAULT%" mkdir "%FM_VAULT%"
"%HERE%program\fm-serve.exe"
