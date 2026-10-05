# Dualink

Utilise une manette DualSense comme une manette Xbox 360 sous Windows, pour les jeux qui ne
reconnaissent que XInput.

Premier jet : passthrough simple, sans HidHide.

## Fonctionnement

1. Lecture de la DualSense en HID brut (`hidapi`), USB ou Bluetooth.
2. Création d'une manette Xbox 360 virtuelle via le driver ViGEmBus (`vigem-client`).
3. Conversion de chaque changement d'état (boutons, sticks, gâchettes, croix directionnelle).

## Prérequis

- Windows
- [ViGEmBus](https://github.com/nefarius/ViGEmBus/releases) installé (projet archivé par son auteur,
  mais toujours fonctionnel)
- Rust (édition 2024)

## Utilisation

```bash
cargo run --release
```

Branchez la DualSense avant de lancer. Ctrl+C pour arrêter (la manette virtuelle disparaît).

## Limites actuelles

- Pas de HidHide : les jeux voient la vraie DualSense en plus de la manette virtuelle (doubles
  entrées possibles). Prochaine étape.
- Pas de retour de vibration vers la DualSense.
- Gyroscope, touchpad et gâchettes adaptatives ignorés.
- Pas d'émulation DS4.
- Pas de détection de reconnexion : relancer le programme.

## Tests

```bash
cargo test
```
