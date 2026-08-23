# Klek-Minitel

*Un vrai Minitel comme interface de vos services modernes.*

**Faites d'un vrai Minitel l'interface d'un service moderne.** Pilote Rust pour
terminaux Minitel 1B (série + Vidéotex), plus un daemon qui transforme le
terminal en client de conversation paginé : vous tapez une question sur le
clavier d'époque, un backend que vous écrivez répond sur l'écran cathodique.

```
     ┌──────────────────────────────────────┐
     │ MON SERVICE           p1/2           │
     │                                      │
     │ > combien de lunes a jupiter ?       │
     │ JUPITER : 95 LUNES CONFIRMEES        │
     │                                      │
     │ Les quatre principales (Io, Europe,  │
     │ Ganymede, Callisto) ont ete vues     │
     │ par Galilee en 1610.                 │
     │                                      │
     │ VOUS :                               │
     │ ......................................│
     │                                      │
     │ ENVOI  SUITE/RETOUR=page  SOMMAIRE=raz│
     └──────────────────────────────────────┘
```

Testé sur matériel : Minitel 1B relié à un Raspberry Pi par un adaptateur
USB-UART à quelques euros. Le binaire pèse ~1 Mo, est lié statiquement et tourne
jusque sur un Pi Zero W.

## Ce que vous obtenez

- **Un lien série qui ne lâche pas.** Le 1B n'a pas d'EEPROM et redémarre à 1200
  bauds : le pilote renégocie 4800 tout seul, à chaque connexion, et se
  reconnecte si vous éteignez le terminal.
- **Le protocole Vidéotex fait proprement.** Accents (G2), mosaïque
  semi-graphique (G1), couleurs, tailles doubles, rangée 0 de service,
  compression `REP`, césure à 40 colonnes — avec les pièges documentés.
- **Un clavier décodé.** Touches de fonction (Envoi, Sommaire, Suite, Retour,
  Guide, Correction, Annulation), flèches, accents composés, accusés PRO.
- **Un éditeur de saisie** multi-lignes projeté sur la grille de l'écran.
- **Des images.** Convertisseur image → mosaïque G1 (quantification adaptative
  ou niveaux de gris) et prévisualisation ASCII pour itérer sans matériel.
- **Un daemon prêt à l'emploi** : accueil, conversation paginée, menu de services
  sur la touche Guide, mise en veille du terminal, API de contrôle réseau.
- **Une télécommande TUI** pour écrire sur l'écran cathodique et y pousser des
  images depuis votre poste de travail, sans toucher au clavier d'époque.
- **De quoi opérer** : déploiement SSH en une commande, unité systemd, script
  d'état de santé, journal des pannes réelles.

## Deux moitiés

Ce dépôt est **la moitié pilote**. Il ne contient aucune logique métier et
n'accède pas à Internet : il appelle quatre routes HTTP sur un backend que vous
écrivez, dans le langage que vous voulez.

```
┌──────────┐  série 1200/4800 bd  ┌──────────────┐   HTTP/1.0   ┌──────────┐
│ Minitel  │◄────────────────────►│  miniteld    │◄────────────►│ backend  │
│ 1B       │  DIN 5 ↔ USB-UART    │  (ce dépôt)  │  4 routes    │ (à vous) │
└──────────┘                      └──────────────┘              └──────────┘
```

Le contrat tient en quatre routes JSON — `/health`, `/ask`, `/service`, `/reset`
— et une seule exigence de fond : **le texte renvoyé doit tenir en 40 colonnes**.
Un backend de référence complet (~180 lignes, bibliothèque standard, LLM
facultatif) est fourni dans `examples/backend/`.

## Brancher vos modules

Trois points d'extension, du plus simple au plus profond — aucun ne demande de
toucher au pilote :

1. **Une entrée au menu Guide** (`services.json`) : une touche, un nom, un
   libellé. Le daemon appelle `GET /service?name=…` sur votre backend, qui
   renvoie le texte à afficher. Une météo, un flux RSS, l'état de vos serveurs :
   c'est dix lignes dans votre backend, zéro ligne de Rust.
2. **Un backend entier** : réimplémentez les quatre routes dans le langage de
   votre choix et le Minitel devient l'interface de ce que vous voulez — un LLM,
   une domotique, un jeu. Partez de `examples/backend/backend.py`.
3. **Une autre application Minitel** : le crate `minitel` (lien série, protocole,
   encodage Vidéotex, clavier, éditeur) est indépendant du daemon. Importez-le et
   écrivez la vôtre — voir « Utiliser la bibliothèque seule » ci-dessous.

Et de l'extérieur, l'**API de contrôle** (`POST /text`, `POST /show`) permet à
n'importe quel script du LAN de pousser du contenu à l'écran : notifications,
tableaux de bord, dessins.

## Essayer sans matériel

```bash
git clone https://github.com/kerbart/Klek-Minitel && cd Klek-Minitel
cargo build --release

# le backend de référence, en mode écho (aucune clé API nécessaire)
python3 examples/backend/backend.py &

# le daemon, sur un faux device
MINITEL_BACKEND=127.0.0.1:3009 RUST_LOG=info ./target/release/miniteld /dev/null

# dans un autre terminal
curl -s localhost:3010/status
```

Un `état réseau from=Unknown to=Online` dans les logs : la chaîne logicielle est
bonne. Il ne reste qu'à câbler.

## Installer pour de vrai

Trois fils entre l'adaptateur USB-UART et la prise DIN du Minitel (**ne touchez
pas à la broche 5, elle sort 8,5 V**), puis :

```bash
HOST=pi@192.168.1.42 BACKEND=192.168.1.10:3009 TITLE="MON SERVICE" ./deploy.sh
```

`deploy.sh` compile un binaire statique pour l'ARM visé, l'installe sur la
machine distante et écrit l'unité systemd. Ni Docker ni toolchain C requis.

Le pas-à-pas complet (flash du Pi, câblage, premier déploiement, vérifications)
tient dans **[docs/install-raspberry-pi.md](docs/install-raspberry-pi.md)**. Le
brochage exact, le choix de la cible ARM, la configuration et le contrat du
backend sont détaillés dans **[AGENTS.md](AGENTS.md)**.

## Piloter depuis votre poste : le TUI

Une fois le daemon en place, `minitel-tui` transforme votre terminal en
télécommande de l'écran cathodique — tapez du texte, il s'affiche paginé sur le
Minitel ; donnez une image, elle est convertie en mosaïque et affichée :

```bash
cargo run --release --features tui --bin minitel-tui -- 192.168.1.42:3010

# dans le champ de saisie :
#   bonjour depuis 2026        →  s'affiche sur le Minitel
#   /img photo.png --gray      →  convertie (img2vtx.py) puis affichée
#   /vtx logo.vtx              →  flux Vidéotex brut
```

La barre d'état montre en continu le lien série (connecté, bauds, veille) et la
santé du backend. Le TUI est derrière la feature `tui` : le binaire déployé sur
le Pi ne l'embarque jamais.

## Surveiller

```bash
./tools/minitel-status.sh pi@192.168.1.42   # API + service systemd + journal + alim
curl -s 192.168.1.42:3010/status            # juste l'état, en JSON
```

Le script vérifie aussi `vcgencmd get_throttled` : sur ce montage, la
sous-alimentation du Pi est la première cause de glitches série — avant tout
soupçon logiciel.

## Utiliser la bibliothèque seule

Le daemon n'est qu'un exemple. Pour écrire une autre application Minitel :

```rust
use minitel::constants::Color;
use minitel::link::{Link, LinkConfig};
use minitel::{protocol, videotex};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let mut link = Link::spawn(LinkConfig::default());   // reconnexion incluse
link.send(protocol::clear_screen()).await?;
link.send(protocol::move_to(1, 5)).await?;
link.send(videotex::colored(Color::Cyan, "bonjour à toi")).await?;

while let Some(event) = link.recv().await {          // octets reçus, (re)connexions
    println!("{event:?}");
}
# Ok(())
# }
```

`Link::spawn` rend la main immédiatement : ouverture du port, négociation de
vitesse et reconnexions se font en tâche de fond. `recv()` livre les `LinkEvent`
— octets du clavier, connexion établie, lien perdu — qu'un `input::Decoder`
transforme en touches.

`link` (série) · `protocol` (séquences) · `videotex` (encodage) · `input`
(clavier) · `edit` (saisie) sont indépendants du daemon et de tout backend.

## Documentation

| Fichier | Contenu |
|---|---|
| **[AGENTS.md](AGENTS.md)** | déploiement, configuration, contrat du backend, invariants, dépannage |
| [docs/install-raspberry-pi.md](docs/install-raspberry-pi.md) | le pas-à-pas Raspberry Pi, du flash de la carte SD au premier écran |
| [docs/journal-de-bord.md](docs/journal-de-bord.md) | **les pannes réelles** : cause racine, fausse piste, résolution |
| [docs/videotex-1b-cheatsheet.md](docs/videotex-1b-cheatsheet.md) | toutes les séquences hex du Minitel 1B (norme STUM) |
| [docs/image-conversion.md](docs/image-conversion.md) | pipeline image → `.vtx`, modes de rendu, timings |

Ce dépôt est écrit pour être travaillé **avec un agent de code** (Claude Code,
Codex, ou autre) : donnez-lui [AGENTS.md](AGENTS.md) comme point d'entrée, il y
trouvera les invariants matériels que le compilateur ne peut pas vérifier — et
les tests qui les verrouillent. Tout se vérifie sans Minitel branché
(`cargo test`, `vtx-preview.py`, mode `/dev/null`).

Si vous montez le même bricolage, lisez le **journal de bord** avant de souder :
sur onze pannes majeures, six étaient physiques ou électriques — l'alimentation
et le câblage vous occuperont plus que le protocole.

## État du projet

Fonctionnel et utilisé au quotidien sur un Minitel 1B. 37 tests couvrent les
invariants d'affichage et de protocole, sans matériel requis (`cargo test`).

Le pilote cible le **Minitel 1B via la prise péri-informatique**. Les autres
modèles partagent l'essentiel du protocole mais n'ont pas été testés — les
retours (et les correctifs) sont bienvenus.

## Licence

MIT — voir [LICENSE](LICENSE).

Les constantes de protocole s'inscrivent dans la lignée des implémentations
Python du Minitel (PyMinitel et dérivés), revérifiées sur matériel.
