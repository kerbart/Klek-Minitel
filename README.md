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

## Étape 0 — votre machine, votre agent

Ce projet fonctionne depuis **Linux, macOS ou Windows** (via WSL2), et il est
conçu pour que la partie logicielle soit **entièrement déléguée à un agent de
code** : vous décrivez ce que vous avez, il compile, déploie et vérifie. Le
fichier [AGENTS.md](AGENTS.md) — un [standard ouvert](https://agents.md) que
tous ces outils lisent — lui donne les invariants matériels, les pièges, et
les règles de compilation croisée.

**→ [docs/demarrer-selon-votre-machine.md](docs/demarrer-selon-votre-machine.md)** :
ce qu'il faut installer selon votre OS, quel parcours choisir selon ce que vous
avez sous la main (rien / un Pi / juste un Minitel / une installation existante),
et **les prompts de départ à copier-coller** pour chaque cas.

Si vous n'avez encore jamais utilisé d'agent de code, c'est l'occasion — un
projet matériel, borné, vérifiable à l'œil, est un excellent terrain
d'apprentissage. Il vous faut l'un de ces harness (au choix, selon vos accès et
vos goûts — le projet n'en exige aucun en particulier) :

| Harness | Éditeur | Installation | Lien |
|---|---|---|---|
| **Claude Code** | Anthropic | `npm i -g @anthropic-ai/claude-code` | [claude.com/claude-code](https://claude.com/claude-code) |
| **Codex CLI** | OpenAI | `npm i -g @openai/codex` | [github.com/openai/codex](https://github.com/openai/codex) |
| **Gemini CLI** | Google | `npm i -g @google/gemini-cli` | [github.com/google-gemini/gemini-cli](https://github.com/google-gemini/gemini-cli) |
| **pi** | Mario Zechner | `npm i -g @mariozechner/pi-coding-agent` | [github.com/earendil-works/pi](https://github.com/earendil-works/pi) |
| **opencode** | SST | voir le site | [opencode.ai](https://opencode.ai) |
| **Goose** | Block / Linux Foundation | voir le site | [github.com/block/goose](https://github.com/block/goose) |
| **Aider** | communauté | `pip install aider-install` | [aider.chat](https://aider.chat) |
| **Copilot CLI** | GitHub | `npm i -g @github/copilot` | [github.com/github/copilot-cli](https://github.com/github/copilot-cli) |

L'écosystème évolue vite — annuaire tenu à jour :
[awesome-cli-coding-agents](https://github.com/bradAGI/awesome-cli-coding-agents).

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

### Pour vous, humain

| Fichier | Contenu |
|---------|---------|
| 🖥️ **[docs/demarrer-selon-votre-machine.md](docs/demarrer-selon-votre-machine.md)** | **Démarrer** : Linux/macOS/Windows, quel parcours selon ce que vous avez, quel agent de code, les prompts de départ |
| 🔌 **[docs/materiel-branchement.md](docs/materiel-branchement.md)** | **Brancher** : quoi acheter, le brochage DIN, identifier les broches au multimètre, le premier contact, la panne d'alimentation que tout le monde fait |
| 🔥 **[docs/materiel-soudure.md](docs/materiel-soudure.md)** | **Souder** : outillage, le piège du connecteur mâle (miroir !), le geste pas à pas, les 3 tests avant mise sous tension, rattraper un ratage |
| 🧩 **[docs/creer-un-module.md](docs/creer-un-module.md)** | **Créer un module** : les 3 niveaux (entrée de menu → backend complet → app Rust), le contrat exact, écrire pour 40 colonnes |
| [docs/install-raspberry-pi.md](docs/install-raspberry-pi.md) | Le pas-à-pas du flash de la carte au premier écran |
| [docs/journal-de-bord.md](docs/journal-de-bord.md) | 11 pannes réelles du montage — cause racine, fausse piste, résolution. **Six sur onze étaient physiques.** |

### Pour votre agent

| Fichier | Contenu |
|---------|---------|
| **[AGENTS.md](AGENTS.md)** | Le point d'entrée — pièges, invariants d'affichage, tests, conventions |
| [docs/videotex-1b-cheatsheet.md](docs/videotex-1b-cheatsheet.md) | Toutes les séquences hex du Minitel 1B (norme STUM) |
| [docs/image-conversion.md](docs/image-conversion.md) | Le pipeline image → `.vtx`, modes de rendu, timings |

## État du projet

✅ Fonctionnel et utilisé au quotidien. 37 tests couvrent les invariants d'affichage et de protocole, sans matériel (`cargo test`). Tout s'itère avant de câbler.

Le pilote cible le **Minitel 1B via la prise péri-informatique**. Les autres modèles partagent l'essentiel du protocole mais n'ont pas été testés — retours bienvenus.

## Licence

MIT — voir [LICENSE](LICENSE).

Les constantes de protocole s'inscrivent dans la lignée des implémentations Python du Minitel (PyMinitel et dérivés), revérifiées sur matériel. Merci à eux.
