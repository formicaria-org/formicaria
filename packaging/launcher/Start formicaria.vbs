' Start formicaria on Windows. Double-click this.
'
' **Why a .vbs and not just the .exe.** `fm-serve.exe` is a console binary, so double-clicking it
' opens a black window that has to stay open, shows no app, and opens no browser. This runs it
' with no window at all, which is what makes the folder feel like a portable app rather than a
' developer tool. `formicaria.bat` beside it does the same thing with the console visible — use
' that one if Windows Script Host is disabled on this machine, or to see an error.
'
' Hiding the console is only safe because `fm-serve` no longer panics on a busy port: it detects
' its own already-running instance, opens the browser and exits. Without that fix this script
' would turn a confusing error into an app that silently does nothing.
Option Explicit
Dim sh, fso, here, vault, prog, folder, attempts, ts, line, restored

Set sh  = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")

' This script's own folder — never the working directory, which Explorer does not set to
' anything useful. The app IS this folder; copy it to a USB stick and the notes travel with it.
here  = fso.GetParentFolderName(WScript.ScriptFullName)
vault = here & "\vault"

' Both vault variables together: FM_VAULT alone, with no vaults.json beside it, means vault #1
' disappears the moment a second one is created.
With sh.Environment("Process")
  .Item("FM_VAULT")         = vault
  .Item("FM_VAULTS")        = here & "\vaults.json"
  .Item("FM_OPEN")          = "1"
  .Item("FM_AUTO_SHUTDOWN") = "1"
  ' The generation of this launcher, read by the updater. 2 means "this folder can put the
  ' previous version back if an update fails" - the block below. The app refuses to update
  ' itself when this is unset, because an older launcher has no way back.
  .Item("FM_LAUNCHER")      = "2"
End With

' **If the program is missing, put back the one that was working.** The safety net behind
' updating in place (decisions.md, 2026-09-10). The updater renames program\ aside to
' .fm-backup-<version> and renames the new one in, so the only moment fm-serve.exe can be absent
' is between those two renames, or after an update that stopped partway. Either way the version
' that was working is right here.
'
' It lives in the launcher because the app is the thing that is missing, and a person with no
' terminal has exactly one gesture: double-clicking this file.
'
' **Windows renames a folder rather than copying it**, so this is as cheap and as atomic here as
' the `mv` is on the other two platforms.
'
' **Two ways an update can leave you stuck, and this handles both.** The program can be *missing*
' - interrupted between the two renames - or *there and unable to start*, which looking at the
' folder does not reveal. Hence the counter: this script adds one on every start and formicaria
' removes it once it has been serving for a moment, so three starts that never get that far mean
' the new version does not run on this computer.
prog = here & "\program\fm-serve.exe"
attempts = 0
If fso.FileExists(here & "\.fm-attempts") Then
  On Error Resume Next
  Set ts = fso.OpenTextFile(here & "\.fm-attempts", 1)
  If Err.Number = 0 Then
    line = Trim(ts.ReadLine)
    ts.Close
    ' Anything that is not a plain number counts as none: a corrupted counter must not roll
    ' anybody back.
    If IsNumeric(line) Then attempts = CLng(line)
  End If
  Err.Clear
  On Error GoTo 0
End If

' Only reset the count when something was actually put back - zeroing it regardless turns "there
' is nothing to go back to" into an endless 1, 2, 3, "putting the previous version back" cycle.
If Not fso.FileExists(prog) Or attempts >= 3 Then
  restored = False
  For Each folder In fso.GetFolder(here).SubFolders
    If Left(folder.Name, 11) = ".fm-backup-" Then
      If fso.FileExists(folder.Path & "\fm-serve.exe") Then
        If fso.FolderExists(here & "\program") Then fso.DeleteFolder here & "\program", True
        fso.MoveFolder folder.Path, here & "\program"
        If fso.FileExists(here & "\.fm-attempts") Then fso.DeleteFile here & "\.fm-attempts"
        attempts = 0
        restored = True
        Exit For
      End If
    End If
  Next
  If Not restored Then
    MsgBox "formicaria will not start properly and there is no earlier version in this folder." & vbCrLf & vbCrLf & _
           "Download it again from:" & vbCrLf & _
           "https://github.com/formicaria-org/formicaria/releases/latest" & vbCrLf & vbCrLf & _
           "Your notes are in the 'vault' folder beside this file and have not been touched.", _
           vbExclamation, "formicaria"
    If Not fso.FileExists(prog) Then WScript.Quit 1
  End If
End If

' Count this start. Cleared by formicaria itself once it is up, so it only accumulates across
' starts that failed.
If fso.FileExists(prog) Then
  On Error Resume Next
  Set ts = fso.CreateTextFile(here & "\.fm-attempts", True)
  If Err.Number = 0 Then
    ts.WriteLine attempts + 1
    ts.Close
  End If
  Err.Clear
  On Error GoTo 0
End If

If Not fso.FolderExists(vault) Then fso.CreateFolder vault

sh.CurrentDirectory = here
' 0 = no window; False = do not wait for it to finish.
sh.Run """" & here & "\program\fm-serve.exe""", 0, False
