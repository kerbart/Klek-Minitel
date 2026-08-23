# Installer sur un Raspberry Pi — le guide court

Objectif : un Minitel 1B qui affiche votre service, piloté par un Pi, en moins
d'une heure dont 45 minutes de flash de carte SD. Ce guide est volontairement
linéaire ; le pourquoi de chaque étape est dans [AGENTS.md](../AGENTS.md).

## 0. Ce qu'il vous faut

- un Minitel 1B (brocante, Leboncoin — vérifiez que l'écran s'allume) ;
- un Raspberry Pi, n'importe lequel (un Zero W suffit, le binaire pèse ~1 Mo) ;
- un adaptateur **USB-UART TTL** type CH340 (quelques euros) ;
- 3 fils Dupont et une prise **DIN 5 broches 270°** (ou 3 fils nus soigneux) ;
- sur votre poste : Rust (`rustup`), `python3`, un accès SSH au Pi.

## 1. Préparer le Pi (une fois)

1. Flashez Raspberry Pi OS **Lite** avec Raspberry Pi Imager ; dans les options,
   activez SSH et renseignez le Wi-Fi. Un OS 64 bits sur Pi 3/4/5, 32 bits sur
   Pi 1/Zero.
2. Démarrez, vérifiez `ssh pi@<ip-du-pi>`.
3. Ajoutez l'utilisateur au groupe du port série :

   ```bash
   ssh pi@<ip-du-pi> 'sudo usermod -aG dialout $USER'
   ```

4. **Si vous avez déjà activé l'overlayfs** (racine en lecture seule) dans
   `raspi-config` : désactivez-le le temps de l'installation, sinon tout
   déploiement s'évapore au reboot. `deploy.sh` le détecte et vous le rappellera.

## 2. Câbler (3 fils, pas 4)

Prise DIN à l'arrière du Minitel, vue de l'extérieur :

```
     3            broche 1 (RX Minitel)  ←  TX de l'adaptateur
   •   •          broche 2 (0 V)         ←  GND de l'adaptateur
 2 • 5 • 4        broche 3 (TX Minitel)  →  RX de l'adaptateur
     •
     1            broche 5 : 8,5 V — ⚠️ NE RIEN Y BRANCHER
```

Branchez l'adaptateur sur le Pi, puis identifiez le port :

```bash
ssh pi@<ip-du-pi> 'ls -l /dev/serial/by-id/'
# usb-1a86_USB_Serial-if00-port0 -> ../../ttyUSB0
```

Notez le chemin `/dev/serial/by-id/…` complet — c'est lui qu'on utilisera,
jamais `/dev/ttyUSB0` (le numéro change au gré des rebranchements).

## 3. Lancer un backend

Le daemon n'a aucune logique métier : il lui faut un backend HTTP (4 routes, le
contrat est dans [AGENTS.md](../AGENTS.md#contrat-du-backend)). Pour commencer,
le backend de référence en mode écho suffit — sur votre poste ou sur le Pi :

```bash
python3 examples/backend/backend.py &      # écoute sur :3009
```

## 4. Déployer

Depuis la racine du dépôt, sur votre poste :

```bash
HOST=pi@<ip-du-pi> \
BACKEND=<ip-du-backend>:3009 \
TITLE="MON SERVICE" \
DEV=/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0 \
./deploy.sh
```

Le script compile un binaire statique pour l'ARM du Pi (par défaut
`aarch64-unknown-linux-musl` ; Pi 32 bits → `TARGET=armv7-unknown-linux-musleabihf`,
Pi 1/Zero → `TARGET=arm-unknown-linux-musleabihf`), le copie, écrit l'unité
systemd `miniteld` et la démarre. Ni Docker, ni toolchain C, ni rien à installer
sur le Pi.

Allumez le Minitel : l'écran d'accueil apparaît, tapez une question, **Envoi**.

## 5. Vérifier et surveiller

```bash
./tools/minitel-status.sh pi@<ip-du-pi>     # état complet en un coup d'œil
```

ou à la main :

```bash
curl -s <ip-du-pi>:3010/status              # lien série, bauds, veille, backend
ssh pi@<ip-du-pi> journalctl -u miniteld -n 50 --no-pager
```

Et depuis votre poste, la télécommande TUI :

```bash
cargo run --release --features tui --bin minitel-tui -- <ip-du-pi>:3010
```

## Ça ne marche pas ?

Les dix pannes les plus probables et leur remède tiennent dans le tableau
[Dépannage d'AGENTS.md](../AGENTS.md#dépannage). Les causes racines vécues —
alimentation, câblage, glitches USB — sont racontées dans le
[journal de bord](journal-de-bord.md) : lisez-le avant de soupçonner le logiciel,
sur onze pannes majeures de ce montage, six étaient physiques.
