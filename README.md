# Klek-Minitel

*Un vrai Minitel comme interface de vos services modernes.*

> 🚀 **Clonez ce repo et donnez son adresse à votre agent Claude Code.** AGENTS.md dit tout — le reste est pilotage entièrement automatable.

## 3 secondes pour comprendre

Un **vrai Minitel 1B** (achat brocante, ~30 €) devient l'interface de votre service.

```
┌──────────┐  série 1200/4800 bd  ┌──────────────┐   HTTP/1.0   ┌──────────┐
│ Minitel  │◄────────────────────►│  miniteld    │◄────────────►│ backend  │
│ 1B       │  DIN 5 ↔ USB-UART    │  (ce dépôt)  │  4 routes    │ (à vous) │
└──────────┘                      └──────────────┘              └──────────┘
```

Vous tapez sur le clavier d'époque, votre backend HTTP répond sur l'écran cathodique. Voilà. Pas de magie — juste du Rust solide, du Vidéotex correct, et un daemon découplé.

Le pilote ne contient **aucune logique métier** et n'accède pas à Internet : il appelle quatre routes (`/health`, `/ask`, `/service`, `/reset`). Tout l'intelligence vient de votre backend, dans le langage que vous voulez.

## Déployer : passer l'info minimaliste à un agent

Vous avez un Raspberry Pi, un Minitel 1B, et ~1 h de libre. Voici le prompt pour Claude Code :

```
Je veux deployer Klek-Minitel sur mon Pi. Voici ce que j'ai :

- Raspberry Pi : 192.168.1.42 (utilisateur "pi", j'ai la clé SSH)
- Minitel branche sur le port : /dev/serial/by-id/usb-1a86_USB_Serial-if00-port0
  (tu peux y aller direct, ou faire un ls /dev/serial/by-id/ si tu n'es pas sur)
- Mon backend : 192.168.1.10:3009 (j'ai les identifiants)
- Titre sur l'ecran : "MON SERVICE"

Je veux que ca fonctionne en 10 min. AGENTS.md a tout ce qu'il faut.

[Tu peux aussi mentionner : "Je n'ai pas de backend pour l'instant, lance juste
le backend de reference en mode écho"]
```

L'agent :
1. Lit AGENTS.md (1 min) → comprend le contrat du backend, l'architecture, les pièges matériels
2. Lance `deploy.sh` avec tes paramètres (2 min) → compile, déploie, vérifie
3. Vous affiche l'écran d'accueil qui s'allume ✅

C'est conçu **pour être piloté par un agent** — AGENTS.md est le manuel des invariants et des pièges que le compilateur ne peut pas vérifier. Lisez-le avant d'éditer le pilote.

## Piloter depuis votre poste : TUI ou API brute

Une fois déployé, vous avez deux voies :

**TUI du poste** (interface confortable) :
```bash
cargo run --release --features tui --bin minitel-tui -- 192.168.1.42:3010

# dans le champ de saisie :
#   bonjour depuis 2026        →  s'affiche sur le Minitel
#   /img photo.png --gray      →  convertie (img2vtx.py) puis affichée
#   /vtx logo.vtx              →  flux Vidéotex brut
```

**API brute** (si vous avez un script qui parle HTTP) :
```bash
curl -d "COUCOU DEPUIS L'API" 192.168.1.42:3010/text
curl --data-binary @image.vtx 192.168.1.42:3010/show
curl -s 192.168.1.42:3010/status   # lien, bauds, veille, backend
```

## Surveiller

```bash
./tools/minitel-status.sh pi@192.168.1.42
```

Affiche : l'API (connexion, bauds, veille, backend), le service systemd (restarts), le journal des 15 dernières lignes, et **l'état d'alimentation du Pi** (`vcgencmd get_throttled`) — c'est la cause racine la plus souvent cherchée au mauvais endroit.

## Votre premier backend : 2 min

Copier-coller le backend de référence et modifiez la fonction `respond()` :

```python
def respond(question: str, cont: int) -> str:
    # Votre logique : LLM, requête, calcul, ce que vous voulez
    answer = f"Vous avez écrit : {question}"
    return answer   # 40 colonnes max
```

Lancez `python3 examples/backend/backend.py`, l'agent déploie avec `BACKEND=votre_ip:3009`, c'est fait.

## Écrire du Rust pour le pilote

L'agent peut aussi vous aider là — il a AGENTS.md pour les invariants (40 colonnes, positionnement G1, double hauteur interdite en ligne 1…) et les 37 tests qui les verrouillent. Tout est itérable sans matériel : `cargo test`, `minitel-demo`, `vtx-preview.py`.

## Brancher vos modules

Trois approches, aucune ne demande de modifier ce dépôt :

1. **Menu rapide** : éditez `services.json`, une touche = une requête HTTP. Une météo, un flux RSS, l'état de vos serveurs.
2. **Backend personnalisé** : 4 routes HTTP, c'est tout — réimplémentez dans votre langage.
3. **Autre app Minitel** : le crate `minitel` (lien série, protocole, clavier, éditeur) est réutilisable seul.

## Documentation

| Fichier | Pour qui |
|---------|----------|
| **[AGENTS.md](AGENTS.md)** | Les agents IA qui vont toucher au code — pièges, invariants, tests, conventions |
| [docs/install-raspberry-pi.md](docs/install-raspberry-pi.md) | Vous : le pas-à-pas du flash de la carte au premier écran |
| [docs/journal-de-bord.md](docs/journal-de-bord.md) | Vous : 11 pannes réelles du montage (cause racine, fausse piste, résolution) |
| [docs/videotex-1b-cheatsheet.md](docs/videotex-1b-cheatsheet.md) | Les agents qui éditent du Vidéotex en Rust — toutes les séquences hex |
| [docs/image-conversion.md](docs/image-conversion.md) | Guide du pipeline image → `.vtx` |

## État du projet

✅ Fonctionnel et utilisé au quotidien. 37 tests couvrent les invariants d'affichage et de protocole, sans matériel (`cargo test`). Tout s'itère avant de câbler.

Le pilote cible le **Minitel 1B via la prise péri-informatique**. Les autres modèles partagent l'essentiel du protocole mais n'ont pas été testés — retours bienvenus.

## Licence

MIT — voir [LICENSE](LICENSE).

Les constantes de protocole s'inscrivent dans la lignée des implémentations Python du Minitel (PyMinitel et dérivés), revérifiées sur matériel. Merci à eux.
