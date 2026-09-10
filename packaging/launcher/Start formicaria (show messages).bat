@echo off
rem Start formicaria on Windows with the console VISIBLE.
rem
rem "Start formicaria.vbs" beside this is the normal way in - it runs the same server with no
rem window at all. This one exists for the two cases that need the output: script hosting disabled
rem by policy, and diagnosing a launch that did not work. fm-serve prints the keep-this-window-open
rem warning itself, so this file does not repeat it.
rem
rem **No labels and no `goto` anywhere in this file, deliberately.** `.gitattributes` keeps every
rem file in this repo LF-only and the release archive ships this one as it is (only README.txt is
rem converted to CRLF), and cmd.exe seeks by byte offset when it resolves a label - on an LF-only
rem batch file that lands in the wrong place. The rescue below is written with a flag and delayed
rem expansion instead, which needs no seeking.
setlocal enabledelayedexpansion
set "HERE=%~dp0"
set "FM_VAULT=%HERE%vault"
set "FM_VAULTS=%HERE%vaults.json"
set "FM_OPEN=1"
set "FM_AUTO_SHUTDOWN=1"
rem The generation of this launcher, read by the updater. 2 means "this folder can put the previous
rem version back if an update fails" - the block below. The app refuses to update itself when this
rem is unset, because a folder with an older launcher has no way back.
set "FM_LAUNCHER=2"

rem **Two ways an update can leave you stuck, and this handles both.** The program can be *missing*
rem - interrupted between the two renames the updater does - or *there and unable to start*, which
rem looking at the folder does not reveal. Hence the counter: this script adds one on every start
rem and formicaria removes it once it has been serving for a moment, so three starts that never get
rem that far mean the new version does not run on this computer.
set "ATTEMPTS=0"
if exist "%HERE%.fm-attempts" set /p ATTEMPTS=<"%HERE%.fm-attempts"
rem Anything that is not a plain number counts as none: a corrupted counter must not roll anyone
rem back. `set /a` reads a non-numeric value as zero, which is exactly what is wanted here.
set /a ATTEMPTS=ATTEMPTS+0 >nul 2>&1

set "NEEDRESTORE="
if not exist "%HERE%program\fm-serve.exe" set "NEEDRESTORE=1"
if !ATTEMPTS! GEQ 3 set "NEEDRESTORE=1"

set "RESTORED="
if defined NEEDRESTORE (
  rem `move` on a directory is a rename here, same as the `mv` on the other two platforms.
  for /d %%B in ("%HERE%.fm-backup-*") do (
    if not defined RESTORED if exist "%%~fB\fm-serve.exe" (
      echo formicaria: putting the previous version back.
      if exist "%HERE%program" rmdir /s /q "%HERE%program"
      move "%%~fB" "%HERE%program" >nul
      del /q "%HERE%.fm-attempts" >nul 2>&1
      set "ATTEMPTS=0"
      set "RESTORED=1"
    )
  )
  rem **Only reset the count when something was actually put back.** Saying "putting the previous
  rem version back" when there is nothing to put back is a sentence that repeats forever and
  rem changes nothing - which is what a user with no terminal would be left with.
  if not defined RESTORED (
    echo formicaria: this copy will not start properly and there is no earlier version here.
    echo Download it again: https://github.com/formicaria-org/formicaria/releases/latest
    echo Your notes are in the "vault" folder beside this file and have not been touched.
    if not exist "%HERE%program\fm-serve.exe" exit /b 1
  )
)

rem Count this start. Cleared by formicaria once it is up, so it only ever accumulates across
rem starts that failed.
if exist "%HERE%program\fm-serve.exe" (
  set /a NEXT=ATTEMPTS+1
  >"%HERE%.fm-attempts" echo !NEXT!
)

if not exist "%FM_VAULT%" mkdir "%FM_VAULT%"
"%HERE%program\fm-serve.exe"
