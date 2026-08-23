# Brancher un Minitel sur un Raspberry Pi

Guide pour humain, à lire avant d'acheter et avant de câbler. Il vise un
objectif précis : que votre premier branchement fonctionne, et que si vous vous
trompez, vous ne détruisiez rien.

**Le résumé, si vous ne lisez qu'une chose :** trois fils, jamais la broche 5,
et vérifiez au multimètre avant de mettre sous tension.

- [Ce qu'il faut acheter](#ce-quil-faut-acheter)
- [Comprendre la liaison](#comprendre-la-liaison)
- [Le brochage](#le-brochage)
- [Identifier les broches pour de vrai](#identifier-les-broches-pour-de-vrai)
- [Câbler](#câbler)
- [Le premier contact](#le-premier-contact)
- [L'alimentation du Pi](#lalimentation-du-pi--la-panne-que-tout-le-monde-fait)
- [Quand rien ne s'affiche](#quand-rien-ne-saffiche)

---

## Ce qu'il faut acheter

| Élément | Repère de prix | À savoir avant d'acheter |
|---|---|---|
| **Minitel 1B** | 15–40 € | Brocante, Emmaüs, Leboncoin. **Demandez une photo de l'écran allumé** : un tube fatigué ou un clavier mort ne se voit pas sur une photo d'objet éteint. Vérifiez qu'il a bien une prise DIN à l'arrière (voir plus bas). |
| **Raspberry Pi** | 15–80 € | N'importe lequel, même un Zero W : le binaire pèse ~1 Mo et le CPU ne fait rien. À 4800 bauds, le fil est le goulet d'étranglement, jamais le processeur. |
| **Adaptateur USB-UART TTL** | 2–8 € | Un module **CH340** ou **CP2102** suffit. Prenez-en un qui expose au moins TX, RX, GND — et si possible un cavalier ou une pastille **3,3 V / 5 V**. |
| **Prise DIN 5 broches mâle** | 1–3 € | Le connecteur qui entre dans le Minitel. Achetez-en **deux** : le premier sert à apprendre à souder. |
| **Fils Dupont femelle-femelle** | 2 € le lot | Pour tester **avant** de souder quoi que ce soit. Indispensable. |
| **Multimètre** | 10 € | Le seul outil qui vous dira si vous avez raison. Ne sautez pas cette ligne. |

Total réaliste : **40 à 60 €** si vous partez de zéro et que le Minitel est bon marché.

> **Pourquoi le modèle compte.** Ce pilote cible le **Minitel 1B** via sa prise
> péri-informatique. Les autres modèles (1, 2, 12, Magis…) partagent l'essentiel
> du protocole Vidéotex, mais n'ont pas été testés ici : brochage, vitesses
> disponibles et comportement au démarrage peuvent différer. Si vous avez un
> autre modèle, tout ce document reste utile — vérifiez simplement chaque
> affirmation électrique contre la notice de *votre* terminal.

---

## Comprendre la liaison

Ce que vous êtes en train de fabriquer est une **liaison série asynchrone**, la
même famille que le RS-232, mais en niveaux logiques directs (TTL) au lieu des
±12 V du RS-232 historique.

```
┌─────────────┐                      ┌──────────────┐         ┌──────────┐
│  Minitel 1B │   3 fils (TX/RX/GND) │  adaptateur  │   USB   │Raspberry │
│  prise DIN  │◄────────────────────►│  USB-UART    │◄───────►│   Pi     │
└─────────────┘   7 bits, parité     └──────────────┘         └──────────┘
                  paire, 1 stop
                  1200 puis 4800 bd
```

Trois choses à retenir, elles expliquent tout le reste :

1. **Les paramètres série ne sont pas négociables.** Le terminal impose
   **7 bits de données, parité paire, 1 bit de stop** (7E1). Ce n'est pas un
   choix du logiciel, c'est le matériel. Les vitesses possibles sur la DIN sont
   **300, 1200 et 4800 bauds** — et rien au-dessus.

2. **Le 1B n'a pas de mémoire de configuration.** Il redémarre *toujours* à
   1200 bauds, quoi qu'on lui ait dit avant. Le pilote gère ça tout seul : il
   tente 4800 directement, sinon ouvre à 1200, envoie la commande de changement
   de vitesse, et réouvre à 4800. Vous n'avez **rien** à régler. Si vous voyez
   l'affichage se remplir lentement, caractère par caractère, c'est que le lien
   est retombé à 1200 — normal après une reconnexion.

3. **Les niveaux sont TTL collecteur ouvert.** C'est écrit dans la notice
   constructeur. En pratique un module CH340 se raccorde directement. Le seul cas
   qui coince : le Minitel émet mais rien n'arrive côté Pi — voir
   [Quand rien ne s'affiche](#quand-rien-ne-saffiche).

---

## Le brochage

Voici le diagramme **tel qu'il figure dans la notice constructeur du Minitel 1B**,
section « Prise péri-informatique » :

```
     3
   •   •
 2 • 5 • 4
     •
     1
```

| Broche | Signal (vocabulaire de la notice) | Ce que ça veut dire pour vous |
|---|---|---|
| **1** | Entrée RX — réception de données venant du périphérique | Le Minitel **écoute** ici → à relier au **TX** de l'adaptateur |
| **2** | Référence zéro volt du terminal | La masse → **GND** de l'adaptateur |
| **3** | Sortie TX — émission de données vers le périphérique | Le Minitel **parle** ici → à relier au **RX** de l'adaptateur |
| **4** | Entrée PT — périphérique prêt à travailler | **Non utilisé** par ce pilote. Laissez libre. |
| **5** | Sortie **8,5 volts / 1 A** | ⚠️ **NE RIEN Y BRANCHER.** Voir ci-dessous. |

### ⚠️ La broche 5

Elle délivre **8,5 V sous 1 A**. C'est une sortie d'alimentation prévue pour des
périphériques d'époque. Un adaptateur USB-UART travaille en 3,3 V ou 5 V.

Relier la broche 5 à une entrée de votre adaptateur, c'est le détruire — et
potentiellement emporter le port USB du Pi avec, voire le Pi. Ce n'est pas une
précaution théorique : 8,5 V sur une broche prévue pour 3,3 V, c'est plus du
double de la tension admissible.

**Ne câblez que 1, 2 et 3.** Il n'y a aucune raison de toucher aux broches 4 et 5.

### Le croisement TX/RX

C'est l'erreur numéro un des débutants, alors posons-la clairement : **TX d'un
côté va sur RX de l'autre.** Un émetteur parle à un récepteur.

```
Minitel broche 1 (son RX, il écoute)  ←────  TX de l'adaptateur (il parle)
Minitel broche 2 (0 V)                ─────  GND de l'adaptateur
Minitel broche 3 (son TX, il parle)   ────→  RX de l'adaptateur (il écoute)
```

Si vous branchez TX sur TX et RX sur RX, **rien ne casse** — vous avez juste deux
émetteurs qui se parlent dans le vide et deux récepteurs qui attendent. L'écran
reste noir, `connected:false`. C'est la première chose à inverser quand ça ne
marche pas.

---

## Identifier les broches pour de vrai

**Ne faites pas confiance à un schéma redessiné — y compris le mien.**

Un diagramme de connecteur est ambigu de nature : est-il vu de face ou de dos ?
côté prise ou côté fiche ? Et sur une fiche **mâle**, l'arrangement est le
**miroir** de celui de la prise femelle. C'est exactement là que les montages
partent de travers.

La seule méthode fiable, avec un multimètre, en trois minutes :

### Étape 1 — trouver la masse et le 8,5 V

Minitel **allumé**, multimètre en mode **tension continue** (20 V), pointe noire
sur une masse connue (le châssis métallique, ou la broche 2 supposée) :

- Vous devez trouver **≈ 8,5 V sur une seule broche** : c'est la **broche 5**.
  Notez-la. C'est celle qu'il ne faut plus jamais toucher.
- La broche par rapport à laquelle vous lisez 0 V et qui a une continuité avec le
  châssis, c'est la **broche 2** (masse).

Ces deux repères suffisent à orienter le connecteur : vous savez maintenant dans
quel sens lire le diagramme de la notice, et vous pouvez en déduire 1, 3 et 4.

### Étape 2 — confirmer par continuité, Minitel **éteint** et débranché

Multimètre en mode **continuité** (le bip). Testez entre chaque broche de votre
**fiche mâle** (côté soudure) et la broche correspondante de la prise, fiche
insérée. Vous cartographiez ainsi vos propres fils : « le fil rouge arrive sur la
broche que j'ai identifiée comme 1 ».

C'est cette cartographie-là qui compte, pas le dessin.

### Étape 3 — noter, physiquement

Un bout de gaffer sur le câble, au feutre : `R=1(TX-adapt) N=2(GND) J=3(RX-adapt)`.
Dans six mois, vous ne vous en souviendrez pas, et vous ne recommencerez pas la
mesure.

> **Le raccourci honnête :** si vous n'avez pas de multimètre, câblez d'abord en
> Dupont (rien de définitif), et **ne branchez que 2 et 3** — masse et sortie du
> Minitel. Lancez le daemon : s'il reçoit des octets quand vous tapez sur le
> clavier, vous avez trouvé le TX du Minitel et la masse. Ajoutez le troisième
> fil ensuite. Vous n'aurez jamais mis de tension sur une entrée.

---

## Câbler

### Toujours en Dupont d'abord

Résistez à l'envie de souder tout de suite. Avec trois fils Dupont
femelle-femelle enfoncés sur les broches de la fiche DIN (ou tenus à la main
contre les contacts de la prise, ça suffit pour un test de dix secondes), vous
validez la chaîne complète **avant** d'immobiliser quoi que ce soit.

Ordre de branchement — il a son importance :

1. **Le Minitel est éteint.** Le Pi est éteint aussi.
2. Câblez **GND en premier**, toujours. Une masse établie avant les signaux évite
   qu'un signal cherche son retour par un chemin imprévu.
3. Câblez TX puis RX.
4. Branchez l'adaptateur sur le Pi.
5. Allumez le Pi, laissez-le démarrer.
6. Allumez le Minitel **en dernier**.

### Trouver le port série sur le Pi

```bash
ls -l /dev/serial/by-id/
# usb-1a86_USB_Serial-if00-port0 -> ../../ttyUSB0   (typique d'un CH340)
```

**Utilisez toujours le chemin `/dev/serial/by-id/…`, jamais `/dev/ttyUSB0`.** Le
numéro change au gré des rebranchements et des ports USB ; le chemin `by-id` est
stable. C'est la première cause de service qui ne redémarre pas après un reboot,
et elle est pénible à diagnostiquer parce que tout marchait la veille.

### Les permissions

```bash
sudo usermod -aG dialout "$USER"   # puis se déconnecter / reconnecter
```

Sans ça : `Permission denied` sur le device. Le changement de groupe ne prend
effet qu'à la session suivante — un `su - $USER` ou une reconnexion SSH suffit.

---

## Le premier contact

Vous n'avez pas besoin d'un backend fonctionnel pour valider le câblage. Le
daemon affiche son écran d'accueil dès que le lien série répond.

```bash
# sur le Pi, ou via le déploiement automatique (voir install-raspberry-pi.md)
RUST_LOG=info ./miniteld /dev/serial/by-id/usb-1a86_USB_Serial-if00-port0
```

Ce que vous devez voir, dans l'ordre :

| Signe | Signification |
|---|---|
| `lien série ouvert` dans les logs | Le device existe et vous avez les droits |
| `Connected` / `baud=4800` | **Le câblage est bon.** La négociation de vitesse a réussi |
| L'écran d'accueil s'affiche | Le Minitel comprend le Vidéotex qu'on lui envoie |
| Vous tapez, ça apparaît | La voie retour (broche 3 → RX) fonctionne |

Et depuis une autre machine :

```bash
curl -s <ip-du-pi>:3010/status
# {"connected":true,"baud":4800,"idle_secs":2,"sleeping":false,"net":"offline"}
```

`"connected":true` = le fil est bon. `"net":"offline"` à ce stade est **normal** :
c'est votre backend qui n'est pas là, pas un problème de câblage. Les deux
diagnostics sont volontairement séparés pour que vous ne cherchiez pas la panne
du mauvais côté.

---

## L'alimentation du Pi — la panne que tout le monde fait

Lisez ce paragraphe même si vous croyez que ça ne vous concerne pas. Sur ce
montage, **la panne la plus coûteuse en temps n'était ni le Minitel ni le
logiciel : c'était un câble micro-USB.**

Le symptôme était trompeur au possible : décrochages Wi-Fi, reboots spontanés
du Pi, et **un Minitel figé, écran gelé**. Quatre hypothèses ont été explorées
avant la bonne (« le Minitel a un bug », « le daemon a planté », « l'alim est
trop faible », « l'UART tire trop »). Toutes fausses.

La cause : un câble aux conducteurs trop fins. Chute de tension sous charge, la
puce Wi-Fi brownout la première — d'où l'illusion d'un problème réseau. La
résolution a été de remplacer le câble par un autre visiblement plus épais.

**Le réflexe à prendre, avant même de lire un log applicatif :**

```bash
vcgencmd get_throttled
# 0x0       : sain
# 0x50000   : un pic au boot seulement — acceptable
# bit 0 à 1 : sous-tension EN COURS → cherchez le câble, pas le bug
```

Le raisonnement qui a débloqué l'affaire vaut d'être gardé : *le lien Minitel ↔ Pi
est série, donc totalement indépendant du Wi-Fi. Un Minitel figé **et** un Pi
injoignable en SSH ne peut pas être un bug d'affichage — c'est le Pi qui a
redémarré.* Séparer les domaines de panne fait plus que n'importe quel correctif.

Le récit complet est dans [journal-de-bord.md](journal-de-bord.md), section 1.

---

## Quand rien ne s'affiche

| Symptôme | Cause la plus probable | Geste |
|---|---|---|
| `Permission denied` sur le device | utilisateur absent du groupe `dialout` | `usermod -aG dialout`, puis reconnexion |
| Aucun `/dev/serial/by-id/` | adaptateur non reconnu | `dmesg \| tail -20` après branchement ; câble USB ou module mort |
| `connected:false`, écran noir | **TX/RX inversés** (broches 1 ↔ 3) | inverser les deux fils de signal |
| `connected:false` toujours | **GND absent** | c'est le fil qu'on oublie ; sans masse, aucun niveau logique n'a de sens |
| L'écran affiche, mais le clavier ne remonte rien | broche 3 (TX du Minitel) mal câblée, ou collecteur ouvert sans tirage | ajouter un **pull-up ~10 kΩ** entre cette ligne et le VCC de l'adaptateur |
| Tout est lent, caractère par caractère | lien retombé à **1200 bauds** | normal après une reconnexion ; il repassera à 4800 au prochain cycle |
| Comportement erratique, gels, reboots | **alimentation du Pi** | `vcgencmd get_throttled` — voir section ci-dessus |
| Charabia alphanumérique à l'écran | problème de protocole, **pas de câblage** | voir [journal-de-bord.md](journal-de-bord.md) §9 (mosaïque G1) |

Le cas du **pull-up** mérite un mot, parce qu'il surprend : la notice précise que
les niveaux sont « TTL collecteur ouvert ». Un étage à collecteur ouvert sait
tirer une ligne vers 0 V mais **pas** la ramener activement au niveau haut : il
faut une résistance qui fasse remonter la ligne au repos. La plupart des modules
USB-UART en intègrent une, d'où le fait que ça marche « tout seul » le plus
souvent. Si votre module n'en a pas, le Minitel semblera muet alors qu'il émet
correctement. Une résistance de l'ordre de **10 kΩ** entre la ligne de réception
de l'adaptateur et son VCC règle le cas.

---

## Et ensuite

- Vous voulez un montage définitif, propre, qui ne se débranche pas tout seul :
  → **[materiel-soudure.md](materiel-soudure.md)**
- Vous voulez installer le logiciel de bout en bout :
  → **[install-raspberry-pi.md](install-raspberry-pi.md)**
- Vous voulez lui faire afficher *vos* données :
  → **[creer-un-module.md](creer-un-module.md)**
