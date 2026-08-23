# Créer un module

Guide pour humain. Objectif : faire afficher **vos** données sur le Minitel —
votre météo, votre domotique, vos serveurs, votre boîte mail, ce que vous voulez.

Il y a trois façons de s'y prendre, de dix minutes à un week-end. Commencez par
la première : elle suffit dans la grande majorité des cas.

| | Approche | Vous écrivez | Temps | Pour quoi |
|---|---|---|---|---|
| **1** | [Une entrée au menu Guide](#niveau-1--une-entrée-au-menu-guide) | ~10 lignes dans votre backend | 10 min | Afficher une information sur appui d'une touche |
| **2** | [Votre propre backend](#niveau-2--votre-propre-backend) | Un serveur HTTP, 4 routes | 1–2 h | Remplacer entièrement la logique, dans votre langage |
| **3** | [Une autre application Minitel](#niveau-3--une-autre-application-minitel) | Du Rust, sur le crate `minitel` | Un week-end | Une interface qui n'est pas une conversation (jeu, tableau de bord, menu maison) |

Et transversalement : [pousser du contenu depuis
l'extérieur](#pousser-du-contenu-depuis-lextérieur) (notifications, images) sans
rien écrire du tout.

**La règle qui gouverne tout :** votre code renvoie du **texte déjà prêt pour
l'écran**. Le Minitel fait **40 colonnes**. Ce n'est pas une suggestion — voir
[Écrire pour 40 colonnes](#écrire-pour-40-colonnes).

---

## Niveau 1 — une entrée au menu Guide

Le scénario : l'utilisateur appuie sur **Guide**, voit un menu numéroté, tape un
chiffre, l'information s'affiche. C'est le moyen le plus rapide d'avoir « votre »
module sur le Minitel.

### Étape 1 — déclarer l'entrée côté daemon

Copiez `services.example.json` en `services.json` (il est dans `.gitignore`,
c'est votre fichier), à côté du binaire sur le Pi :

```json
[
  { "key": "1", "name": "meteo",   "label": "Meteo du jour" },
  { "key": "2", "name": "serveurs", "label": "Etat de mes serveurs" },
  { "key": "3", "name": "poubelle", "label": "Quelle poubelle ce soir" }
]
```

| Champ | Rôle | Contrainte |
|---|---|---|
| `key` | la touche que l'utilisateur tape | **un seul caractère** ; les entrées invalides sont ignorées en silence |
| `name` | l'identifiant envoyé à votre backend (`GET /service?name=meteo`) | libre, c'est votre vocabulaire |
| `label` | ce que lit l'utilisateur à l'écran | court : ça s'affiche sur 40 colonnes, préfixe compris |

Deux limites à connaître, elles ne produisent aucune erreur :

- Le menu affiche **une entrée toutes les deux lignes à partir de la ligne 5** :
  au-delà d'environ **9 entrées**, les suivantes sont **silencieusement omises**.
- **Fichier absent = menu vide**, et la page Guide le dit franchement plutôt que
  de rester blanche.

Le chemin est configurable par `MINITEL_SERVICES` (défaut : `services.json` dans
le répertoire de travail du service).

### Étape 2 — servir l'entrée côté backend

Le daemon appelle `GET /service?name=<votre nom>` et attend `{"text": "..."}`.
Dans `examples/backend/backend.py`, il y a exactement un endroit à modifier :

```python
def service(name: str) -> str:
    """Sert une entrée du menu Guide (`services.json` côté daemon)."""

    if name == "meteo":
        # Votre vraie logique : requête HTTP, base de données, calcul…
        return (
            "METEO NOISY-LE-SEC\n"
            "\n"
            "Aujourd'hui : 18 C, couvert\n"
            "Demain      : 21 C, eclaircies\n"
            "\n"
            "Vent 15 km/h nord-est"
        )

    if name == "serveurs":
        return etat_de_mes_serveurs()      # votre fonction

    if name == "poubelle":
        return "CE SOIR : BAC JAUNE\n\nSortir avant 19h."

    return f"Service inconnu : {name}"
```

C'est tout. Redémarrez le backend, appuyez sur **Guide** sur le Minitel, tapez la
touche.

### Étape 3 — vérifier sans mobiliser le Minitel

```bash
# la route, telle que le daemon l'appellera
curl -s 'localhost:3009/service?name=meteo'
# {"text":"METEO NOISY-LE-SEC\n\nAujourd'hui : 18 C, couvert\n…"}

# et le contrôle qui compte vraiment : est-ce que ça tient en 40 colonnes ?
curl -s 'localhost:3009/service?name=meteo' \
  | python3 -c 'import json,sys; [print(f"{len(l):3d} | {l}") for l in json.load(sys.stdin)["text"].split("\n")]'
```

Toute ligne au-delà de **40** sera coupée par le daemon, souvent au mauvais
endroit. Cette petite commande vous évite l'aller-retour devant l'écran.

> **Le délai :** le daemon accorde **40 secondes** à `/service`. Si votre module
> interroge une API lente, prévoyez un cache — l'utilisateur, lui, regarde un
> écran figé pendant ce temps.

---

## Niveau 2 — votre propre backend

Le daemon ne contient **aucune** logique métier et n'accède pas à Internet. Tout
ce que le Minitel affichera d'intelligent vient de votre backend. Le contrat est
volontairement minuscule : **quatre routes, du JSON, pas d'authentification,
HTTP/1.0 sans keep-alive**.

Vous pouvez donc l'écrire dans n'importe quel langage. Voici le contrat exact,
tel qu'il est implémenté dans `src/backend.rs` :

### Les quatre routes

#### `GET /health` — la sonde, toutes les 20 s

```json
{"ok": true, "net": true}
```

**Délai : 6 s.** Le champ `net` n'est **pas** la santé de votre backend : c'est
**son accès à Internet**. Cette distinction pilote le voyant de la rangée 0 :

| Voyant | Signification |
|---|---|
| `WEB OK` | backend joignable **et** connecté à Internet |
| `WEB KO` | backend joignable, **mais coupé d'Internet** |
| `SRV KO` | backend **injoignable** |

C'est ce qui évite de chercher la panne du mauvais côté. Champ `net` absent →
interprété comme `false`.

#### `GET /ask?q=<question>&cont=1` — l'utilisateur a tapé Envoi

```json
{"text": "La reponse, deja formatee pour 40 colonnes."}
```

**Délai : 130 s** — un LLM lent a le droit de réfléchir, mais pas éternellement.

`cont=1` signifie **relance dans le fil courant**. Le paramètre **absent** signifie
**nouvelle conversation** : videz votre historique. Le daemon ne renvoie jamais
l'historique, c'est à votre backend de le tenir — voir la variable `HISTORY` du
backend d'exemple.

#### `GET /service?name=<nom>` — une entrée du Guide

```json
{"text": "Le contenu de votre module."}
```

**Délai : 40 s.** Voir [Niveau 1](#niveau-1--une-entrée-au-menu-guide).

#### `GET /reset` — l'utilisateur a appuyé sur Sommaire

Réponse ignorée, appel *best-effort*, **délai 5 s**. Videz votre historique.

### Signaler une erreur

Renvoyez `error` au lieu de `text` :

```json
{"error": "API meteo injoignable"}
```

Le message **s'affiche à l'écran**. Écrivez-le pour un humain devant un écran de
40 colonnes : court, utile, actionnable. Pas de trace de pile. Un HTTP 500 sans
corps JSON donne un message générique.

### Le squelette minimal

En Python, sans dépendance :

```python
#!/usr/bin/env python3
"""Backend Klek-Minitel minimal : les 4 routes, rien de plus."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

HISTORY = []


class Handler(BaseHTTPRequestHandler):
    def _json(self, payload, code=200):
        body = json.dumps(payload).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        url = urlparse(self.path)
        qs = parse_qs(url.query)

        if url.path == "/health":
            # `net` = VOTRE acces Internet, pas votre sante
            return self._json({"ok": True, "net": True})

        if url.path == "/reset":
            HISTORY.clear()
            return self._json({"ok": True})

        if url.path == "/ask":
            q = (qs.get("q") or [""])[0]
            if not qs.get("cont"):
                HISTORY.clear()          # nouvelle conversation
            HISTORY.append(q)
            return self._json({"text": f"Vous avez tape :\n{q[:120]}"})

        if url.path == "/service":
            name = (qs.get("name") or [""])[0]
            return self._json({"text": f"Service {name}\n\nA vous de jouer."})

        self._json({"error": "route inconnue"}, 404)

    def log_message(self, fmt, *args):
        pass                              # silence


ThreadingHTTPServer(("0.0.0.0", 3009), Handler).serve_forever()
```

Lancez-le, pointez le daemon dessus avec `MINITEL_BACKEND=<ip>:3009`, c'est
fonctionnel.

### ⚠️ Mettez une IP, pas un nom d'hôte

Le binaire est lié statiquement à **musl**, qui n'embarque pas de résolveur DNS
complet. `mon-serveur.local` peut échouer là où `192.168.1.10` fonctionne. Devant
un `SRV KO` inexplicable, c'est la **première** chose à tester.

### Tester votre backend sans Minitel

Les quatre routes, à la main :

```bash
curl -s localhost:3009/health
curl -s 'localhost:3009/ask?q=bonjour'
curl -s 'localhost:3009/ask?q=et%20ensuite&cont=1'
curl -s 'localhost:3009/service?name=meteo'
curl -s localhost:3009/reset
```

Puis la chaîne complète, toujours sans matériel — `/dev/null` accepte l'écriture
et ne répond jamais :

```bash
MINITEL_BACKEND=127.0.0.1:3009 RUST_LOG=info ./target/release/miniteld /dev/null
```

Dans les logs, `état réseau from=Unknown to=Online` signifie que le daemon voit
votre backend : la moitié réseau est bonne.

---

## Niveau 3 — une autre application Minitel

Le daemon `miniteld` n'est qu'**un** exemple d'usage. Si votre projet n'est pas
une conversation paginée — un jeu, un tableau de bord, un menu maison, un
afficheur de gare — n'essayez pas de le plier au daemon : utilisez directement le
crate.

Ces modules sont indépendants du daemon **et de tout backend** :

| Module | Ce qu'il vous donne |
|---|---|
| `link` | le lien série : ouverture, négociation de vitesse, **reconnexion automatique** |
| `protocol` | les séquences de commande : curseur, effacement, écho, vitesse |
| `videotex` | l'encodage du texte : accents → G2, couleurs, tailles, mosaïque, césure 40 colonnes |
| `input` | le décodeur clavier : touches de fonction, flèches, accents composés |
| `edit` | un éditeur de saisie multi-lignes projeté sur la grille 40×24 |
| `constants` | les codes du protocole (C0, G0/G1/G2, touches), **vérifiés sur matériel** |

```rust
use minitel::constants::Color;
use minitel::link::{Link, LinkConfig};
use minitel::{protocol, videotex};

# async fn demo() -> Result<(), Box<dyn std::error::Error>> {
let mut link = Link::spawn(LinkConfig::default());   // reconnexion incluse

link.send(protocol::clear_screen()).await?;
link.send(protocol::move_to(1, 5)).await?;
link.send(videotex::colored(Color::Cyan, "bonjour à toi")).await?;

while let Some(event) = link.recv().await {          // octets, (re)connexions
    println!("{event:?}");
}
# Ok(())
# }
```

`Link::spawn` **rend la main immédiatement** : ouverture du port, négociation de
vitesse et reconnexions se font en tâche de fond. `recv()` livre des `LinkEvent`
— octets du clavier, connexion établie, lien perdu — qu'un `input::Decoder`
transforme en touches.

Pour vous inspirer, par ordre de complexité : `src/bin/show.rs` (affiche un
`.vtx` et s'arrête), `src/bin/demo.rs` (10 écrans de démonstration),
`src/bin/miniteld.rs` (le daemon complet).

### Avant d'écrire du Vidéotex, lisez ceci

Ces règles ont été apprises sur du matériel réel. **Les enfreindre ne provoque
aucune erreur de compilation** — ça provoque un écran faux, et vous chercherez
longtemps. La liste complète est dans [AGENTS.md](../AGENTS.md#les-invariants-à-ne-pas-casser) ;
voici les quatre qui mordent le plus vite :

1. **Une séquence d'attribut consomme une case écran.** Un changement de couleur
   occupe une position. Donc une chaîne colorée de 40 caractères **déborde** sur
   la ligne suivante : le maximum est **39**.
2. **Mosaïque G1 : positionner PUIS basculer.** Le positionnement `1F L C`
   réinitialise le jeu en G0. Il faut donc, dans cet ordre : `1F L C` **puis**
   `0E`, puis les octets G1. L'ordre inverse affiche du charabia alphanumérique.
   *C'est le piège le plus coûteux du projet.*
3. **Double hauteur interdite en ligne 1 et en rangée 0** — et interdite en
   mosaïque. Le terminal n'affiche rien d'utile, sans se plaindre.
4. **Les majuscules accentuées ne s'affichent pas.** Un accent est un glyphe G2
   superposé par un OU binaire sur les rangées hautes de la cellule ; sur une
   capitale ces pixels sont **déjà allumés**, donc l'accent disparaît. `É` sort
   `E`. La cédille occupe les rangées basses : `Ç` fonctionne, lui.

### La boucle de vérification qui ne coûte rien

```bash
cargo test                                      # 37 tests, aucun matériel requis
cargo run --release --bin minitel-demo -- /dev/serial/by-id/…
python3 tools/vtx-preview.py fichier.vtx        # aperçu ASCII d'un .vtx
```

**Si vous touchez à la disposition de l'écran, un test doit bouger.** Si aucun ne
bouge, c'est que la disposition n'est pas couverte : ajoutez-en un. C'est le
principe qui structure ce dépôt — ce qui a été payé une fois devant le verre ne
doit pas se repayer.

---

## Pousser du contenu depuis l'extérieur

Sans écrire de module du tout : le daemon écoute sur `MINITEL_CTRL_PORT` (défaut
**3010**) et n'importe quel script du réseau peut écrire sur l'écran.

```bash
# état du lien série (JSON)
curl -s hote:3010/status

# afficher du texte — corps brut, pas du JSON ; paginé comme une réponse
curl -d "SAUVEGARDE TERMINEE" hote:3010/text

# afficher un flux Vidéotex brut (une image convertie, par exemple)
curl --data-binary @image.vtx hote:3010/show
```

De quoi faire, en trois lignes de cron ou de script : une notification de fin de
sauvegarde, l'affichage d'une alerte de supervision, un dessin du jour.

### Envoyer une image

```bash
python3 tools/img2vtx.py photo.png -o photo.vtx --gray
python3 tools/vtx-preview.py photo.vtx          # vérifier sans mobiliser le Minitel
curl --data-binary @photo.vtx hote:3010/show
```

Deux modes de rendu : **adaptatif** (quantification 2 tons par case — bon pour
les logos et le graphisme net) et **`--gray`** (niveaux de gris 40×24 — meilleur
pour les portraits et les photos). Détails dans
[image-conversion.md](image-conversion.md).

Le TUI fait la même chose de façon confortable, conversion comprise :

```bash
cargo run --release --features tui --bin minitel-tui -- hote:3010
# puis, dans le champ de saisie :  /img photo.png --gray
```

### 🔓 Aucune authentification

L'API de contrôle écoute sur `0.0.0.0` et **n'a aucun contrôle d'accès**.
N'importe qui sur le réseau peut écrire sur l'écran. C'est acceptable sur un LAN
de confiance, pas au-delà : si la machine est exposée, **filtrez ce port au
pare-feu**. Ne l'ouvrez jamais sur Internet — le daemon n'a pas d'authentification
et n'en aura pas.

---

## Écrire pour 40 colonnes

C'est le travail de **votre** module, pas du daemon. Le daemon découpe les lignes
trop longues, mais il ne peut pas rattraper du Markdown ni un tableau.

| Règle | Pourquoi |
|---|---|
| **40 caractères par ligne**, maximum absolu | c'est la largeur physique de l'écran |
| **Pas de Markdown** | `**gras**` s'affiche *avec* les astérisques |
| **Pas d'art ASCII** à base de `╔═╗` | à cette résolution les caractères de biseau se soudent en un pâté illisible |
| **Une ligne vide entre les blocs** | c'est le seul « style » réellement disponible |
| **Chiffres et noms propres d'abord** | l'utilisateur lit un écran cathodique, pas un rapport |
| **MAJUSCULES pour un titre**, jamais pour une phrase entière | lisible en titre, pénible en corps de texte |
| Évitez les **majuscules accentuées** | elles ne s'affichent pas (voir plus haut). Écrivez `ELECTRICITE` en connaissance de cause |

Si votre module appelle un LLM, **donnez-lui ces règles dans son prompt système**.
Celui de `examples/backend/backend.py` les encode déjà — réutilisez-le tel quel :

```python
SYSTEM = """Tu réponds sur un Minitel : écran de 40 colonnes, 20 lignes utiles.

Règles absolues :
- Jamais plus de 40 caractères par ligne.
- Pas de Markdown (ni **, ni #, ni tableaux) : le Minitel ne le rend pas.
- Va droit au fait. Trois phrases valent mieux qu'un paragraphe.
- Chiffres et noms propres d'abord, explications ensuite.
- Utilise des MAJUSCULES pour un titre, jamais pour une phrase entière.
"""
```

### La disposition de l'écran, en mode conversation

Utile pour savoir de combien de place vous disposez réellement. Cette
disposition est **verrouillée par des tests** :

```
  1       en-tête (titre + pagination « p n/m »)
  3..20   fil de discussion — 18 lignes utiles
  21      « VOUS : » — sert aussi de ligne d'attente (spinner + chrono)
  22      saisie (1 ligne)
  23      vide — respiration avant le pied de page
  24      pied de page (navigation)
```

Votre texte s'affiche dans les **18 lignes** du fil, paginé automatiquement :
**Suite** = page suivante, **Retour** = page précédente, **Sommaire** = nouveau
fil. Vous pouvez donc renvoyer plus de 18 lignes sans crainte — mais rappelez-vous
que l'utilisateur devra paginer pour lire la suite. Trois phrases valent mieux
qu'un paragraphe.

---

## Et ensuite

- Les invariants et pièges pour éditer le pilote : **[AGENTS.md](../AGENTS.md)**
- Les séquences hex du Minitel 1B (norme STUM) :
  **[videotex-1b-cheatsheet.md](videotex-1b-cheatsheet.md)**
- Ce qui peut lâcher, et pourquoi :
  **[journal-de-bord.md](journal-de-bord.md)**
