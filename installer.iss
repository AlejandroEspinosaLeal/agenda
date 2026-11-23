[Setup]
AppId={{8E3C6B42-7A11-4F92-A9C4-2D8F110B55E1}
AppName=Mi Agenda
AppVersion=1.1.2
AppPublisher=Mi Agenda
DefaultDirName={localappdata}\Programs\Mi Agenda
DefaultGroupName=Mi Agenda
PrivilegesRequired=lowest
OutputDir=target\installer
OutputBaseFilename=Mi_Agenda_Setup
SetupIconFile=assets\icon.ico
UninstallDisplayIcon={app}\Mi_Agenda.exe
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=yes
CloseApplications=yes

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear acceso directo en el Escritorio"; GroupDescription: "Opciones adicionales:"
Name: "autostart"; Description: "Iniciar automáticamente con Windows (en la bandeja junto al reloj)"; GroupDescription: "Opciones adicionales:"

[Files]
Source: "target\release\organizador_tareas.exe"; DestDir: "{app}"; DestName: "Mi_Agenda.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Mi Agenda"; Filename: "{app}\Mi_Agenda.exe"
Name: "{autodesktop}\Mi Agenda"; Filename: "{app}\Mi_Agenda.exe"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "MiAgenda"; ValueData: """{app}\Mi_Agenda.exe"" --autostart"; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\Mi_Agenda.exe"; Description: "Abrir Mi Agenda ahora"; Flags: nowait postinstall
