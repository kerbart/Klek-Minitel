# AGENTS.md — déployer et modifier Klek-Minitel

Guide de travail sur ce dépôt, à destination d'un agent de code ou d'un humain
pressé. Il dit **ce qui se passe si vous vous trompez**, parce que la plupart des
pièges de ce projet ne produisent pas d'erreur : ils produisent un écran illisible.

Le lecteur pressé va directement à [Démarrage en 10 minutes](#démarrage-en-10-minutes)
puis au [Contrat du backend](#contrat-du-backend).

---

## Ce que fait ce dépôt

Un **vrai** terminal Minitel devient l'interface d'un service que vous écrivez.
Deux moitiés indépendantes :

| Moitié | Où | Rôle |
|---|---|---|
| **Le pilote** (ce dépôt) | Rust, sur une machine reliée au Minitel par un câble série | parle Vidéotex, décode le clavier, dessine les écrans |
| **Le backend** (à vous) | n'importe quel langage, n'importe où sur le réseau | reçoit une question, renvoie du texte à afficher |

Le pilote ne contient **aucune** logique métier et n'accède pas à Internet. Il
appelle quatre routes HTTP. Tout ce que le Minitel affichera d'intelligent vient
de votre backend.

### Trois noms, ne les confondez pas

| Nom | Ce que c'est |
|---|---|
| **Klek-Minitel** | le projet et le dépôt |
| `minitel` | le crate de bibliothèque — le pilote réutilisable |
| `miniteld` | le binaire du daemon, et le nom de l'unité systemd |

```
┌──────────┐  série 1200/4800 bd   ┌───────────────┐   HTTP/1.0    ┌──────────┐
│ Minitel  │◄─────────────────────►│  miniteld     │◄─────────────►│ backend  │
│ 1B       │   DIN 5 ↔ USB-UART    │  (Pi, ~1 Mo)  │  4 routes     │ (à vous) │
└──────────┘                       └───────────────┘               └──────────┘
```

---

## Carte du dépôt

```
src/
  link.rs        lien série : ouverture, négociation de vitesse, reconnexion
  constants.rs   codes du protocole (C0, G0/G1/G2, touches) — vérifiés matériel
  protocol.rs    séquences de commande (curseur, effacement, écho, vitesse)
  videotex.rs    encodage texte : accents → G2, couleurs, tailles, mosaïque, wrap
  input.rs       décodeur clavier : touches de fonction, flèches, accusés PRO
  edit.rs        éditeur de saisie multi-lignes projeté sur la grille 40×24
  backend.rs     client HTTP/1.0 minimal vers votre backend
  bin/
    miniteld.rs  LE daemon : accueil, conversation paginée, menu Guide, API ctrl
    demo.rs      10 écrans de démonstration des capacités Vidéotex
    show.rs      affiche un fichier .vtx, et rien d'autre
    tui.rs       télécommande TUI du poste de travail (feature `tui`, jamais sur le Pi)
docs/
  materiel-branchement.md     (humain) acheter, câbler, identifier les broches
  materiel-soudure.md         (humain) souder le câble DIN, vérifier avant tension
  creer-un-module.md          (humain) les 3 niveaux d'extension, contrat détaillé
  install-raspberry-pi.md     le pas-à-pas Pi, du flash au premier écran
  journal-de-bord.md          les pannes réelles du projet et leur cause racine
  videotex-1b-cheatsheet.md   toutes les séquences hex (la référence à ouvrir)
  image-conversion.md         pipeline image → .vtx
tools/
  img2vtx.py        image → mosaïque G1 (adaptatif ou niveaux de gris)
  vtx-preview.py    rend un .vtx en ASCII dans le terminal (itérer sans matériel)
  minitel-status.sh état de santé complet : API, systemd, journal, alimentation
examples/backend/backend.py   backend de référence, 4 routes, ~180 lignes
systemd/miniteld.service      gabarit d'unité
deploy.sh                     build statique + install du service à distance
services.example.json         gabarit du menu de la touche Guide
```

`src/link.rs` → `src/edit.rs` forment une bibliothèque réutilisable sans rapport
avec le daemon : si vous voulez écrire une toute autre application Minitel,
importez le crate `minitel` et ignorez `bin/miniteld.rs`.

---

## Matériel

### Ce qu'il faut

- un **Minitel 1B** (ou compatible péri-informatique) ;
- un **adaptateur USB-UART TTL** — un module CH340 à quelques euros suffit ;
- trois fils vers la prise **DIN 5 broches** à l'arrière du Minitel ;
- une machine Linux qui reste allumée : un Raspberry Pi (même un Zero W) fait
  largement l'affaire — le binaire pèse ~1 Mo et le CPU ne fait rien.

### Câblage

Brochage de la prise péri-informatique, **d'après la notice constructeur du
Minitel 1B** (vue de l'extérieur du terminal) :

```
     3
   •   •
 2 • 5 • 4
     •
     1
```

| Broche | Signal Minitel | À relier |
|---|---|---|
| 1 | **RX** — entrée de données | **TX** de l'adaptateur |
| 2 | Référence 0 V | **GND** de l'adaptateur |
| 3 | **TX** — sortie de données | **RX** de l'adaptateur |
| 4 | PT — périphérique prêt | *(non utilisé)* |
| 5 | Sortie 8,5 V / 1 A | ⚠️ **ne rien y brancher** |

⚠️ **La broche 5 délivre 8,5 V.** La relier à un adaptateur USB-UART en 3,3 V ou
5 V, c'est le détruire, et potentiellement le port USB avec. Ne câblez que 1, 2, 3.

> Si l'utilisateur en est au stade du câblage ou de la soudure, renvoyez-le vers
> **[docs/materiel-branchement.md](docs/materiel-branchement.md)** et
> **[docs/materiel-soudure.md](docs/materiel-soudure.md)** : ils contiennent la
> procédure d'identification des broches au multimètre (le diagramme seul est
> ambigu — sur une fiche mâle vue côté soudure, l'arrangement est en miroir).

Les niveaux sont **TTL collecteur ouvert**. Un module CH340 se raccorde
directement ; si le Minitel émet mais que rien n'arrive (broche 3 muette), un
pull-up de ~10 kΩ vers le VCC de l'adaptateur sur cette ligne règle le cas.

### Paramètres série

Fixés par le terminal, non négociables : **7 bits, parité paire, 1 bit de stop**.
Vitesses possibles sur la DIN : **300, 1200 ou 4800 bauds** — et rien au-dessus.

Le 1B **n'a pas d'EEPROM** : il redémarre toujours à 1200 bauds. `link.rs` le
sait et fait, à chaque connexion : tentative directe en 4800 → sinon ouverture à
1200, envoi de la commande PRO2 de changement de vitesse, réouverture en 4800 →
sinon repli définitif sur 1200, qui marche toujours. Vous n'avez rien à régler.

### Trouver le port série

```bash
ls -l /dev/serial/by-id/
# usb-1a86_USB_Serial-if00-port0 -> ../../ttyUSB0   (typique d'un CH340)
```

**Utilisez toujours le chemin `/dev/serial/by-id/…`**, jamais `/dev/ttyUSB0` :
le numéro change au gré des rebranchements et des ports USB, le chemin `by-id`
non. C'est la première cause de service qui ne redémarre pas après un reboot.

L'utilisateur doit être dans le groupe qui possède le port (`dialout` sur Debian
et Raspberry Pi OS) — sinon « Permission denied » :

```bash
sudo usermod -aG dialout "$USER"   # puis se déconnecter/reconnecter
```

---

## Démarrage en 10 minutes

Rien de tout ceci n'exige le Minitel : commencez par faire tourner la chaîne
logicielle, branchez le matériel ensuite.

```bash
# 1. le backend de référence, en mode écho (aucune clé API requise)
python3 examples/backend/backend.py &        # écoute sur :3009

# 2. le daemon, sans matériel : /dev/null accepte l'écriture et ne répond jamais
cargo build --release
MINITEL_BACKEND=127.0.0.1:3009 RUST_LOG=info ./target/release/miniteld /dev/null
```

Dans les logs, `état réseau from=Unknown to=Online` signifie que le daemon voit
le backend : la moitié réseau est bonne. Vérifiez aussi l'API de contrôle :

```bash
curl -s localhost:3010/status
# {"connected":false,"baud":1200,"idle_secs":3,"sleeping":false,"net":"online"}
```

`connected:false` est normal sans Minitel branché. Rebranchez sur le vrai port
série et relancez : `connected` passe à `true` et l'écran d'accueil s'affiche.

Pour un vrai LLM derrière, installez le SDK et donnez une clé :

```bash
pip install anthropic
export ANTHROPIC_API_KEY=sk-ant-...
python3 examples/backend/backend.py
```

---

## Configuration

Tout passe par l'environnement — **aucun secret ne vit dans ce dépôt**, et le
daemon n'en manipule aucun (les clés d'API restent côté backend).

| Variable | Défaut | Rôle |
|---|---|---|
| `MINITEL_BACKEND` | `127.0.0.1:3009` | `ip:port` du backend |
| `MINITEL_TITLE` | `MINITEL` | titre de la rangée d'en-tête (tronqué à 24) |
| `MINITEL_LOGO` | *(aucun)* | `.vtx` affiché à l'accueil |
| `MINITEL_SERVICES` | `services.json` | menu de la touche **Guide** |
| `MINITEL_CTRL_PORT` | `3010` | port de l'API de contrôle |
| `RUST_LOG` | `info` | verbosité — `debug` trace chaque octet décodé |
| `TZ` | *(système)* | fuseau de l'horloge de la rangée 0 |

Le device série se passe en **argument** : `miniteld /dev/serial/by-id/…`.

### `MINITEL_BACKEND` : mettez une IP, pas un nom

Le binaire est lié statiquement à musl, qui n'embarque pas de résolveur DNS
complet. `mon-serveur.local` peut échouer là où `192.168.1.10` marche. En cas de
`SRV KO` inexplicable, c'est la première chose à tester.

### Menu de la touche Guide

Copiez `services.example.json` vers `services.json` (ignoré par git) :

```json
[
  { "key": "1", "name": "meteo", "label": "Meteo du jour" }
]
```

`key` = la touche à taper (1 caractère), `name` = ce qui part vers le backend
(`GET /service?name=meteo`), `label` = ce que lit l'utilisateur. **Fichier absent
= menu vide**, et la page Guide le dit franchement plutôt que de rester blanche.
Le menu affiche une entrée toutes les 2 lignes à partir de la ligne 5 : au-delà
de ~9 entrées, les suivantes sont silencieusement omises.

### Bannière d'accueil

Sans `MINITEL_LOGO`, l'accueil affiche `MINITEL_TITLE` en double taille — c'est
volontaire, le dépôt n'embarque aucune image. Pour votre propre logo :

```bash
python3 tools/img2vtx.py logo.png -o logo.vtx --row 2
python3 tools/vtx-preview.py logo.vtx        # vérifier sans mobiliser le Minitel
```

Le `.vtx` doit tenir dans les **lignes 2 à 9** et se positionner lui-même. Voir
`docs/image-conversion.md`. Un fichier illisible n'empêche pas le démarrage : le
daemon trace un `warn` et retombe sur la bannière texte.

---

## Contrat du backend

**La partie à implémenter.** Quatre routes, du JSON, aucune authentification,
HTTP/1.0 sans keep-alive. `examples/backend/backend.py` en est une
implémentation complète en bibliothèque standard : partez de là.

| Route | Appelée quand | Réponse attendue |
|---|---|---|
| `GET /health` | toutes les 20 s, en fond | `{"ok":true,"net":true}` |
| `GET /ask?q=…&cont=1` | l'utilisateur valide par **Envoi** | `{"text":"…"}` |
| `GET /service?name=…` | l'utilisateur choisit une entrée du **Guide** | `{"text":"…"}` |
| `GET /reset` | l'utilisateur appuie sur **Sommaire** | ignorée (best-effort) |

Détails qui comptent :

- **`net` dans `/health` est l'accès Internet de votre backend**, pas sa santé.
  Le voyant de la rangée 0 distingue `WEB OK` (tout va bien), `WEB KO` (backend
  joignable mais coupé d'Internet) et `SRV KO` (backend injoignable). Cette
  distinction est ce qui évite de chercher la panne du mauvais côté. Champ absent
  → interprété comme `false`.
- **`cont=1` = relance dans le fil courant.** Absent = nouvelle conversation, il
  faut vider votre historique. Le daemon ne renvoie jamais l'historique : c'est
  au backend de le tenir.
- **`{"error":"…"}` s'affiche à l'écran.** Renvoyez un message court et utile,
  pas une stack trace. Un HTTP 500 sans corps JSON donne un message générique.
- **Timeouts côté daemon** : 6 s pour `/health`, 40 s pour `/service`, **130 s**
  pour `/ask`. Un LLM lent a donc le droit de réfléchir, mais pas éternellement.

### Formater pour 40 colonnes — c'est le travail du backend

Le daemon découpe les lignes trop longues, mais il ne peut pas rattraper du
Markdown ni un tableau. **Écrivez directement pour l'écran** :

- **40 caractères par ligne**, maximum absolu ;
- **pas de Markdown** — `**gras**` s'affiche avec les astérisques ;
- pas d'art ASCII à base de `╔═╗` : à cette résolution les caractères de biseau
  se soudent en un pâté illisible ;
- les MAJUSCULES accentuées passent (É, À…) mais restent moins lisibles ;
- une ligne vide sépare les blocs — c'est le seul « style » disponible.

Le prompt système de `examples/backend/backend.py` encode déjà ces règles ;
réutilisez-le si votre backend appelle un LLM.

---

## Déploiement

### Automatique

```bash
HOST=pi@192.168.1.42 BACKEND=192.168.1.10:3009 TITLE="MON SERVICE" ./deploy.sh
```

Le script compile un binaire **statique musl** (aucune dépendance à installer sur
la cible), le copie dans `~/minitel-driver/`, écrit l'unité systemd, l'active et
la redémarre. Cibles utiles via `TARGET=` :

| Machine | `TARGET` |
|---|---|
| Pi 3/4/5, OS 64 bits | `aarch64-unknown-linux-musl` *(défaut)* |
| Pi 2/3, OS 32 bits | `armv7-unknown-linux-musleabihf` |
| Pi 1 / Zero W (ARMv6) | `arm-unknown-linux-musleabihf` |

La cross-compilation ne demande **ni Docker ni toolchain C** : tout est du Rust
pur, le lien passe par `rust-lld` (voir `.cargo/config.toml`). Un simple
`rustup target add <cible>` suffit, et `deploy.sh` le fait pour vous.

### Manuel

```bash
cargo build --release --target aarch64-unknown-linux-musl --bin miniteld
scp target/aarch64-unknown-linux-musl/release/miniteld pi@hote:~/minitel-driver/
# puis adapter systemd/miniteld.service (les <CHEVRONS>) et l'installer
```

### Piège Raspberry Pi : la racine en lecture seule

Si vous avez activé l'overlayfs de `raspi-config` (pour épargner la carte SD),
**tout déploiement disparaît au reboot** — sans le moindre message d'erreur.
`deploy.sh` détecte le cas et refuse de continuer. Pour lever :

```bash
sudo raspi-config nonint disable_overlayfs && sudo reboot
```

---

## API de contrôle

Le daemon écoute sur `MINITEL_CTRL_PORT` (3010) pour piloter l'écran depuis le
réseau — pratique pour une notification ou une image, sans toucher au clavier.

```bash
curl -s          hote:3010/status                     # état du lien série
curl -d "COUCOU" hote:3010/text                       # afficher du texte
curl --data-binary @image.vtx hote:3010/show          # afficher un .vtx
```

`/text` prend du texte brut (pas du JSON) et le pagine comme une réponse
normale ; `/show` prend des octets Vidéotex bruts et les envoie tels quels.

La même API a une interface confortable : le TUI du poste de travail, qui
enchaîne conversion d'image et envoi, et affiche l'état du lien en continu.

```bash
cargo run --release --features tui --bin minitel-tui -- hote:3010
```

🔓 **Aucune authentification, écoute sur `0.0.0.0`.** N'importe qui sur le réseau
peut écrire sur l'écran. C'est acceptable sur un LAN de confiance, pas au-delà :
si la machine est exposée, filtrez ce port au pare-feu. Ne l'ouvrez jamais sur
Internet — le daemon n'a aucun contrôle d'accès et n'en aura pas.

---

## Les invariants à ne pas casser

Ces règles ont toutes été apprises sur du matériel réel. Les enfreindre ne
provoque pas d'erreur de compilation : ça provoque un écran faux, et vous
chercherez longtemps.

### Une séquence d'attribut consomme une case écran

En Vidéotex, un changement de couleur occupe **une position à l'écran**. Donc
`colored(Color::Green, "…")` sur 40 caractères déborde sur la ligne suivante.
Le pied de page du daemon fait exactement 38 caractères pour cette raison, et un
test le verrouille. Toute chaîne colorée pleine largeur doit tenir en **39**.

### Mosaïque G1 : positionner **puis** basculer

Le positionnement `1F L C` (comme `CR`/`LF`) **réinitialise le jeu en G0**. Par
ligne de mosaïque il faut donc, dans cet ordre : `1F L C` **puis** `0E`, puis les
octets G1. L'ordre inverse affiche du charabia alphanumérique. C'est le piège le
plus coûteux du projet.

### Double hauteur interdite en ligne 1 et rangée 0

Et interdite en mosaïque. Le terminal n'affiche rien d'utile, sans se plaindre.

### La zone de saisie doit correspondre à la place réelle

`Editor::set_rows()` borne la capacité du buffer. Si l'éditeur croit avoir plus
de lignes que l'écran n'en offre, une saisie longue écrit **sous** la ligne 24 —
et le Minitel fait alors défiler tout l'affichage. `draw_answer()` appelle
`set_rows(CHAT_INPUT_ROWS)` et `draw_home()` `set_rows(AREA_ROWS)` : si vous
changez une constante de disposition, changez l'appel correspondant.

Disposition du mode conversation, telle que verrouillée par les tests :

```
  1       en-tête (titre + p n/m)
  3..20   fil de discussion (18 lignes)
  21      « VOUS : » — sert aussi de ligne d'attente (spinner + chrono)
  22      saisie (1 ligne)
  23      vide — respiration avant le pied de page
  24      pied de page (navigation)
```

### `Net::label()` fait exactement 6 caractères

La rangée 0 est cadrée sur 37 colonnes avec l'heure calée à droite. Un libellé
d'une autre largeur fait sauter l'heure. Un test le vérifie pour les 4 états.

### Les majuscules accentuées ne s'affichent pas

Le pilote **sait les encoder**, et le terminal ne s'en plaint pas — mais elles ne
se voient pas. Un accent est un glyphe G2 superposé à la lettre par un OU binaire
sur les rangées 1-2 de la cellule : sur une capitale, ces pixels sont **déjà
allumés**, donc l'accent disparaît. `É` sort `E`. La cédille occupe les rangées
8-9, restées libres : `Ç` fonctionne, lui.

Conséquence pratique : n'écrivez pas de titre en majuscules accentuées en pensant
qu'il sera correct. Écrivez `ELECTRICITE` en connaissance de cause, ou passez le
mot en minuscules accentuées.

Côté clavier, le Minitel envoie **diacritique puis lettre** ; `input.rs`
recompose. Certaines combinaisons ne sont pas saisissables sur le terminal, quoi
qu'on tape : ne construisez pas d'interface qui les exige.

### Un caractère double largeur consomme deux colonnes

Le flux ne doit **pas** contenir la case masquée : ajoutez un remplissage et tout
ce qui suit se décale d'une colonne. La double hauteur, elle, écrit aussi sur la
ligne **au-dessus** et écrase son contenu — prévoyez la ligne libre.

---

## Tests

```bash
cargo test              # 37 tests, aucun matériel requis
```

Ils ne testent pas « est-ce que ça compile » mais les invariants ci-dessus :
cadrage de la rangée 0, pied de page qui tient dans 40 colonnes avec son
attribut, saisie qui ne peut pas déborder sous l'écran, fil paginable après
troncature, aller-retour encodage → décodage des accents, non-régression du bug
`CANCEL` hérité des implémentations Python.

**Si vous touchez à la disposition de l'écran, un test doit bouger.** Si aucun ne
bouge, c'est que la disposition n'est pas couverte : ajoutez-en un.

Pour itérer sur du rendu sans mobiliser le Minitel :

```bash
cargo run --release --bin minitel-demo -- /dev/serial/by-id/…   # 10 écrans types
python3 tools/vtx-preview.py fichier.vtx                        # aperçu ASCII
```

---

## Dépannage

| Symptôme | Cause la plus probable |
|---|---|
| `Permission denied` sur le device | utilisateur absent du groupe `dialout` |
| Le service ne repart pas après reboot | `/dev/ttyUSB0` codé en dur au lieu de `/dev/serial/by-id/…` |
| Rien à l'écran, `connected:false` | TX/RX inversés (broche 1 ↔ 3), ou GND absent |
| Écran de charabia alphanumérique | mosaïque G1 envoyée avant `0E` après un positionnement |
| Texte correct mais lignes décalées | une chaîne colorée dépasse 39 caractères |
| L'affichage défile tout seul | écriture sous la ligne 24 — `set_rows()` désaccordé |
| `SRV KO` en permanence | backend éteint, mauvais port, ou nom d'hôte non résolu (mettre une IP) |
| `WEB KO` en permanence | le backend répond mais n'a pas Internet — vérifier de son côté |
| Tout est lent, caractère par caractère | lien retombé à 1200 bauds : normal après une reconnexion |
| Le déploiement disparaît au reboot | overlayfs actif sur le Pi |

Chacune de ces lignes est une panne réellement vécue : les causes racines, les
fausses pistes qui ont précédé et ce qu'il a fallu changer physiquement sont
détaillés dans **[docs/journal-de-bord.md](docs/journal-de-bord.md)**.

Le premier réflexe utile est toujours le même :

```bash
./tools/minitel-status.sh pi@hote           # tout en un : API, systemd, journal, alim
```

ou à la main :

```bash
journalctl -u miniteld -n 50 --no-pager     # RUST_LOG=debug pour tracer les octets
curl -s localhost:3010/status
```

Sur Raspberry Pi, regardez aussi `vcgencmd get_throttled` (le script le fait) :
tout code autre que `0x0` signale une sous-alimentation — la cause racine la plus
fréquente des glitches série de ce montage, et la plus longue à trouver quand on
la cherche dans le logiciel.

---

## Conventions de code

- **Commentaires en français**, comme le reste du dépôt.
- Un commentaire explique **pourquoi**, pas quoi. Les commentaires de valeur ici
  documentent un piège matériel ou une contrainte d'écran — gardez ce niveau.
- Aucune dépendance HTTP (`reqwest`, `hyper`) : le client et le serveur sont
  écrits à la main pour garder le binaire statique petit. N'en ajoutez pas pour
  quatre routes en HTTP/1.0.
- Les dépendances de confort du poste de travail (ratatui…) vivent **derrière la
  feature `tui`** : rien de tout cela ne doit entrer dans le graphe du binaire
  déployé sur le Pi. `deploy.sh` compile sans features, et ça doit rester vrai.
- Le pilote ne doit **jamais** paniquer sur une entrée matérielle : un octet
  inattendu s'ignore, un lien coupé se reconnecte. `panic = "abort"` est actif,
  une panique tue le service.
- Aucun secret, aucune IP privée, aucun nom d'hôte personnel dans le code : tout
  ce qui est propre à une installation passe par l'environnement.
