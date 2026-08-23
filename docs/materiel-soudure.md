# Souder le câble Minitel

Guide pour humain, écrit pour quelqu'un qui n'a jamais soudé. Trois soudures
suffisent pour ce projet — c'est un excellent premier exercice, et l'objet
obtenu sert vraiment.

**Prérequis :** avoir lu [materiel-branchement.md](materiel-branchement.md) et
**avoir déjà validé le montage en Dupont**. On ne soude pas pour découvrir si ça
marche ; on soude pour rendre définitif ce qui marche déjà.

- [Faut-il vraiment souder ?](#faut-il-vraiment-souder-)
- [L'outillage](#loutillage)
- [Le piège du connecteur mâle](#le-piège-du-connecteur-mâle)
- [Préparer](#préparer)
- [Souder, geste par geste](#souder-geste-par-geste)
- [La reprise de traction](#la-reprise-de-traction)
- [Vérifier avant de mettre sous tension](#vérifier-avant-de-mettre-sous-tension)
- [Rattraper une soudure ratée](#rattraper-une-soudure-ratée)

---

## Faut-il vraiment souder ?

Honnêtement : **non, pas forcément.** Trois fils Dupont enfoncés sur les broches
d'une fiche DIN mâle fonctionnent, et fonctionnent longtemps. Si le montage vit
sur une étagère et que personne ne tire dessus, vous pouvez rester comme ça.

Soudez si l'une de ces phrases vous concerne :

- le câble passe dans un endroit où on marche, on tire, on déplace ;
- vous en avez assez de perdre le contact quand vous bougez le Minitel ;
- vous voulez un objet fini, dont vous n'aurez plus à vous occuper ;
- vous voulez apprendre à souder sur quelque chose de tolérant.

Ce montage est effectivement **tolérant** : trois soudures, des pastilles larges,
pas de composant sensible à la chaleur, aucune piste fine à décoller. Si vous
ratez, vous recommencez. C'est le contraire d'une réparation de carte mère.

---

## L'outillage

### Le minimum réel

| Outil | Repère de prix | Ce qui compte |
|---|---|---|
| **Fer à souder** | 15–30 € | **Température réglable** si possible, panne fine ou moyenne. Un fer 25 W de supermarché fait le travail, mais chauffe mal et pardonne moins. |
| **Étain (soudure)** | 5 € | Fil de **0,7 à 1 mm**, **avec flux incorporé** (« résine », *rosin core*). C'est le flux qui fait que la soudure « mouille » le métal au lieu de perler. |
| **Éponge ou laine de bronze** | 3 € | Pour nettoyer la panne. Une panne sale ne transmet pas la chaleur. |
| **Pince coupante + pince à dénuder** | 8 € | Un cutter et des ongles marchent, mais dénuder au cutter coupe des brins. |
| **Multimètre** | 10 € | **Non négociable.** C'est lui qui vous dit que c'est bon avant que le courant vous le dise mal. |

### Le confort qui change tout

- **Un étau, une troisième main, ou du gaffer.** Vous avez deux mains : une pour
  le fer, une pour l'étain. Il n'en reste **aucune** pour tenir la pièce. C'est la
  cause n°1 des soudures ratées par un débutant.
- **De la gaine thermorétractable** (2 €) : isole proprement et fait office de
  reprise de traction. À défaut, du chatterton.
- **De la tresse à dessouder** (3 €) : pour rattraper un excès d'étain.

### Sécurité, brièvement mais sérieusement

- La panne est à **300–350 °C**. Elle ne fait pas mal *tout de suite* — elle fait
  très mal ensuite. Posez toujours le fer sur son support, jamais sur la table.
- **Ventilez.** Les fumées de flux sont irritantes. Fenêtre ouverte, et ne soudez
  pas le nez à 10 cm de la pièce.
- L'étain sans plomb est la norme grand public aujourd'hui ; s'il contient du
  plomb (vieux stock), **lavez-vous les mains** après.
- Ne soudez **jamais** sur un appareil sous tension ou branché. Minitel éteint,
  débranché du secteur, adaptateur débranché du Pi.

---

## Le piège du connecteur mâle

**C'est la partie la plus importante de ce document.** Lisez-la deux fois.

Le diagramme de brochage de la notice montre la **prise femelle du Minitel, vue
de l'extérieur du terminal**. Vous, vous allez souder sur une **fiche mâle, vue
du côté des soudures** — c'est-à-dire de derrière.

Or de derrière, l'arrangement est **inversé en miroir**. Une broche qui apparaît
à gauche sur le diagramme de la prise se retrouve à droite sur le cul de la
fiche. Souder en recopiant naïvement le diagramme donne un câble qui a l'air
parfait et qui inverse les signaux.

```
     Prise du Minitel                Fiche mâle, côté soudure
     (vue de l'extérieur)            (ce que vous avez sous les yeux)

            3                                  3
          •   •                              •   •
        2 • 5 • 4                    ← 4 •   5   • 2 →      ← MIROIR
            •                                  •
            1                                  1

     ⚠️ Ces deux vues ne se lisent PAS dans le même sens.
```

### La seule méthode qui ne trompe pas

N'essayez pas de raisonner sur le miroir. **Mesurez.** Beaucoup de fiches DIN
portent de minuscules numéros moulés près des cosses — s'ils sont là, faites-leur
confiance, puis vérifiez quand même.

Procédure, fiche **non soudée**, Minitel **éteint et débranché** :

1. Insérez la fiche nue dans la prise du Minitel.
2. Multimètre en **continuité** (le bip).
3. Une pointe sur le **châssis métallique** du Minitel (masse), l'autre promenée
   sur les cosses de la fiche, côté soudure.
4. La cosse qui bipe est reliée à la **broche 2** (le 0 V). **Marquez-la
   immédiatement** — un point de feutre indélébile sur le plastique de la fiche.

Vous avez maintenant un point d'ancrage certain. Pour lever la dernière
ambiguïté (savoir de quel côté sont 1 et 3), reprenez la mesure de tension
décrite dans [materiel-branchement.md](materiel-branchement.md#étape-1--trouver-la-masse-et-le-85-v) :
la broche à **8,5 V** est la 5, elle est au centre, et connaître 2 **et** 5 fixe
définitivement l'orientation.

5. Notez au feutre, sur la fiche, l'affectation de chaque cosse **avant** de
   sortir le fer. Trois lettres suffisent : `T` (vers TX adaptateur, broche 1),
   `G` (masse, broche 2), `R` (vers RX adaptateur, broche 3).

> Cinq minutes de mesure évitent une heure de « pourtant j'ai bien suivi le
> schéma ». C'est le meilleur rapport temps/frustration de tout le projet.

---

## Préparer

### 1. Choisir les couleurs et s'y tenir

Une convention, n'importe laquelle, mais **écrite** :

| Fil | Broche Minitel | Va vers |
|---|---|---|
| **Rouge** | 1 (RX du Minitel) | **TX** de l'adaptateur |
| **Noir** | 2 (0 V) | **GND** de l'adaptateur |
| **Jaune** | 3 (TX du Minitel) | **RX** de l'adaptateur |

Le noir pour la masse est une convention universelle : respectez-la, elle vous
sauvera un jour où vous relirez ce câble sans la documentation.

### 2. Dénuder court

**3 à 4 mm de cuivre nu, pas plus.** Un dénudage trop long laisse du cuivre à
l'air une fois la soudure faite : deux fils voisins finissent par se toucher,
surtout dans un capot serré.

### 3. Torsader et pré-étamer

Torsadez les brins entre les doigts, puis **pré-étamez** : posez la panne sur le
cuivre, touchez avec l'étain, laissez-le s'infiltrer dans les brins. Le fil doit
devenir gris brillant et rigide.

Un fil pré-étamé se soude ensuite en **une seconde**, sans que les brins
s'éparpillent. Ne sautez pas cette étape, c'est elle qui rend la suite facile.

### 4. Étamer la panne

Panne chaude, un peu d'étain dessus, essuyez sur l'éponge humide. Elle doit
briller. Une panne noircie et mate **ne transmet pas la chaleur** — vous
appuierez de plus en plus fort en vous demandant pourquoi rien ne fond.

---

## Souder, geste par geste

Le principe, en une phrase : **on chauffe la pièce, pas l'étain.** L'étain doit
fondre au contact du métal chaud et être aspiré par capillarité. Si vous faites
fondre l'étain sur la panne pour le déposer comme de la colle, vous obtenez une
**soudure sèche** : ça tient mécaniquement, ça ne conduit pas fiablement, et ça
lâchera dans six mois.

Pour chacun des trois fils :

1. **Immobilisez la fiche.** Étau, troisième main, ou gaffer sur la table. Les
   deux mains libres.
2. **Chauffez la cosse** environ **1 à 2 secondes**, panne bien en contact.
3. **Amenez l'étain** au point de jonction cosse/panne. Il doit fondre
   immédiatement et **couler dans la cosse**. Une petite goutte suffit.
4. **Posez le fil pré-étamé** dans la cosse, maintenez la panne encore
   **1 seconde** : les deux étains fusionnent.
5. **Retirez la panne, puis l'étain.** Ne bougez plus le fil pendant environ
   **3 secondes** — l'étain se solidifie en refroidissant, et bouger pendant ce
   temps crée une microfissure invisible.

### Reconnaître une bonne soudure

| | Aspect | Diagnostic |
|---|---|---|
| ✅ | **Brillante, lisse, en forme de petit volcan** qui épouse la cosse et le fil | Bonne. Passez à la suivante. |
| ❌ | **Mate, grise, granuleuse** | Soudure sèche (pièce pas assez chaude, ou bougée en refroidissant). Reprenez : chauffez à nouveau jusqu'à refonte complète. |
| ❌ | **Grosse boule qui ne mouille pas**, posée comme une perle | L'étain n'a pas accroché le métal. Trop d'étain, pas assez de chaleur, ou surface oxydée. Retirez à la tresse et refaites. |
| ❌ | **Ponts entre deux cosses voisines** | Court-circuit. Retirez l'excès à la tresse à dessouder. **Ne mettez pas sous tension avant d'avoir corrigé.** |
| ❌ | Le fil **bouge** quand on le tire doucement | Contact mécanique seul. Refaites. |

Trois soudures, trois vérifications visuelles. Prenez le temps : c'est plus rapide
que de diagnostiquer un faux contact intermittent la semaine suivante.

---

## La reprise de traction

L'erreur classique du débutant : des soudures impeccables, et six mois plus tard
un fil arraché. **La soudure assure le contact électrique, pas la tenue
mécanique.** Tirer sur un fil soudé, c'est tirer sur la soudure elle-même.

Par ordre de qualité :

1. **Le capot de la fiche DIN avec son serre-câble.** La plupart en ont un : une
   petite bride qui pince la gaine extérieure. C'est fait pour ça, utilisez-le.
2. **Gaine thermorétractable** sur chaque soudure (isolation) **plus** un manchon
   plus large sur l'ensemble du faisceau juste avant la fiche.
3. **Un nœud simple** dans le câble à l'intérieur du capot, avant les soudures :
   la traction s'exerce sur le nœud contre le plastique, pas sur le cuivre.
4. Au minimum : **une boucle de gaffer** autour du faisceau, collée au capot.

Faites-en au moins une. Le test : tirez fermement sur le câble — la contrainte
doit se sentir dans le capot, jamais au niveau des cosses.

---

## Vérifier avant de mettre sous tension

**Ne branchez rien avant d'avoir fait ces trois mesures.** Elles prennent deux
minutes et interceptent tout ce qui pourrait détruire du matériel.

Multimètre en **continuité**, rien n'est branché ni alimenté.

### Test 1 — chaque fil va bien où il doit aller

Une pointe sur la cosse de la fiche DIN, l'autre sur l'extrémité correspondante
côté adaptateur. Ça doit biper, pour les trois fils.

Un fil qui ne bipe pas = soudure sèche ou fil coupé, malgré une belle apparence.

### Test 2 — aucun court-circuit entre fils

Testez les trois paires : rouge↔noir, rouge↔jaune, noir↔jaune. **Aucune ne doit
biper.** Si l'une bipe, vous avez un pont d'étain ou deux dénudages qui se
touchent dans le capot. Corrigez avant d'aller plus loin.

### Test 3 — la broche 5 est bien libre

Le plus important. Une pointe sur la cosse de la **broche 5** de votre fiche
(celle du 8,5 V, identifiée plus haut), l'autre promenée successivement sur
**chacun** de vos trois fils.

**Aucun bip. Jamais.** Un bip ici signifie que le 8,5 V arrivera sur votre
adaptateur dès la mise sous tension, et le détruira — avec peut-être le port USB
et le Pi. Si ça bipe, coupez le fil fautif et refaites la soudure.

### Puis, dans l'ordre

1. Minitel éteint, insérez la fiche.
2. Branchez l'adaptateur sur le Pi.
3. Allumez le Pi, attendez le démarrage complet.
4. Allumez le Minitel.
5. Lancez le daemon et regardez `connected` passer à `true` :
   ```bash
   curl -s <ip-du-pi>:3010/status
   ```

Si `connected:false` avec un câble qui a passé les trois tests, ce n'est pas la
soudure : c'est presque toujours **TX/RX inversés** (le miroir du connecteur vous
a eu). Inversez les deux fils de signal côté adaptateur — c'est un geste de dix
secondes, aucune soudure à reprendre, puisque l'inversion se fait sur les
broches Dupont de l'adaptateur.

> C'est d'ailleurs un bon argument pour **souder côté DIN et rester en Dupont
> côté adaptateur** : l'extrémité fragile et définitive est protégée dans le
> capot, et l'extrémité où l'on se trompe reste réversible.

---

## Rattraper une soudure ratée

Rien n'est perdu, dans aucun cas.

| Problème | Solution |
|---|---|
| Soudure mate / sèche | Rechauffez jusqu'à refonte complète, ajoutez un soupçon d'étain frais (son flux relance le mouillage), laissez refroidir sans bouger |
| Trop d'étain, pont entre cosses | Panne chaude + **tresse à dessouder** posée sur l'excès : elle l'aspire. À défaut, chauffez et essuyez vivement sur l'éponge |
| Fil arraché | Redénudez 3 mm, pré-étamez, refaites. La cosse est réutilisable indéfiniment |
| Cosse noire d'oxyde | Grattez délicatement (cutter, papier de verre très fin), pré-étamez |
| Vous avez soudé les mauvaises broches | Dessoudez les trois, remesurez la correspondance, refaites. C'est la deuxième fiche qui sert — vous en avez acheté deux |
| Vous avez soudé un fil sur la broche 5 | Dessoudez-le **immédiatement**, avant toute mise sous tension. Vérifiez au multimètre que plus rien n'y touche |

Un connecteur DIN supporte sans problème plusieurs cycles de soudure/dessoudure.
Les cosses sont larges, il n'y a pas de piste fine à décoller.

---

## Et ensuite

- Installer le logiciel : **[install-raspberry-pi.md](install-raspberry-pi.md)**
- Lui faire afficher vos données : **[creer-un-module.md](creer-un-module.md)**
- Comprendre ce qui peut lâcher ensuite :
  **[journal-de-bord.md](journal-de-bord.md)** — sur onze pannes majeures de ce
  montage, six étaient physiques. Le câble que vous venez de fabriquer n'en fait
  probablement pas partie ; l'alimentation du Pi, elle, oui.
