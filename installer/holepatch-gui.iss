#define MyAppName "HolePatch GUI"
#define MyAppExe "holepatch-gui.exe"
#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#define MyAppSourceDir "..\\target\\x86_64-pc-windows-msvc\\release"

[Setup]
AppId={{B2D9CF2D-6F2E-4B84-A6D8-8C8B1A8E7F10}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
DefaultDirName={autopf}\\{#MyAppName}
DefaultGroupName={#MyAppName}
OutputBaseFilename={#MyAppName}-Setup-v{#MyAppVersion}
OutputDir=..\\dist
Compression=lzma
SolidCompression=yes
WizardStyle=modern
DisableProgramGroupPage=no
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=admin

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "{#MyAppSourceDir}\\{#MyAppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\\{#MyAppName}"; Filename: "{app}\\{#MyAppExe}"
Name: "{autodesktop}\\{#MyAppName}"; Filename: "{app}\\{#MyAppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\\{#MyAppExe}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifdoesntexist skipifsilent
