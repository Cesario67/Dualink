# Dualink

Utilise une manette DualSense comme une manette Xbox 360 sous Windows, pour les jeux qui ne
reconnaissent que XInput.

Passthrough simple avec une petite interface (egui), sans HidHide pour l'instant.

## Fonctionnement

1. Un thread de fond lit la DualSense en HID brut (`hidapi`), USB ou Bluetooth.
2. Il crée une manette Xbox 360 virtuelle via le driver ViGEmBus (`vigem-client`) et y copie chaque
   changement d'état (boutons, sticks, gâchettes, croix directionnelle).
3. La fenêtre affiche le statut, un interrupteur d'émulation et un visualiseur d'entrées en direct.

La manette virtuelle n'existe que tant que la vraie est connectée. Si la DualSense est débranchée
puis rebranchée, la connexion reprend toute seule.

## Masquage de la vraie manette (HidHide)

Si [HidHide](https://github.com/nefarius/HidHide) est installé, Dualink peut cacher la DualSense
physique aux autres applications tant que la manette virtuelle est active, pour éviter les doublons
dans Steam et les jeux. Il passe par `HidHideCLI.exe`, autorise `dualink.exe` à continuer de la voir,
et ne retire à la fermeture que ce qu'il a lui-même ajouté (la configuration de HidHide est partagée
avec DSX ou DS4Windows). Ses ajouts sont consignés dans
`%LOCALAPPDATA%\Dualink\hidhide-undo.txt` : si Dualink est tué avant d'avoir restauré, le prochain
lancement défait les changements. HidHide reste optionnel.

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

- `src/dualsense.rs` : ouverture de la manette et décodage des rapports HID
- `src/xbox.rs` : manette virtuelle et mapping DualSense vers Xbox 360
- `src/bridge.rs` : thread de fond, état partagé, reconnexion
- `src/hidhide.rs` : masquage de la DualSense via HidHideCLI, journal de restauration
- `src/app.rs`, `src/widgets.rs` : interface

## Limites actuelles

- Le masquage via HidHide n'a pas encore été éprouvé sur une vraie machine. Il peut demander les
  droits administrateur (l'erreur s'affiche dans la fenêtre). Les programmes qui tenaient déjà la
  manette ouverte (Steam, un jeu) doivent être relancés pour qu'elle disparaisse pour eux.
- Pas de retour de vibration vers la DualSense.
- Gyroscope, touchpad et gâchettes adaptatives ignorés.
- Pas d'émulation DS4.

## Tests

```bash
cargo test
```
