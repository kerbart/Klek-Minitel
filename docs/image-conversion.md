# Afficher une image sur le Minitel 1B

Pipeline complet **image → écran Minitel**, validé sur matériel (2026-07-19).

```
image ──(convertisseur, poste de dev)──> .vtx ──scp──> hôte ──minitel-show──> 📟
```

Le `.vtx` = flux d'octets Vidéotex prêts à envoyer. On le fabrique sur son poste,
on le pousse sur la machine reliée au Minitel, et `minitel-show` l'affiche.

## Les 2 modes de rendu — chacun son usage

| Mode | Outil | Idéal pour | Rendu |
|---|---|---|---|
| **Adaptatif** | `tools/img2vtx.py` (défaut) | **logos / texte** contrastés | 2 gris dominants par case 2×3 → aplats nets |
| **Niveau de gris** | `tools/img2vtx.py --gray` | **portraits / photos** | 1 gris uni par case (8 niveaux), 40×24, sans tramage |

Règle empirique : **aplats/texte → adaptatif**, **photos/visages → gris**.
Les images **texturées** (motifs fins, pois, rayures) rendent mal dans les deux
modes à 40×24 → préférez une **silhouette 2-tons** (flou fort + seuil) plutôt
qu'une conversion directe.

### Résolution utile
- Mosaïque (adaptatif/dither) : **80×72 blocs** (2×3 par case sur 40×24).
- Gris uni : **40×24 cases** (1 gris/case) — plus grossier mais vrai grayscale.

## Outils

### `tools/img2vtx.py` (Python 3 + Pillow)
Quantification 2 couleurs adaptative par case 2×3 + compression `REP`, **plus un
mode `--gray`** (1 niveau de gris uni par case).

Seule dépendance : `pip install Pillow`.

```bash
# logo / texte (adaptatif)
python3 tools/img2vtx.py logo.png -o logo.vtx
# portrait (niveaux de gris)
python3 tools/img2vtx.py portrait.jpg -o portrait.vtx --gray
# bannière d'accueil, positionnée en ligne 2
python3 tools/img2vtx.py logo.png -o logo.vtx --row 2
# autres options : --cols N --rows N --disjoint
```

### `tools/vtx-preview.py` (Python 3, sans dépendance)
Rend un `.vtx` en art ASCII dans le terminal — pour itérer sur une conversion
sans mobiliser le Minitel.

```bash
python3 tools/vtx-preview.py logo.vtx [--rows 1-12]
```

### `minitel-show` (Rust, embarqué)
Affiche un `.vtx` : init (écho off, curseur caché), efface écran + rangée 0,
envoie le flux, reste en vie (reconnexion auto).

```bash
minitel-show <fichier.vtx> [device]
```

## Afficher une image (recette)

```bash
# 1. convertir (sur votre machine de dev)
python3 tools/img2vtx.py mon-image.jpg -o img.vtx --gray

# 2. prévisualiser sans matériel
python3 tools/vtx-preview.py img.vtx

# 3. pousser sur la machine reliée au Minitel, puis afficher
scp img.vtx <user>@<hote>:~/minitel-driver/img.vtx
ssh <user>@<hote> '~/minitel-driver/minitel-show ~/minitel-driver/img.vtx'
```

Pour l'afficher **sans arrêter le daemon**, préférez l'API de contrôle :

```bash
curl --data-binary @img.vtx http://<hote>:3010/show
```

Pré-traitement utile (PIL) selon l'image : recadrage serré sur le sujet,
`ImageOps.autocontrast` (étale la dynamique), `GaussianBlur` léger (anti-speckle),
ou seuil dur pour une silhouette.

## ⚠️ Le piège G1 (vérifié sur matériel)

Le positionnement `1F L C` (comme `CR`/`LF`) **réinitialise le jeu en G0**.
Il faut donc, par ligne : **`1F L C` PUIS `0E` (SHIFT_OUT)** puis les octets
mosaïque. L'ordre inverse affiche du charabia alphanumérique. Cf.
`videotex-1b-cheatsheet.md`.

## Timings (4800 bauds ≈ 480 o/s)

| Image | Octets | Durée |
|---|---|---|
| Bannière de texte (adaptatif) | ~590 | ~1,2 s |
| Portrait niveaux de gris 18×24 | ~870 | ~1,8 s |
| Plein écran mosaïque dense | ~1000–1400 | ~2–3 s |

## Utiliser le résultat comme logo d'accueil

Un `.vtx` destiné à l'accueil doit tenir dans les **lignes 2 à 9** et se
positionner lui-même (`img2vtx.py --row 2`). Pointez-le ensuite avec
`MINITEL_LOGO=/chemin/logo.vtx` : le daemon l'envoie tel quel. Sans cette
variable, l'accueil affiche simplement `MINITEL_TITLE` en double taille.
