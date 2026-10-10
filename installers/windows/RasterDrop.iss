#ifndef AppVersion
  #error AppVersion must be provided by the packaging script
#endif
#ifndef PayloadDir
  #error PayloadDir must be provided by the packaging script
#endif
#ifndef BootstrapperPath
  #error BootstrapperPath must be provided by the packaging script
#endif

[Setup]
AppId={{B13B3B77-639F-4B2A-9A83-34492A6A7B13}
AppName=RasterDrop
AppVersion={#AppVersion}
AppPublisher=RasterDrop Contributors
AppPublisherURL=https://github.com/ealitt/RasterDrop
DefaultDirName={localappdata}\Programs\RasterDrop
DefaultGroupName=RasterDrop
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputBaseFilename=RasterDrop-{#AppVersion}-windows-x64-setup
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\RasterDrop.exe
LicenseFile={#PayloadDir}\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BootstrapperPath}"; DestName: "MicrosoftEdgeWebview2Setup.exe"; Flags: dontcopy

[Icons]
Name: "{group}\RasterDrop"; Filename: "{app}\RasterDrop.exe"
Name: "{group}\Uninstall RasterDrop"; Filename: "{uninstallexe}"
Name: "{autodesktop}\RasterDrop"; Filename: "{app}\RasterDrop.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\RasterDrop.exe"; Description: "Launch RasterDrop"; Flags: nowait postinstall skipifsilent

[Code]
function HasRuntimeVersion(RootKey: Integer): Boolean;
var
  Version: String;
begin
  Result := RegQueryStringValue(RootKey,
    'Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'pv', Version) and (Version <> '') and (Version <> '0.0.0.0');
end;

function WebView2Installed: Boolean;
begin
  Result := HasRuntimeVersion(HKLM32) or HasRuntimeVersion(HKLM64) or
    HasRuntimeVersion(HKCU32) or HasRuntimeVersion(HKCU64);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ExitCode: Integer;
begin
  Result := '';
  if WebView2Installed then
    Exit;
  ExtractTemporaryFile('MicrosoftEdgeWebview2Setup.exe');
  WizardForm.StatusLabel.Caption := 'Installing Microsoft Edge WebView2 Runtime (internet access required)...';
  if not Exec(ExpandConstant('{tmp}\MicrosoftEdgeWebview2Setup.exe'),
    '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) then
  begin
    Result := 'Unable to start Microsoft Edge WebView2 setup. Please install WebView2 Runtime and run setup again.';
    Exit;
  end;
  if (ExitCode <> 0) and (ExitCode <> 3010) then
  begin
    Result := 'Microsoft Edge WebView2 installation failed. Check your internet connection and run setup again.';
    Exit;
  end;
  NeedsRestart := ExitCode = 3010;
  if not WebView2Installed then
    Result := 'Microsoft Edge WebView2 Runtime is not available. Please install it and run setup again.';
end;
