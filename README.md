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

- Windows
- [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) installé (projet archivé par son auteur,
  mais toujours fonctionnel)
- Rust (édition 2024)

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
- `src/app.rs`, `src/widgets.rs` : interface

## Limites actuelles

- La vibration et le mode DualShock 4 sont couverts par des tests unitaires, pas encore éprouvés
  avec une vraie manette. La vibration Bluetooth n'a jamais été essayée.
- Pas de vibration en mode DualShock 4 : `vigem-client` n'expose pas les notifications de vibration
  de la DS4.
- Gyroscope et gâchettes adaptatives ignorés ; pas de pavé tactile en mode Xbox.

## Tests

```bash
cargo test
```
