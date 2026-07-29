# Minitel 1B — Cheat-sheet Vidéotex (norme STUM 1B)

> Cible : Minitel 1B via prise péri-informatique (DIN 5), 4800 bauds, 7 bits, parité paire, 1 stop.
> Toutes les séquences sont en hexadécimal.

---

## 1. Codes de contrôle C0 (octet seul)

| Hex | Nom | Effet |
|-----|-----|-------|
| `05` | ENQ | Demande d'identification (réponse ROM) |
| `07` | BEL | Bip sonore |
| `08` | BS | Curseur ← 1 colonne |
| `09` | HT | Curseur → 1 colonne |
| `0A` | LF | Curseur ↓ 1 ligne |
| `0B` | VT | Curseur ↑ 1 ligne |
| `0C` | FF | **Clear screen** + curseur en (1,1) + attributs réinitialisés |
| `0D` | CR | Curseur en colonne 1 de la ligne courante |
| `0E` | SO | Passage en **G1 (mosaïque)** |
| `0F` | SI | Passage en **G0 (alphanumérique)** |
| `11` | DC1 (Con) | Curseur **visible** |
| `12` | REP | Répétition : `12` + `0x40+n` répète le dernier caractère n fois (n ≤ 63) |
| `13` | SEP (DC3) | Préfixe des séquences clavier Minitel (touches fonction) |
| `14` | DC4 (Coff) | Curseur **caché** |
| `18` | CAN | Effacement du curseur à la fin de ligne (remplissage espaces) |
| `19` | SS2 | Accès **G2** pour 1 caractère (accents, symboles) |
| `1A` | SUB | Caractère d'erreur (affiche un ▚) |
| `1B` | ESC | Introducteur d'échappement (attributs, CSI, PRO) |
| `1E` | RS | Home : curseur en (1,1), **sans** effacement |
| `1F` | US | Introducteur d'**adressage direct** |

---

## 2. Positionnement du curseur

### Adressage direct (le plus utilisé)
```
1F  (0x40 + ligne)  (0x40 + colonne)
```
- Ligne : 1–24 → `0x41`–`0x58`
- Colonne : 1–40 → `0x41`–`0x68`
- Exemple ligne 5, colonne 10 : `1F 45 4A`
- Réinitialise les attributs en cours (repartir proprement à chaque zone).

### Rangée 0 (ligne de service, 40 colonnes, au-dessus de l'écran)
```
1F 40 41   → entrée rangée 0, colonne 1
```
- Attributs limités (pas de double hauteur).
- Sortie obligatoire par un nouvel adressage `1F` vers l'écran ou `0A`/`0D` (le contenu de la rangée 0 n'est pas déplacé par le rouleau).

### Séquences CSI — `1B 5B` (spécifiques 1B, absentes du Minitel 1)
| Séquence | Effet |
|----------|-------|
| `1B 5B` n `41` | Curseur ↑ n lignes (`n` en ASCII décimal, ex. `32` = "2") |
| `1B 5B` n `42` | Curseur ↓ n lignes |
| `1B 5B` n `43` | Curseur → n colonnes |
| `1B 5B` n `44` | Curseur ← n colonnes |
| `1B 5B` l `3B` c `48` | Positionnement ligne;colonne (style ANSI, ex. `1B 5B 30 35 3B 31 30 48`) |
| `1B 5B 4A` | Effacement du curseur à la fin d'écran |
| `1B 5B 31 4A` | Effacement du début d'écran au curseur |
| `1B 5B 32 4A` | Effacement écran complet (curseur inchangé) |
| `1B 5B 4B` | Effacement fin de ligne (≈ CAN) |
| `1B 5B 31 4B` | Effacement début de ligne → curseur |
| `1B 5B 32 4B` | Effacement ligne complète |
| `1B 5B` n `4C` | Insertion de n lignes |
| `1B 5B` n `4D` | Suppression de n lignes |
| `1B 5B` n `50` | Suppression de n caractères |
| `1B 5B 34 68` | Mode insertion ON |
| `1B 5B 34 6C` | Mode insertion OFF |

---

## 3. Jeux de caractères

### G0 — Alphanumérique (`0F`)
ASCII standard 0x20–0x7F, **sans** accents. Quelques différences : pas de `` ` ``, `#` etc. selon version ROM.

### G1 — Mosaïque semi-graphique (`0E`)
> ⚠️ **Piège vérifié sur matériel (2026-07-19)** : le positionnement `1F` (et `0D`/`0A`) **réinitialise le jeu de caractères en G0**. Pour dessiner en mosaïque : émettre `1F L C` **PUIS** `0E` (SO), *ensuite* les octets G1. L'ordre inverse affiche du charabia alphanumérique.

- Chaque caractère = matrice **2×3 blocs**.
- Codes 0x20–0x3F et 0x60–0x7F : 64 motifs. Bit → bloc :

```
bit0 (0x01) : haut-gauche      bit1 (0x02) : haut-droit
bit2 (0x04) : milieu-gauche    bit3 (0x08) : milieu-droit
bit4 (0x10) : bas-gauche       bit5 (0x40) : bas-droit  (⚠ pas 0x20)
```
- Encodage : `code = 0x20 + (pattern & 0x1F) + (pattern & 0x20 ? 0x40 : 0x00)`
  - Tous blocs éteints : `0x20` (espace) — tous allumés : `0x7F`
- Codes 0x40–0x5F en G1 : majuscules (identiques à G0).
- **Pas d'accents en G1**, pas de double taille.

### G2 — Caractères spéciaux (`19` + code, un seul caractère)
| Séquence | Caractère |
|----------|-----------|
| `19 23` | £ |
| `19 24` | $ |
| `19 26` | # |
| `19 27` | § |
| `19 2C` | ← |
| `19 2D` | ↑ |
| `19 2E` | → |
| `19 2F` | ↓ |
| `19 30` | ° |
| `19 31` | ± |
| `19 38` | ÷ |
| `19 3C` | ¼ |
| `19 3D` | ½ |
| `19 3E` | ¾ |
| `19 6A` | Œ |
| `19 7A` | œ |
| `19 7B` | ß |

### Accents (diacritique G2 **puis** lettre G0)
| Séquence | Résultat |
|----------|----------|
| `19 41` + voyelle | accent grave (`19 41 61` = à) |
| `19 42` + voyelle | accent aigu (`19 42 65` = é) |
| `19 43` + voyelle | circonflexe (`19 43 6F` = ô) |
| `19 48` + voyelle | tréma (`19 48 65` = ë) |
| `19 4B 63` | ç (cédille) |

---

## 4. Attributs de visualisation (`1B` + code)

### Couleur de caractère (8 niveaux de gris sur écran mono)
| Séquence | Couleur | Gris |
|----------|---------|------|
| `1B 40` | noir | 0 % |
| `1B 41` | rouge | 50 % |
| `1B 42` | vert | 70 % |
| `1B 43` | jaune | 90 % |
| `1B 44` | bleu | 40 % |
| `1B 45` | magenta | 60 % |
| `1B 46` | cyan | 80 % |
| `1B 47` | blanc | 100 % |

### Couleur de fond
`1B 50` → `1B 57` (même ordre : noir…blanc).

### Autres attributs
| Séquence | Effet |
|----------|-------|
| `1B 48` | Clignotement ON |
| `1B 49` | Clignotement OFF (fixe) |
| `1B 4C` | Taille normale |
| `1B 4D` | Double hauteur |
| `1B 4E` | Double largeur |
| `1B 4F` | Double taille (2×2) |
| `1B 59` | Fin soulignement / mosaïque **jointive** |
| `1B 5A` | Début soulignement / mosaïque **disjointe** |
| `1B 5C` | Fond normal (fin inversion) |
| `1B 5D` | Inversion vidéo |
| `1B 58` | Masquage (invisible jusqu'à démasquage) |
| `1B 5F` | Démasquage |

### Règles de validation — les pièges
1. **Attributs "caractère"** (couleur avant-plan, taille, clignotement, inversion) : effet **immédiat**, portent sur chaque caractère affiché.
2. **Attributs "zone"** (couleur de **fond**, soulignement, masquage) en mode **alphanumérique** : effet **différé**, validé par un **espace délimiteur** (`20`). L'attribut court jusqu'au prochain délimiteur ou fin de ligne.
   → Pour un fond rouge : `1B 51 20` texte… puis `1B 50 20` pour clore.
3. En **mosaïque (G1)** : la couleur de fond est **immédiate** (pas de délimiteur).
4. **Double hauteur / double taille** : interdite en ligne 1 et rangée 0 (il faut la place au-dessus) ; interdite en G1 ; le caractère s'écrit sur la ligne courante **et celle du dessus**.
5. Retour ligne / adressage `1F` : réinitialise les attributs → toujours ré-émettre.
6. `1B 5A` en G1 = mosaïque **disjointe** (blocs séparés par un liseré de fond).

---

## 5. Compression / astuces d'affichage

| Technique | Séquence | Usage |
|-----------|----------|-------|
| Répétition | `12` + `0x40+n` | Aplats horizontaux (max 63, réémettre pour plus) |
| Effacement fin de ligne | `18` | Nettoyage rapide sans réécrire 40 espaces |
| Adressage direct | `1F L C` | Toujours moins cher que des déplacements unitaires au-delà de 3 |
| Rangée 0 | `1F 40 41` | Statuts/HUD hors zone scrollable |

---

## 6. Séquences protocole (PRO)

Préfixes : **PRO1** `1B 39` (+1 octet), **PRO2** `1B 3A` (+2 octets), **PRO3** `1B 3B` (+3 octets).

### Identification / état
| Séquence | Effet | Réponse |
|----------|-------|---------|
| `1B 39 7B` | ENQROM (identification) | `01` + constructeur + type + version + `04` (ex. type `76` = Minitel 1B) |
| `1B 39 72` | Demande position curseur | `1F` L C |
| `1B 3A 31` + module | STATUS terminal | mots d'état |

### Modes écran
| Séquence | Effet |
|----------|-------|
| `1B 3A 69 43` | **Rouleau ON** (scrolling en bas d'écran) |
| `1B 3A 6A 43` | Rouleau OFF (retour page, écrasement ligne 1) |
| `1B 3A 69 45` | Zoom/lois d'affichage (selon ROM) |
| `1B 3A 32 7D` | Passage **mode mixte** (80 col. téléinformatique, pas de couleur ni G1) |
| `1B 3A 32 7E` | Retour **Vidéotex** 40 colonnes |

### Clavier
| Séquence | Effet |
|----------|-------|
| `1B 3B 69 59 41` | Clavier **étendu** ON (touches curseur → séquences `1B 5B x`, minuscules directes) |
| `1B 3B 6A 59 41` | Clavier étendu OFF (mode Vidéotex standard) |
| `1B 3B 69 59 43` | Codes curseur en mode C0 |

### Aiguillage des modules (PRO3 `60`/`61`)
Modules : écran `58` (rx) / `50` (tx), clavier `59`/`51`, modem `5A`/`52`, prise DIN `5B`/`53`.

```
1B 3B 60 <destinataire_rx> <émetteur_tx>   → aiguillage OFF
1B 3B 61 <destinataire_rx> <émetteur_tx>   → aiguillage ON
```

| Séquence | Effet |
|----------|-------|
| `1B 3B 60 58 52` | Coupe modem → écran |
| `1B 3B 61 58 53` | Prise DIN → écran (mode périinformatique standard) |
| `1B 3B 60 5B 51` | **Coupe l'écho clavier → prise** (le Pi ne reçoit plus les frappes en double) |
| `1B 3B 61 5B 51` | Clavier → prise ON (le Pi reçoit les touches) |
| `1B 3B 60 58 51` | Coupe **écho local** clavier → écran (l'hôte gère l'affichage) |

> Acquittements : le Minitel répond par une séquence PRO3 `63` (statut d'aiguillage). Prévoir la lecture/purge dans le driver.

### Vitesse (PRO2 PROG `1B 3A 6B` + octet)
| Octet | Vitesse |
|-------|---------|
| `52` | 1200 bauds |
| `64` | 4800 bauds (max sur 1B) |
| `40` | 300 bauds |
> Format de l'octet : `0b01 P2 P1 P0 E2 E1 E0` (émission/réception identiques sur 1B). Réponse : PRO2 QUERY. 9600 = Minitel 2 uniquement.

### PCE — Procédure de Correction d'Erreur
| Séquence | Effet |
|----------|-------|
| `1B 39 44` | PCE ON |
| `1B 39 45` | PCE OFF |
> Utile sur modem bruité, inutile sur liaison DIN courte.

---

## 7. Codes clavier reçus (côté driver)

### Touches de fonction (préfixe `13` = SEP)
| Séquence | Touche |
|----------|--------|
| `13 41` | Envoi |
| `13 42` | Retour |
| `13 43` | Répétition |
| `13 44` | Guide |
| `13 45` | Annulation |
| `13 46` | Sommaire |
| `13 47` | Correction |
| `13 48` | Suite |
| `13 49` | Connexion/Fin |

### Clavier étendu activé
- Flèches : `1B 5B 41/42/43/44` (↑ ↓ → ←)
- Le pavé alphabétique envoie minuscules/majuscules normalement.

---

## 8. Limites du Minitel 1B (vs Minitel 2)

- Pas de **DRCS** (caractères redéfinissables) → pas de pseudo-bitmap au-delà de la mosaïque 2×3.
- Vitesse max **4800 bauds** sur la prise DIN.
- Écran monochrome : les 8 "couleurs" = 8 niveaux de gris.
- Pas de sauvegarde de configuration (tout est à ré-émettre après chaque mise sous tension : vitesse revient à 1200, rouleau OFF, écho ON).

---

## 9. Checklist d'init typique côté driver (liaison DIN)

```
1B 3A 6B 64          ; 4800 bauds (puis rebasculer l'UART du Pi)
1B 3B 60 58 51       ; écho local OFF
1B 3B 61 5B 51       ; clavier → prise ON
1B 3B 69 59 41       ; clavier étendu ON
1B 3A 69 43          ; rouleau ON (ou 6A 43 pour mode page)
0C                   ; clear screen
14                   ; curseur caché
```

---

## Correspondance avec le driver Rust (`minitel-rs`)

État au 2026-07-19 (ce qui est implémenté vs ce que la cheat-sheet ouvre) :

| Cheat-sheet | Implémenté ? | Où / à faire |
|---|---|---|
| C0 (FF, BS, CR, BEL, curseur on/off) | partiel | `protocol.rs` (clear, beep, cursor) |
| Adressage direct `1F L C` | ✅ | `protocol::move_to` |
| Accents G2 (SS2) | ✅ | `videotex::ss2_sequence` + `input` (décodage) |
| Couleur avant-plan | ✅ | `videotex::fg/colored` |
| Couleur de **fond** (délimiteur `20`) | ⚠️ `bg()` existe mais **règle du délimiteur non gérée** | à corriger |
| Double hauteur/largeur/taille (`1B 4D/4E/4F`) | ❌ | à ajouter |
| Clignotement / inversion / masquage | ❌ | à ajouter |
| Mosaïque G1 2×3 (SO/SI) | ❌ | gros potentiel (dessins/logos) |
| Répétition `12` (aplats) | ❌ | optimisation d'affichage |
| Rangée 0 (HUD/statut) | ❌ | idéal pour une barre d'état |
| Mode rouleau (`1B 3A 69 43`) | ❌ | utile pour du texte long |
| Clavier étendu ON (flèches) | ⚠️ décodé, **pas activé** à l'init | ajouter `1B 3B 69 59 41` |
| Négociation vitesse PRO2 | ✅ | `link::establish` (1200→4800) |
