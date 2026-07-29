# minitia

*Minitel + IA.*

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

## Essayer sans matériel

```bash
git clone <URL_DU_DEPOT> && cd minitia
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

Le brochage exact, le choix de la cible ARM, la configuration et le contrat du
backend sont détaillés dans **[AGENTS.md](AGENTS.md)** — commencez par là.

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
| [docs/journal-de-bord.md](docs/journal-de-bord.md) | **les pannes réelles** : cause racine, fausse piste, résolution |
| [docs/videotex-1b-cheatsheet.md](docs/videotex-1b-cheatsheet.md) | toutes les séquences hex du Minitel 1B (norme STUM) |
| [docs/image-conversion.md](docs/image-conversion.md) | pipeline image → `.vtx`, modes de rendu, timings |

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
