#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Backend de référence pour miniteld — le « cerveau » du Minitel.

Implémente les quatre routes attendues par le daemon (cf. AGENTS.md) en
bibliothèque standard uniquement, sauf pour l'appel LLM qui utilise le SDK
Anthropic si présent :

    GET /health              -> {"ok": true, "net": true}
    GET /ask?q=...&cont=1    -> {"text": "..."}   (ou {"error": "..."})
    GET /service?name=...    -> {"text": "..."}
    GET /reset               -> {"ok": true}

Le contrat impose une seule chose au-delà du JSON : **le texte renvoyé doit
tenir sur 40 colonnes**. C'est le backend qui met en forme, pas le daemon.

Lancement :

    pip install anthropic                  # facultatif : sinon mode écho
    export ANTHROPIC_API_KEY=sk-ant-...
    python3 backend.py                     # écoute sur 0.0.0.0:3009

Ce fichier est un point de départ à copier et tordre dans tous les sens : la
partie intéressante est `answer()`, tout le reste est de la plomberie HTTP.
"""
import json
import os
import socket
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

PORT = int(os.environ.get("PORT", "3009"))

# --- LLM (facultatif) -------------------------------------------------------
# Sans clé API ni SDK, le backend répond en mode écho : de quoi valider la
# chaîne série et l'affichage avant de brancher un vrai modèle.
MODEL = os.environ.get("MODEL", "claude-haiku-4-5-20251001")
try:
    import anthropic

    _client = anthropic.Anthropic() if os.environ.get("ANTHROPIC_API_KEY") else None
except ImportError:
    _client = None

# Le Minitel affiche 40 colonnes × 24 lignes. On demande au modèle de produire
# directement du texte à cette taille : le reformater après coup casserait les
# listes et les tableaux qu'il aurait pu vouloir dessiner.
SYSTEM = """Tu réponds sur un Minitel : écran de 40 colonnes, 20 lignes utiles.

Règles absolues :
- Jamais plus de 40 caractères par ligne.
- Pas de Markdown (ni **, ni #, ni tableaux) : le Minitel ne le rend pas.
- Va droit au fait. Trois phrases valent mieux qu'un paragraphe.
- Chiffres et noms propres d'abord, explications ensuite.
- Utilise des MAJUSCULES pour un titre, jamais pour une phrase entière.
"""

# Fil de conversation. Un seul Minitel = un seul fil, gardé en mémoire ;
# `cont=0` (touche Envoi depuis l'accueil) ou /reset le remet à zéro.
HISTORY: list[dict] = []
MAX_TURNS = 6  # au-delà, le contexte coûte plus qu'il n'apporte


def answer(query: str, cont: bool) -> str:
    """Produit la réponse affichée. C'est ici que vous branchez votre logique."""
    global HISTORY
    if not cont:
        HISTORY = []

    if _client is None:
        return (
            "MODE ECHO\n"
            f"Vous avez tape : {query[:80]}\n\n"
            "Pas de cle ANTHROPIC_API_KEY :\n"
            "le backend renvoie l'entree telle\n"
            "quelle. Voir examples/backend/."
        )

    HISTORY.append({"role": "user", "content": query})
    msg = _client.messages.create(
        model=MODEL,
        max_tokens=800,
        system=SYSTEM,
        messages=HISTORY[-MAX_TURNS * 2 :],
    )
    text = "".join(b.text for b in msg.content if b.type == "text").strip()
    HISTORY.append({"role": "assistant", "content": text})
    return text


def service(name: str) -> str:
    """Sert une entrée du menu Guide (`services.json` côté daemon).

    Les noms sont les vôtres : branchez ici vos APIs, votre domotique, votre
    base. Ce squelette montre juste la forme attendue.
    """
    if name == "meteo":
        return "METEO\n\nBranchez ici votre API meteo."
    if name == "actus":
        return "ACTUALITES\n\nBranchez ici un flux RSS."
    return f"Service inconnu : {name}"


# --- sonde Internet ---------------------------------------------------------
# Le daemon affiche WEB OK / WEB KO / SRV KO d'après ce booléen : il distingue
# « backend injoignable » de « backend sans Internet », ce qui évite de courir
# après le mauvais problème. Mise en cache : la sonde est appelée en boucle.
_net_cache = (0.0, False)


def internet_up() -> bool:
    global _net_cache
    ts, val = _net_cache
    if time.time() - ts < 20:
        return val
    try:
        socket.create_connection(("1.1.1.1", 443), timeout=3).close()
        val = True
    except OSError:
        val = False
    _net_cache = (time.time(), val)
    return val


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"  # le client Rust parle HTTP/1.0, sans keep-alive

    def _json(self, payload: dict, code: int = 200) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802 (nom imposé par BaseHTTPRequestHandler)
        url = urlparse(self.path)
        qs = parse_qs(url.query)

        if url.path == "/health":
            self._json({"ok": True, "net": internet_up()})
            return

        if url.path == "/reset":
            HISTORY.clear()
            self._json({"ok": True})
            return

        if url.path == "/ask":
            q = (qs.get("q") or [""])[0].strip()
            if not q:
                self._json({"error": "requete vide"})
                return
            # Une exception ici ne doit pas tuer la connexion : le daemon
            # attend du JSON, et `error` s'affiche proprement sur l'écran.
            try:
                self._json({"text": answer(q, cont=qs.get("cont") == ["1"])})
            except Exception as e:  # noqa: BLE001
                print(f"ask: {e}", file=sys.stderr)
                self._json({"error": str(e)[:120]})
            return

        if url.path == "/service":
            name = (qs.get("name") or [""])[0]
            try:
                self._json({"text": service(name)})
            except Exception as e:  # noqa: BLE001
                self._json({"error": str(e)[:120]})
            return

        self._json({"error": "route inconnue"}, code=404)

    def log_message(self, fmt: str, *args) -> None:
        print(f"{self.address_string()} {fmt % args}", file=sys.stderr)


if __name__ == "__main__":
    mode = "LLM" if _client else "echo (pas de ANTHROPIC_API_KEY)"
    print(f"backend miniteld sur 0.0.0.0:{PORT} — mode {mode}", file=sys.stderr)
    ThreadingHTTPServer(("0.0.0.0", PORT), Handler).serve_forever()
