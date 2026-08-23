#!/usr/bin/env bash
# État de santé du montage Minitel en un coup d'œil, depuis le poste de travail.
#
#   ./tools/minitel-status.sh pi@192.168.1.42            # tout : daemon + hôte
#   ./tools/minitel-status.sh 192.168.1.42:3010          # juste l'API (pas de SSH)
#
# Variables :
#   CTRL_PORT  port de l'API de contrôle (défaut 3010)
#   SERVICE    nom de l'unité systemd    (défaut miniteld)
#
# Sorties volontairement bornées (pas de -f, pas de TUI) : le script est fait
# pour être lu tel quel — par un humain ou collé dans un agent de code.
set -euo pipefail

CTRL_PORT=${CTRL_PORT:-3010}
SERVICE=${SERVICE:-miniteld}
TARGET=${1:?usage: minitel-status.sh <user@hote | hote:port>}

if [[ "$TARGET" == *:* ]]; then
  # forme hote:port → uniquement l'API de contrôle
  CTRL="$TARGET"
  SSH_HOST=""
else
  SSH_HOST="$TARGET"
  CTRL="${TARGET#*@}:$CTRL_PORT"
fi

echo "── API de contrôle ($CTRL)"
if OUT=$(curl -sS --max-time 5 "http://$CTRL/status" 2>&1); then
  # jq si présent, sinon le JSON brut (il tient sur une ligne)
  echo "$OUT" | { command -v jq >/dev/null && jq . || cat; }
  case "$OUT" in
    *'"connected":true'*)  echo "   lien série : OK" ;;
    *'"connected":false'*) echo "   lien série : COUPÉ — Minitel éteint, câble, ou mauvais device" ;;
  esac
  case "$OUT" in
    *'"net":"online"'*)  echo "   backend    : OK" ;;
    *'"net":"noweb"'*)   echo "   backend    : joignable mais SANS Internet" ;;
    *'"net":"offline"'*) echo "   backend    : INJOIGNABLE — vérifier MINITEL_BACKEND" ;;
  esac
else
  echo "   ✗ injoignable : $OUT"
  echo "   → daemon arrêté, mauvaise IP, ou port $CTRL_PORT filtré"
fi

[ -z "$SSH_HOST" ] && exit 0

echo
echo "── Hôte ($SSH_HOST)"
ssh -o ConnectTimeout=6 "$SSH_HOST" "
  echo \"   service    : \$(systemctl is-active $SERVICE 2>/dev/null || echo inconnu) (\$(systemctl show -p NRestarts --value $SERVICE 2>/dev/null || echo '?') redémarrages)\"
  echo \"   port série : \$(ls /dev/serial/by-id/ 2>/dev/null | head -3 | tr '\n' ' ' || echo AUCUN)\"
  # Sur Raspberry Pi : 0x0 = alimentation saine. Tout autre code = sous-tension
  # ou throttling, LA cause n°1 des glitches série (cf. docs/journal-de-bord.md).
  command -v vcgencmd >/dev/null && echo \"   throttled  : \$(vcgencmd get_throttled)\"
  echo
  echo '── Dernières lignes du journal'
  journalctl -u $SERVICE -n 15 --no-pager -o short 2>/dev/null | tail -15
"
