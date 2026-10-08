; Installateur de Dualink (Inno Setup 6).
;
; Compilation : scripts\package-release.ps1 (passe les paramètres ci-dessous), ou à la main :
;   ISCC /DAppVersion=0.5.0 /DStageDir=<dossier> /DViGEmFile=<fichier> /DHidHideFile=<fichier> installer\dualink.iss
;
; Ce que fait l'installateur : vérifie ViGEmBus et HidHide, installe ceux qui manquent (installateurs
; officiels fournis dans le Setup, aucune connexion Internet nécessaire), puis installe Dualink.

#ifndef AppVersion
  #error AppVersion non défini (ex : /DAppVersion=0.5.0)
#endif
#ifndef StageDir
  #error StageDir non défini (dossier contenant dualink.exe, redist\ et THIRD_PARTY_NOTICES.txt)
#endif
#ifndef ViGEmFile
  #error ViGEmFile non défini
#endif
#ifndef HidHideFile
  #error HidHideFile non défini
#endif

[Setup]
AppId={{F68D772D-5B5C-4820-8660-135718188FAB}
AppName=Dualink
AppVersion={#AppVersion}
AppPublisher=Cesario67
AppPublisherURL=https://github.com/Cesario67/Dualink
AppSupportURL=https://github.com/Cesario67/Dualink/issues
DefaultDirName={autopf}\Dualink
DefaultGroupName=Dualink
DisableProgramGroupPage=yes
OutputDir=..\dist
OutputBaseFilename=Dualink-Setup-v{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; Windows 10 ou plus, 64 bits uniquement (ViGEmBus et HidHide sont fournis en x64).
MinVersion=10.0
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
; Installer des pilotes demande les droits administrateur : une seule invite, au lancement du Setup.
PrivilegesRequired=admin
UninstallDisplayIcon={app}\dualink.exe
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "desktopicon"; Description: "Créer un raccourci sur le Bureau"; Flags: unchecked
Name: "hidhide"; Description: "Installer HidHide (masque la vraie manette aux autres applications, recommandé)"; Check: not HidHideInstalled

[Files]
Source: "{#StageDir}\dualink.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StageDir}\THIRD_PARTY_NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
; Installateurs tiers : extraits à la demande dans le dossier temporaire, jamais copiés sur le disque.
Source: "{#StageDir}\redist\{#ViGEmFile}"; Flags: dontcopy
Source: "{#StageDir}\redist\{#HidHideFile}"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\Dualink"; Filename: "{app}\dualink.exe"
Name: "{autodesktop}\Dualink"; Filename: "{app}\dualink.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\dualink.exe"; Description: "Lancer Dualink"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; Retire Dualink de la liste des applications autorisées de HidHide. Les pilotes restent installés :
; d'autres programmes (DSX, DS4Windows) peuvent en dépendre.
Filename: "{commonpf64}\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe"; Parameters: "--app-unreg ""{app}\dualink.exe"""; Flags: runhidden skipifdoesntexist; RunOnceId: "HidHideUnregister"

[UninstallDelete]
Type: filesandordirs; Name: "{localappdata}\Dualink"

[Code]
const
  EXIT_SUCCESS_REBOOT_REQUIRED = 3010;

var
  RestartNeeded: Boolean;

function ViGEmInstalled: Boolean;
begin
  Result := RegKeyExists(HKLM64, 'SYSTEM\CurrentControlSet\Services\ViGEmBus');
end;

function HidHideInstalled: Boolean;
begin
  Result := FileExists(ExpandConstant('{commonpf64}\Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe'));
end;

{ Lance un installateur Burn en silencieux. Renvoie True s'il a réussi (0 ou 3010 = redémarrage requis). }
function RunRedist(const DisplayName, FileName: String): Boolean;
var
  ResultCode: Integer;
begin
  WizardForm.StatusLabel.Caption := 'Installation de ' + DisplayName + '...';
  WizardForm.ProgressGauge.Style := npbstMarquee;
  try
    ExtractTemporaryFile(FileName);
    Result := Exec(ExpandConstant('{tmp}\' + FileName), '/quiet /norestart', '', SW_HIDE, ewWaitUntilTerminated, ResultCode)
      and ((ResultCode = 0) or (ResultCode = EXIT_SUCCESS_REBOOT_REQUIRED));
  finally
    WizardForm.ProgressGauge.Style := npbstNormal;
  end;
  if Result and (ResultCode = EXIT_SUCCESS_REBOOT_REQUIRED) then
    RestartNeeded := True;
  if not Result then
    MsgBox(DisplayName + ' n''a pas pu être installé (code ' + IntToStr(ResultCode) + ').' + #13#10 +
      'Dualink sera installé quand même ; vous pourrez réessayer avec le bouton « Installer en un clic » de l''application.',
      mbError, MB_OK);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssInstall then
  begin
    if not ViGEmInstalled then
      RunRedist('ViGEmBus', '{#ViGEmFile}');
    if WizardIsTaskSelected('hidhide') and not HidHideInstalled then
      RunRedist('HidHide', '{#HidHideFile}');
  end;
end;

function NeedRestart: Boolean;
begin
  Result := RestartNeeded;
end;
