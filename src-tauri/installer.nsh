; Marq NSIS installer hooks.
; Registers Marq with Windows (per-user, no admin) so it appears in
; "Open with" and Settings > Default apps, then offers to make it the
; default Markdown viewer. Windows requires the final choice to happen
; in Settings / the Open-with dialog - installers cannot set it silently.

!macro NSIS_HOOK_POSTINSTALL
  ; ProgID
  WriteRegStr HKCU "Software\Classes\Marq.Markdown" "" "Markdown document"
  WriteRegStr HKCU "Software\Classes\Marq.Markdown\DefaultIcon" "" '"$INSTDIR\Marq.exe",0'
  WriteRegStr HKCU "Software\Classes\Marq.Markdown\shell\open" "" "Open with Marq"
  WriteRegStr HKCU "Software\Classes\Marq.Markdown\shell\open\command" "" '"$INSTDIR\Marq.exe" "%1"'

  ; Offer Marq in the Open-with list for every Markdown extension
  WriteRegStr HKCU "Software\Classes\.md\OpenWithProgids" "Marq.Markdown" ""
  WriteRegStr HKCU "Software\Classes\.markdown\OpenWithProgids" "Marq.Markdown" ""
  WriteRegStr HKCU "Software\Classes\.mdown\OpenWithProgids" "Marq.Markdown" ""
  WriteRegStr HKCU "Software\Classes\.mkd\OpenWithProgids" "Marq.Markdown" ""

  ; Default-apps registration (Settings > Default apps > Marq)
  WriteRegStr HKCU "Software\Marq\Capabilities" "ApplicationName" "Marq"
  WriteRegStr HKCU "Software\Marq\Capabilities" "ApplicationDescription" "Markdown, beautifully typeset"
  WriteRegStr HKCU "Software\Marq\Capabilities\FileAssociations" ".md" "Marq.Markdown"
  WriteRegStr HKCU "Software\Marq\Capabilities\FileAssociations" ".markdown" "Marq.Markdown"
  WriteRegStr HKCU "Software\Marq\Capabilities\FileAssociations" ".mdown" "Marq.Markdown"
  WriteRegStr HKCU "Software\Marq\Capabilities\FileAssociations" ".mkd" "Marq.Markdown"
  WriteRegStr HKCU "Software\RegisteredApplications" "Marq" "Software\Marq\Capabilities"

  ; Tell Explorer the association set changed
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'

  ; Interactive option: open Windows Settings on Marq's default-apps page
  IfSilent marq_skip_default
  MessageBox MB_YESNO|MB_ICONQUESTION "Make Marq your default Markdown viewer?$\r$\n$\r$\nWindows Settings will open - pick Marq under '.md'." IDNO marq_skip_default
    ExecShell "open" "ms-settings:defaultapps?registeredAppUser=Marq"
  marq_skip_default:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\Marq.Markdown"
  DeleteRegValue HKCU "Software\Classes\.md\OpenWithProgids" "Marq.Markdown"
  DeleteRegValue HKCU "Software\Classes\.markdown\OpenWithProgids" "Marq.Markdown"
  DeleteRegValue HKCU "Software\Classes\.mdown\OpenWithProgids" "Marq.Markdown"
  DeleteRegValue HKCU "Software\Classes\.mkd\OpenWithProgids" "Marq.Markdown"
  DeleteRegKey HKCU "Software\Marq"
  DeleteRegValue HKCU "Software\RegisteredApplications" "Marq"
  ; Autostart entry written by the app on first run
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Marq"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend
