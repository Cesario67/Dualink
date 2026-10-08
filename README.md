# Dualink

Utilise une manette DualSense comme une manette Xbox 360 ou DualShock 4 sous Windows, pour les jeux
qui ne reconnaissent pas la DualSense.

## Fonctionnement

1. Un thread de fond lit la DualSense en HID brut (`hidapi`), USB ou Bluetooth.
2. Il crée une manette virtuelle (Xbox 360 ou DualShock 4, au choix dans la fenêtre) via le driver
   ViGEmBus (`vigem-client`) et y copie chaque changement d'état (boutons, sticks, gâchettes, croix
   directionnelle, pavé tactile en DS4).
3. En mode Xbox 360, la vibration demandée par les jeux est relayée à la DualSense (rapports de
   sortie USB et Bluetooth).
4. La fenêtre affiche le statut, un interrupteur d'émulation, le choix du mode et un visualiseur
   d'entrées en direct.

La manette virtuelle n'existe que tant que la vraie est connectée. Si la DualSense est débranchée
puis rebranchée, la connexion reprend toute seule. Changer de mode débranche et recrée la manette
virtuelle.

## Masquage de la vraie manette (HidHide)

Si [HidHide](https://github.com/nefarius/HidHide) est installé, Dualink peut cacher la DualSense
physique aux autres applications tant que la manette virtuelle est active, pour éviter les doublons
dans Steam et les jeux. Il passe par `HidHideCLI.exe`, autorise `dualink.exe` à continuer de la voir,
et ne retire à la fermeture que ce qu'il a lui-même ajouté (la configuration de HidHide est partagée
avec DSX ou DS4Windows). Ses ajouts sont consignés dans
`%LOCALAPPDATA%\Dualink\hidhide-undo.txt` : si Dualink est tué avant d'avoir restauré, le prochain
lancement défait les changements. HidHide reste optionnel.

Lancez Dualink avant Steam et les jeux : HidHide empêche d'ouvrir la manette, il ne ferme pas les
accès déjà pris.

## Installation (utilisateurs)

Chaque [release](https://github.com/Cesario67/Dualink/releases) propose deux fichiers au choix :

- **`Dualink-Setup-v<version>.exe`** : l'installateur. Il vérifie ViGEmBus (obligatoire) et HidHide (recommandé), installe ceux qui manquent à partir des installateurs officiels qu'il embarque (aucune connexion Internet nécessaire), puis installe Dualink avec ses raccourcis et un désinstalleur. À la désinstallation, Dualink est retiré de la liste de HidHide ; les pilotes restent en place.
- **`Dualink-v<version>-windows-x64.exe`** : l'application seule, à lancer directement. Sa section « Installation » (voir plus bas) permet d'installer ViGEmBus et HidHide s'ils manquent.

Le runtime Visual C++ n'est **pas** nécessaire : l'exécutable est compilé avec le runtime C intégré (`.cargo/config.toml`).

## Prérequis

| Composant | Rôle | Obligatoire |
|---|---|---|
| Windows 10 ou 11 (64 bits) | La DualSense n'a besoin d'aucun pilote : Windows la gère en USB et en Bluetooth | oui |
| [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) | Pilote qui crée les manettes virtuelles Xbox 360 / DualShock 4 (projet archivé par son auteur, mais toujours fonctionnel) | oui |
| [HidHide](https://github.com/nefarius/HidHide/releases) | Masque la vraie DualSense aux autres applications (évite les doublons dans Steam et les jeux) | non |
| Un pilote graphique avec OpenGL | Affichage de la fenêtre (egui) ; une machine virtuelle sans accélération peut poser problème | oui |
| Rust (édition 2024) | Uniquement pour compiler depuis les sources | non |

### Section « Installation » de l'application

La fenêtre contient une section **Installation** (ouverte automatiquement quand un composant manque).
Elle affiche l'état de ViGEmBus et de HidHide (Installé / Manquant, vérifié en continu) avec un bouton
**Installer** par composant et **Tout installer**. Un clic lance une copie de Dualink avec les droits
administrateur (une seule invite d'autorisation Windows), qui installe en silencieux, dans l'ordre :

1. l'installateur du dossier `redist\` à côté de `dualink.exe`, s'il existe (hors ligne) ;
2. sinon l'installateur officiel téléchargé depuis GitHub avec `curl.exe` (fourni avec Windows 10/11),
   dont le **SHA256 est vérifié** avant exécution (connexion Internet nécessaire) ;
3. en dernier recours `winget` : `ViGEm.ViGEmBus` et `Nefarius.HidHide`.

L'état se met à jour tout seul une fois l'installation terminée. HidHide peut demander un
redémarrage. Les traces sont dans `%LOCALAPPDATA%\Dualink\install.log`. Les URL et empreintes sont
dans `src/deps.rs` (et dans `scripts/package-release.ps1` pour l'installateur).

## Construire les fichiers de release

`scripts\package-release.ps1` construit dans `dist\` :

- `Dualink-Setup-v<version>.exe` : l'installateur (script Inno Setup dans `installer\dualink.iss`, nécessite [Inno Setup 6](https://jrsoftware.org/isinfo.php)). Il embarque les installateurs ViGEmBus 1.22.0 et HidHide 1.5.230, téléchargés depuis les dépôts officiels et vérifiés par leur SHA256, et leurs licences (BSD-3-Clause et MIT) ;
- `Dualink-v<version>-windows-x64.exe` : l'application seule ;
- `SHA256SUMS.txt`.

Sans Inno Setup, seule l'application seule est produite.

```bash
powershell -ExecutionPolicy Bypass -File scripts\package-release.ps1
```

La version est celle de `Cargo.toml` (ou `-Version x.y.z`).

Pour mettre à jour un composant, changez l'URL et l'empreinte dans le script. Le dossier `dist\` n'est
pas versionné. Un tag `vX.Y.Z` (qui doit correspondre à la version de `Cargo.toml`) déclenche le workflow `.github/workflows/release.yml`, qui construit et publie la release.


## Utilisation

```bash
cargo run --release
```

## Structure

- `src/dualsense.rs` : ouverture de la manette, décodage des rapports d'entrée, envoi de la vibration
- `src/output_report.rs` : rapports de sortie DualSense (USB, Bluetooth avec CRC32)
- `src/virtual_pad.rs` : choix du mode et manette virtuelle commune
- `src/xbox.rs`, `src/ds4.rs` : manettes virtuelles et mapping DualSense vers Xbox 360 / DualShock 4
- `src/bridge.rs` : thread de fond, état partagé, reconnexion
- `src/hidhide.rs` : masquage de la DualSense via HidHideCLI, journal de restauration
- `src/elevate.rs`, `src/reset.rs`, `src/deps.rs` : actions ponctuelles en administrateur (réinitialiser la
  manette, installer ViGEmBus et HidHide)
- `src/app.rs`, `src/widgets.rs` : interface
- `installer/dualink.iss` : script de l'installateur Inno Setup

## Limites actuelles

- La vibration et le mode DualShock 4 sont couverts par des tests unitaires, pas encore éprouvés
  avec une vraie manette. La vibration Bluetooth n'a jamais été essayée.
- Pas de vibration en mode DualShock 4 : `vigem-client` n'expose pas les notifications de vibration
  de la DS4.
- L'installation en un clic (depuis `redist\` ou winget) et le bouton « Réinitialiser la manette » n'ont
  pas été essayés de bout en bout (ils passent par une invite administrateur). Les options
  d'installation silencieuse `/quiet /norestart` sont celles de WiX Burn, déduites de l'analyse des
  installateurs, jamais exécutées. Sur un Windows vierge, rien n'a été testé.
- Gyroscope et gâchettes adaptatives ignorés ; pas de pavé tactile en mode Xbox.

## Tests

```bash
cargo test
```
