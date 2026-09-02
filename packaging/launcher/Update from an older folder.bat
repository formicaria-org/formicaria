@echo off
setlocal enabledelayedexpansion
rem Move your notes into this newer formicaria, on Windows. Double-click this.
rem
rem A console window, deliberately: this asks a question and reports numbers, and a silent .vbs
rem wrapper — right for the launcher — would hide both. "Start formicaria.vbs" stays silent;
rem an update says what it did.
rem
rem Same rules as "Update from an older folder.sh", which carries the full reasoning:
rem   - run it in the NEW folder; it only ever copies INTO this folder
rem   - the old folder is never written to and never deleted, so it stays a complete backup
rem   - index.sqlite is not kept: it is a per-machine search index, rebuilt on the next start
rem   - vaults.json is NOT copied. It records each vault's ABSOLUTE path, so bringing it across
rem     would leave this newer formicaria writing into the folder you are about to delete.
rem
rem To say which folder to copy from, drag it onto this file.

rem %~dp0 is this script's own folder, with a trailing backslash. Never the working directory:
rem Explorer can start you anywhere, and anything relative would resolve against the wrong place.
set "HERE=%~dp0"
if "%HERE:~-1%"=="\" set "HERE=%HERE:~0,-1%"

echo.
echo   Moving your notes into this version of formicaria
echo   ================================================
echo.
echo   This folder:  %HERE%
echo.

rem --- 1. Refuse if this folder has already been used. ------------------------------------
set "USED="
if exist "%HERE%\vaults.json"        set "USED=vaults.json"
if exist "%HERE%\vault\.git"         set "USED=!USED! vault\.git"
if exist "%HERE%\vault\index.sqlite" set "USED=!USED! vault\index.sqlite"
if exist "%HERE%\update.log"         set "USED=!USED! update.log"

if not "!USED!"=="" (
    echo   This folder has already been used ^(!USED!^).
    echo.
    echo   Nothing has been changed. Copying another notebook on top of this one would mix
    echo   the two together, and there would be no way to separate them afterwards.
    echo.
    echo   If you meant to start again, unpack a fresh copy of the download and run this
    echo   script in THAT folder instead.
    echo.
    pause
    exit /b 1
)

rem --- 2. Find the folder to copy from. ---------------------------------------------------
set "OLD="
if not "%~1"=="" (
    if exist "%~1\vault\" (
        set "OLD=%~f1"
    ) else (
        echo   That does not look like a formicaria folder: %~1
        echo.
        pause
        exit /b 1
    )
)

if "!OLD!"=="" (
    set "COUNT=0"
    for /d %%D in ("%HERE%\..\formicaria-*") do (
        if /i not "%%~fD"=="%HERE%" (
            if exist "%%~fD\vault\notes\" (
                dir /b "%%~fD\vault\notes\*.md" >nul 2>&1 && (
                    set /a COUNT+=1
                    set "OLD=%%~fD"
                    set "FOUND!COUNT!=%%~fD"
                )
            )
        )
    )
    if !COUNT! EQU 0 (
        echo   I could not find an older formicaria folder beside this one.
        echo.
        echo   Drag the old folder onto this script to say where it is. Or copy its "vault"
        echo   folder into this one by hand - that is all this script does.
        echo.
        pause
        exit /b 1
    )
    if !COUNT! GTR 1 (
        echo   I found more than one older folder with notes in it:
        echo.
        for /l %%I in ^(1,1,!COUNT!^) do echo       !FOUND%%I!
        echo.
        echo   Drag the one you want onto this script, so the choice is yours and not mine.
        echo.
        pause
        exit /b 1
    )
)

rem --- 3. Say what will happen, then ask. -------------------------------------------------
set "NOTES=0"
for %%F in ("!OLD!\vault\notes\*.md") do set /a NOTES+=1
set "HISTORY=no"
if exist "!OLD!\vault\.git" set "HISTORY=yes"

echo   Copy FROM:    !OLD!
echo                 !NOTES! note^(s^), history: !HISTORY!
echo   Copy INTO:    %HERE%
echo.
echo   The old folder is not changed and not deleted. Keep it until you are sure.
echo.
set "ANSWER="
set /p "ANSWER=  Type yes to continue: "
if /i not "!ANSWER!"=="yes" (
    echo.
    echo   Nothing was changed.
    echo.
    pause
    exit /b 1
)
echo.

rem --- 4. Copy. ---------------------------------------------------------------------------
rem /E every subfolder including empty ones, /H the hidden ones so .git comes too, /I treat the
rem destination as a folder, /Y overwrite without asking, /Q quietly. Nothing here deletes.
xcopy "!OLD!\vault" "%HERE%\vault" /E /H /I /Y /Q >nul
if errorlevel 1 (
    echo   The copy did not finish.
    echo.
    echo   Your old folder has not been touched - everything is still in:
    echo       !OLD!
    echo.
    pause
    exit /b 1
)
if exist "%HERE%\vault\index.sqlite" del /q "%HERE%\vault\index.sqlite"

rem --- 5. Check it arrived, in numbers. ---------------------------------------------------
set "COPIED=0"
for %%F in ("%HERE%\vault\notes\*.md") do set /a COPIED+=1
if !COPIED! LSS !NOTES! (
    echo   SOMETHING WENT WRONG.
    echo.
    echo   Expected !NOTES! note^(s^) here, found !COPIED!.
    echo   Your old folder has not been touched - everything is still in:
    echo       !OLD!
    echo.
    pause
    exit /b 1
)

> "%HERE%\update.log" (
    echo updated %DATE% %TIME%
    echo from    !OLD!
    echo notes   !COPIED!
)

echo   Done. !NOTES! note^(s^) copied. This folder now holds !COPIED!.
if !COPIED! GTR !NOTES! echo   ^(The extra is the "Start here" note this download came with. Delete it whenever.^)
echo.
echo   Now start this version, and check your notes are here before deleting the old folder.
echo.
pause
