# Démarrer selon votre machine — Linux, macOS, Windows

Guide pour humain. Vous venez de cloner le dépôt : cette page vous dit **ce
qu'il faut installer sur votre ordinateur** et **quel chemin prendre selon votre
envie** — avant de confier la suite à un agent de code.

Le principe de ce dépôt : **vous décrivez, l'agent exécute.** Chaque parcours
ci-dessous se termine par un prompt à copier-coller. Vous n'êtes pas obligé de
travailler comme ça — tout est faisable à la main, les documents le détaillent —
mais c'est le mode d'emploi le plus court.

- [Choisir son parcours](#choisir-son-parcours)
- [Préparer sa machine : Linux](#linux)
- [Préparer sa machine : macOS](#macos)
- [Préparer sa machine : Windows](#windows)
- [Choisir son agent de code](#choisir-son-agent-de-code)
- [Les prompts de départ](#les-prompts-de-départ)

---

## Choisir son parcours

| Vous avez / vous voulez | Parcours | Il vous faut |
|---|---|---|
| **Rien du tout — juste voir si ça me plaît** | **A. Essai sans matériel** : compiler, lancer le daemon sur un faux device, dialoguer avec le backend d'exemple | Rust + Python. 15 min |
| **Un Minitel + un Raspberry Pi** | **B. L'installation complète** : cross-compiler depuis votre machine, déployer sur le Pi en SSH | Rust + Python + SSH vers le Pi. 1 h, câblage compris |
| **Un Minitel, pas de Pi** | **C. Direct sur votre ordinateur** : l'adaptateur USB-UART se branche sur votre machine, le daemon tourne dessus | Rust + Python. Le Pi n'est qu'une commodité, pas une exigence |
| **Un Minitel déjà installé (par vous ou quelqu'un d'autre)** | **D. Piloter** : le TUI et l'API de contrôle, pour écrire et envoyer des images à l'écran | Rust (pour le TUI) ou juste `curl` |

Quel que soit le parcours : commencez par **A**. Il valide toute la chaîne
logicielle en 15 minutes, sans rien risquer, et tous les autres parcours
s'appuient dessus.

---

## Linux

La plateforme d'origine du projet — tout fonctionne nativement.

```bash
# 1. Rust (si absent) — installe rustup, cargo, et le linker rust-lld
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Python 3 + git (exemple Debian/Ubuntu ; adaptez à votre distribution)
sudo apt install python3 python3-pip git
pip install Pillow        # uniquement pour la conversion d'images

# 3. Vérifier
git clone https://github.com/kerbart/Klek-Minitel && cd Klek-Minitel
cargo test                # 37 tests, aucun matériel requis
```

**Si vous branchez le Minitel directement sur cette machine** (parcours C) :
votre utilisateur doit appartenir au groupe propriétaire du port série —
`dialout` sur Debian/Ubuntu/Raspberry Pi OS, `uucp` sur Arch :

```bash
sudo usermod -aG dialout "$USER"    # puis se déconnecter/reconnecter
ls -l /dev/serial/by-id/            # votre adaptateur apparaît ici
```

---

## macOS

Tout fonctionne : la compilation, la **cross-compilation vers le Pi** (aucune
toolchain supplémentaire — voir l'encadré plus bas), le TUI, les outils Python.

```bash
# 1. Les outils en ligne de commande Apple (git, notamment)
xcode-select --install

# 2. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. Python 3 : celui du système suffit, sinon `brew install python3`
pip3 install Pillow

# 4. Vérifier
git clone https://github.com/kerbart/Klek-Minitel && cd Klek-Minitel
cargo test
```

**Si vous branchez le Minitel directement sur ce Mac** (parcours C) :

- Les adaptateurs CH340 sont reconnus nativement par les macOS récents
  (Big Sur et suivants). Si le port n'apparaît pas, le pilote du fabricant
  existe — cherchez « CH34x macOS driver ».
- Le port s'appelle `/dev/tty.usbserial-XXXX` (et non `/dev/serial/by-id/…`) :

  ```bash
  ls /dev/tty.usbserial-* /dev/tty.wchusbserial-*
  ./target/release/miniteld /dev/tty.usbserial-1420
  ```

- Pas de groupe `dialout` sur macOS : aucun réglage de permissions à faire.

> Honnêteté : le projet est développé et exploité sous Linux. La compilation et
> les tests passent sur macOS ; le lien série direct (parcours C) y est
> **plausible mais peu éprouvé**. Les retours sont bienvenus.

---

## Windows

**Le chemin recommandé est WSL2** (Windows Subsystem for Linux) : les scripts du
dépôt (`deploy.sh`, `minitel-status.sh`) sont du bash, et sous WSL vous suivez
simplement la colonne Linux de ce guide.

```powershell
# PowerShell administrateur — installe WSL2 avec Ubuntu
wsl --install
# puis, dans le terminal Ubuntu : suivez la section Linux ci-dessus
```

| Tâche | Windows natif | WSL2 |
|---|---|---|
| Compiler, `cargo test` | ✅ (rustup + Visual Studio Build Tools) | ✅ |
| Cross-compiler pour le Pi | ✅ (les cibles musl + rust-lld fonctionnent) | ✅ |
| `deploy.sh`, `minitel-status.sh` | ❌ (bash) | ✅ |
| Le TUI `minitel-tui` | ✅ (mais `/img` exige `python3` dans le PATH) | ✅ |
| Le daemon sur un port COM local | ⚠️ non testé | via usbipd (voir plus bas) |

En natif, l'installation de Rust passe par [rustup.rs](https://rustup.rs) et
réclame les *Visual Studio Build Tools* (l'installateur vous guide). C'est
utilisable pour le parcours D (le TUI seul) ; pour tout le reste, WSL est
plus simple.

**Brancher le Minitel sur un PC Windows** (parcours C) : l'USB n'est pas visible
de WSL par défaut. L'outil officiel
[usbipd-win](https://github.com/dorssel/usbipd-win) attache un périphérique USB
à WSL (`usbipd bind` puis `usbipd attach --wsl`), après quoi l'adaptateur
apparaît en `/dev/ttyUSB0` côté Linux et tout ce guide s'applique. C'est le
chemin le moins éprouvé du projet : si vous avez un Pi ou une machine Linux qui
traîne, préférez-les.

---

## Choisir son agent de code

Ce dépôt est écrit pour être travaillé avec un agent : le fichier
**[AGENTS.md](../AGENTS.md)** (un [standard ouvert](https://agents.md) lu par
tous les outils ci-dessous) contient les invariants matériels, les pièges et les
conventions — l'agent les découvre tout seul en arrivant dans le dépôt.

N'importe lequel de ces harness fait l'affaire. Le choix se fait sur vos accès
(abonnement, clé API) et vos goûts, pas sur une exigence du projet :

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

L'écosystème bouge vite ; un annuaire tenu à jour :
[awesome-cli-coding-agents](https://github.com/bradAGI/awesome-cli-coding-agents).

Tous s'utilisent pareil pour ce projet : **ouvrez un terminal à la racine du
dépôt cloné, lancez l'agent, collez un prompt ci-dessous.**

---

## Les prompts de départ

À adapter — remplacez les valeurs entre `<chevrons>`. Le reste, l'agent le
trouve dans AGENTS.md.

### Parcours A — essai sans matériel

```
Je découvre ce dépôt. Fais tourner la chaîne complète sans matériel :
compile, lance le backend d'exemple en mode écho, lance le daemon sur un
faux device, et montre-moi que /status répond. Explique-moi ce que je vois.
Mon OS : <Linux / macOS / Windows+WSL>.
```

### Parcours B — installation complète sur un Raspberry Pi

```
Je veux déployer Klek-Minitel sur mon Raspberry Pi. Ce que j'ai :
- le Pi : <ip>, utilisateur <pi>, OS <64 bits / 32 bits / je ne sais pas>
- ma clé SSH n'est pas encore sur le Pi (fais le ssh-copy-id, le mot de
  passe est à moi : demande-le-moi au bon moment)
- le Minitel est branché ; le port série est à identifier
  (ls /dev/serial/by-id/ sur le Pi)
- backend : <ip:port, ou « lance le backend d'exemple sur le Pi »>
- titre à l'écran : "<MON SERVICE>"
Utilise deploy.sh, puis vérifie avec tools/minitel-status.sh et
montre-moi le résultat.
```

(Le câblage physique, lui, reste votre travail :
[materiel-branchement.md](materiel-branchement.md).)

### Parcours C — sans Pi, Minitel branché sur cette machine

```
Mon Minitel est branché directement sur cette machine par USB-UART
(pas de Raspberry Pi). Identifie le port série, compile en natif, lance
le backend d'exemple et le daemon, et vérifie que le lien série se
négocie (connected:true dans /status). Mon OS : <Linux / macOS>.
```

### Parcours D — piloter un Minitel déjà installé

```
Un daemon Klek-Minitel tourne sur <ip>:3010. Compile le TUI
(feature `tui`) et lance-le vers cette adresse. Ensuite, convertis
<photo.png> et affiche-la sur le Minitel.
```

### Et pour créer votre module

```
Lis docs/creer-un-module.md. Je veux une entrée au menu Guide qui
affiche <ce que vous voulez : votre météo, l'état de vos serveurs…>.
Ajoute-la au services.json et au backend, teste la route en local,
et vérifie que chaque ligne tient en 40 colonnes.
```

---

## Et ensuite

- Le matériel : [materiel-branchement.md](materiel-branchement.md), puis
  [materiel-soudure.md](materiel-soudure.md)
- L'installation Pi pas à pas (sans agent) :
  [install-raspberry-pi.md](install-raspberry-pi.md)
- Vos modules : [creer-un-module.md](creer-un-module.md)
- Ce qui casse pour de vrai : [journal-de-bord.md](journal-de-bord.md)
