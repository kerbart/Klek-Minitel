#!/usr/bin/env bash
# Compile statiquement miniteld et l'installe en service sur une machine distante
# (typiquement le Raspberry Pi relié au Minitel).
#
#   HOST=pi@192.168.1.42 ./deploy.sh
#   HOST=pi@minitel.local TARGET=arm-unknown-linux-musleabihf ./deploy.sh
#
# Variables reconnues :
#   HOST     (obligatoire) user@hote SSH de la machine reliée au Minitel
#   TARGET   triplet Rust           (défaut aarch64-unknown-linux-musl)
#   BIN      binaire à déployer     (défaut miniteld)
#   DEV      port série sur l'hôte  (défaut : adaptateur CH340 par by-id)
#   BACKEND  ip:port du backend     (défaut 127.0.0.1:3009)
#   TITLE    titre de l'en-tête     (défaut MINITEL)
#   SERVICE  nom de l'unité systemd (défaut miniteld)
#
# Cibles musl utiles :
#   aarch64-unknown-linux-musl        Pi 3/4/5 en OS 64 bits
#   armv7-unknown-linux-musleabihf    Pi 2/3 en OS 32 bits
#   arm-unknown-linux-musleabihf      Pi 1 / Pi Zero W (ARMv6)
#
# On produit un binaire **statique musl** : aucune dépendance à installer sur la
# cible (pas de libudev, pas de glibc à la bonne version), et le lien se fait
# avec rust-lld — donc sans toolchain C croisée ni Docker. Voir .cargo/config.toml.
set -euo pipefail

HOST=${HOST:?Definissez HOST, ex. HOST=pi@192.168.1.42 ./deploy.sh}
TARGET=${TARGET:-aarch64-unknown-linux-musl}
BIN=${BIN:-miniteld}
DEV=${DEV:-/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0}
BACKEND=${BACKEND:-127.0.0.1:3009}
TITLE=${TITLE:-MINITEL}
SERVICE=${SERVICE:-miniteld}

USER_REMOTE=${HOST%@*}
HOME_REMOTE=/home/$USER_REMOTE
DIR_REMOTE=$HOME_REMOTE/minitel-driver
HERE="$(cd "$(dirname "$0")" && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"

echo "▶ cible $TARGET — installation de la toolchain si besoin…"
rustup target add "$TARGET" 2>/dev/null || true

echo "▶ build $BIN…"
cd "$HERE"
cargo build --release --target "$TARGET" --bin "$BIN"
ART="target/$TARGET/release/$BIN"
echo "▶ artefact : $(file -b "$ART" | cut -c1-60) ($(du -h "$ART" | cut -f1))"

echo "▶ attente de $HOST…"
until ssh -o ConnectTimeout=6 "$HOST" true 2>/dev/null; do sleep 5; done

# Piège classique sur Raspberry Pi OS : si la racine est en overlayfs (mode
# lecture seule pour épargner la carte SD), tout dépôt disparaît au reboot.
if [ "$(ssh "$HOST" 'findmnt -n -o FSTYPE / 2>/dev/null || true')" = "overlay" ]; then
  echo "✖ Racine en LECTURE SEULE (overlayfs) sur $HOST — le déploiement serait perdu au reboot."
  echo "  Désactiver :  ssh $HOST 'sudo raspi-config nonint disable_overlayfs && sudo reboot'"
  echo "  Déployer, puis réactiver si souhaité."
  exit 1
fi

echo "▶ copie du binaire dans $DIR_REMOTE…"
ssh "$HOST" "mkdir -p '$DIR_REMOTE'"
ssh "$HOST" "sudo systemctl stop $SERVICE 2>/dev/null || true"
scp "$ART" "$HOST:$DIR_REMOTE/$BIN"
ssh "$HOST" "chmod +x '$DIR_REMOTE/$BIN'"

# Menu de services : on n'écrase jamais celui déjà en place sur la cible.
if [ -f "$HERE/services.json" ]; then
  echo "▶ envoi de services.json…"
  scp "$HERE/services.json" "$HOST:$DIR_REMOTE/services.json"
fi

echo "▶ (ré)installation du service systemd $SERVICE…"
ssh "$HOST" "sudo tee /etc/systemd/system/$SERVICE.service >/dev/null <<UNIT
[Unit]
Description=miniteld — pilote Minitel (serie + Videotex)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$USER_REMOTE
WorkingDirectory=$DIR_REMOTE
Environment=MINITEL_BACKEND=$BACKEND
Environment=MINITEL_TITLE=$TITLE
Environment=MINITEL_SERVICES=$DIR_REMOTE/services.json
Environment=RUST_LOG=info
ExecStart=$DIR_REMOTE/$BIN $DEV
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
UNIT
sudo systemctl daemon-reload
sudo systemctl enable $SERVICE
sudo systemctl restart $SERVICE
systemctl is-active $SERVICE"

echo "✔ déployé. Log :  ssh $HOST journalctl -u $SERVICE -f"
