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

## Prérequis

| Composant | Rôle | Obligatoire |
|---|---|---|
| Windows 10 ou 11 (64 bits) | La DualSense n'a besoin d'aucun pilote : Windows la gère en USB et en Bluetooth | oui |
| [Runtime Visual C++ 2015-2022 (x64)](https://aka.ms/vs/17/release/vc_redist.x64.exe) | Nécessaire pour lancer l'exécutable (compilé avec MSVC) | oui |
| [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) | Pilote qui crée les manettes virtuelles Xbox 360 / DualShock 4 (projet archivé par son auteur, mais toujours fonctionnel) | oui |
| [HidHide](https://github.com/nefarius/HidHide/releases) | Masque la vraie DualSense aux autres applications (évite les doublons dans Steam et les jeux) | non |
| Un pilote graphique avec OpenGL | Affichage de la fenêtre (egui) ; une machine virtuelle sans accélération peut poser problème | oui |
| Rust (édition 2024) | Uniquement pour compiler depuis les sources | non |

### Installation de ViGEmBus et HidHide en un clic

Si l'un des deux manque, Dualink affiche un panneau « Composants manquants » avec un bouton
**Installer en un clic**. Il lance une copie de Dualink avec les droits administrateur (une seule
invite d'autorisation Windows), qui installe ce qui manque en silencieux :

1. depuis le dossier `redist\` situé à côté de `dualink.exe` s'il existe (archive de release :
   **aucune connexion Internet nécessaire**) ;
2. sinon via `winget` (présent sur Windows 10/11 récents) : `ViGEm.ViGEmBus` et `Nefarius.HidHide`.

Dualink détecte ensuite tout seul l'installation terminée. HidHide peut demander un redémarrage. Les
traces sont dans `%LOCALAPPDATA%\Dualink\install.log`. Sans `redist\` ni winget, installez les
composants à la main depuis les liens ci-dessus.

## Archive de release

`scripts\package-release.ps1` construit `dist\Dualink-v<version>-windows-x64.zip` : `dualink.exe`,
`redist\` (installateurs ViGEmBus 1.22.0 et HidHide 1.5.230, téléchargés depuis les dépôts officiels
et vérifiés par leur SHA256), `THIRD_PARTY_NOTICES.txt` (licences BSD-3-Clause et MIT) et
`dist\SHA256SUMS.txt`.

```bash
powershell -ExecutionPolicy Bypass -File scripts\package-release.ps1 -Version 0.1.0
```

Pour mettre à jour un composant, changez l'URL et l'empreinte dans le script. Le dossier `dist\` n'est
pas versionné : l'archive s'envoie comme fichier de release GitHub.

Le runtime Visual C++ n'est pas installé par ce bouton (il faut déjà l'avoir pour lancer Dualink) :
`winget install --id Microsoft.VCRedist.2015+.x64 --exact` ou le lien du tableau.

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
