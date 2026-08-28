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
Dim sh, fso, here, vault

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
End With

If Not fso.FolderExists(vault) Then fso.CreateFolder vault

sh.CurrentDirectory = here
' 0 = no window; False = do not wait for it to finish.
sh.Run """" & here & "\fm-serve.exe""", 0, False
