@echo off
rem Start formicaria on Windows with the console VISIBLE.
rem
rem `formicaria.vbs` beside this is the normal way in — it starts the same server with no window.
rem This one exists for the two cases that need output: Windows Script Host disabled by policy,
rem and diagnosing a launch that did not work. Closing this window stops the app.
setlocal
set "HERE=%~dp0"
set "FM_VAULT=%HERE%vault"
set "FM_VAULTS=%HERE%vaults.json"
set "FM_OPEN=1"
set "FM_AUTO_SHUTDOWN=1"
if not exist "%FM_VAULT%" mkdir "%FM_VAULT%"
"%HERE%fm-serve.exe"
