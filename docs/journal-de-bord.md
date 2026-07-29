# Journal de bord — ce qui a résisté

Relier un Minitel de 1985 à un service moderne prend une soirée sur le papier. En
pratique, ça a pris des semaines, et **presque aucun des obstacles n'était du
code**. Ce document recense les pannes réelles, dans l'ordre où elles ont mordu,
avec leur cause racine et la fausse piste qui a précédé.

Il sert deux publics : celui qui monte le même bricolage chez lui et veut éviter
les mêmes murs, et celui qui se demande à quoi ressemble vraiment un projet
« hardware + IA » quand on enlève la démo.

Répartition finale des galères : **matériel et électrique en tête**, réseau
ensuite, protocole Vidéotex en dernier. L'inverse de l'intuition.

---

## 1. L'alimentation — le talon d'Achille, et ce n'était pas l'alim

**Nature** : électrique. **La panne la plus coûteuse du projet.**

**Symptôme** : décrochages Wi-Fi fréquents et inexpliqués. Reboots spontanés du
Pi. Et le symptôme trompeur : **un Minitel figé, écran gelé**.

**Fausse piste, tenace** : « le Minitel a un bug », puis « le daemon a planté »,
puis « l'alimentation est trop faible », puis « l'adaptateur UART tire trop de
courant ». Quatre hypothèses, toutes fausses.

**Cause réelle** : le **câble micro-USB**. Pas le chargeur, pas l'UART, pas le
logiciel. Un câble aux conducteurs trop fins provoque une chute de tension sous
charge ; la puce Wi-Fi, plus gourmande que le reste, brownout la première — d'où
l'illusion d'un problème réseau.

**Le raisonnement qui débloque** : le lien Minitel ↔ Pi est **série**, donc
totalement indépendant du Wi-Fi. Un Minitel figé **et** un Pi injoignable en SSH
ne peut donc pas être un bug d'affichage : c'est le Pi qui a redémarré. Ce
raisonnement-là a fait plus que n'importe quel correctif.

**Diagnostic reproductible** :

```bash
vcgencmd get_throttled
# 0x0       : sain
# 0x50000   : juste le pic de boot — acceptable
# bit 0 à 1 : sous-tension EN COURS -> chercher le câble
```

**Résolution — action physique** : remplacement par un **câble de charge de
sonnette Ring** (orange, gros conducteurs), simplement parce qu'il traînait et
que ses conducteurs étaient visiblement plus épais. Retour à `0x50000`, un seul
événement au boot, plus aucun décrochage.

> Leçon : sur un Raspberry Pi, `get_throttled` devrait être le **premier** réflexe
> devant tout comportement erratique, avant même de lire un log applicatif.

---

## 2. La carte SD morte, et le faux coupable

**Nature** : matériel.

**Symptôme** : le Pi 3 ne boote plus du tout, du jour au lendemain.

**Fausse piste** : le **lecteur de carte SD** du Pi, soupçonné puis mis hors de
cause. Un premier reflashage en 32 bits boot-crashait encore, ce qui semblait
confirmer que la machine était morte — alors que c'était simplement un flash
raté.

**Cause réelle** : la carte SD d'origine, usée. Le second symptôme (crash au boot
après reflash) était une panne **distincte et simultanée** — le piège classique
qui fait condamner le bon composant.

**Résolution — action physique** : reflashage en **64 bits**, qui boote du
premier coup, puis remplacement par une **carte SD neuve**. Lecteur innocenté.

**Effet de bord logiciel** : le Pi de secours était un **Pi Zero W v1**, en
**ARMv6** — et le binaire compilé pour ARMv7 **ne tourne pas** dessus. Il a fallu
ajouter la cible `arm-unknown-linux-musleabihf`. C'est pour cela que
`.cargo/config.toml` porte aujourd'hui trois cibles ARM.

---

## 3. Le Wi-Fi qui décroche — trois causes empilées

**Nature** : réseau. Le log d'enquête fait **3 968 lignes** pour **46
déconnexions** relevées.

**Symptôme** : le Pi disparaît du réseau à intervalles irréguliers et ne revient
pas seul.

**Trois causes distinctes, découvertes l'une après l'autre** :

1. **La sous-tension** (cf. §1) — la puce Wi-Fi lâche avant le reste.
2. **Le powersave Wi-Fi**, actif par défaut, plus **NetworkManager qui abandonne
   après 4 tentatives** de reconnexion. Corrigé en dur :

   ```ini
   # /etc/NetworkManager/conf.d/10-wifi-stability.conf
   [connection]
   wifi.powersave = 2
   [device]
   wifi.scan-rand-mac-address = no
   ```
   avec `autoconnect-retries=0` (réessayer indéfiniment).
3. **La position physique du Minitel.** Depuis ce coin de la maison, la borne
   2,4 GHz de la box **n'est pas audible du tout**. Seuls deux répéteurs le sont,
   à ~50 % et ~34 % de signal. Épingler la connexion sur la box — le réflexe
   naturel — garantissait l'échec.

**Résolution finale — action physique** : abandon du Wi-Fi. Passage en
**Ethernet via un adaptateur CPL**, ce qui a mis fin au problème définitivement.
Les contournements logiciels (timer de self-heal qui pinge la passerelle et
recharge le driver après 3 échecs, reboot après 10) n'ont été que des
pansements — ils ne figurent volontairement **pas** dans ce dépôt.

> Leçon : trois causes empilées sur un même symptôme. Chaque correctif
> « améliorait » sans résoudre, ce qui est le scénario le plus démoralisant qui
> soit. C'est le changement de **support physique** qui a tranché.

---

## 4. Le CPL qui refuse de s'appairer

**Nature** : matériel réseau.

**Symptôme** : deux adaptateurs CPL de la maison ne se voient pas, quoi qu'on
fasse.

**Cause réelle** : **deux normes incompatibles** cohabitaient — du devolo Magic
(**G.hn**) et du TP-Link (**HomePlug AV2**). Ces deux familles ne s'appairent
jamais entre elles, par conception. Aucun réglage n'y change quoi que ce soit.

**Résolution — action physique** : utiliser deux adaptateurs de la **même**
famille pour le lien du Minitel.

---

## 5. « Le Pi ne boote pas » — alors que si

**Nature** : matériel, et pure perte de temps.

**Symptôme** : après réinstallation, hôte pingable mais SSH en *connection
refused*. Puis, en cherchant un témoin visuel : **les LED du port RJ45 restent
éteintes**.

**Deux pièges d'un coup** :

1. **SSH n'était pas coché dans le Raspberry Pi Imager.** Correctif sans
   reflasher : créer un fichier vide nommé `ssh` à la racine de la partition FAT
   `bootfs` de la carte.
2. **Sur un Pi 3B, les LED du RJ45 ne s'allument pas si la machine ne boote
   pas.** L'Ethernet est une puce LAN9514 sur le bus USB, sortie de reset par le
   firmware. Donc « LED réseau éteintes » **ne discrimine rien** entre « pas de
   lien réseau » et « pas de boot ». Le seul témoin fiable est la **LED verte
   ACT**.

> Leçon : un indicateur qu'on croit lire n'est un indicateur que si on connaît son
> circuit. Celui-ci a envoyé chercher une panne réseau devant une machine
> simplement éteinte.

---

## 6. L'IP qui change quand on change de câble

**Nature** : réseau.

**Symptôme** : après passage du Wi-Fi à l'Ethernet, la machine est introuvable à
son adresse habituelle.

**Cause réelle** : `wlan0` et `eth0` ont des **MAC différentes**, donc des baux
DHCP différents. Changer de support change l'adresse.

**Résolution** : chercher le **nouveau** bail, pas l'ancienne IP. Et retenir que
plusieurs fichiers de configuration ailleurs sur le réseau référençaient l'IP en
dur — tous à repointer. C'est précisément pour cette raison que ce dépôt ne
contient **aucune** adresse : tout passe par l'environnement.

---

## 7. La racine en lecture seule qui avale les déploiements

**Nature** : déploiement.

**Symptôme** : on déploie, ça marche, on redémarre le Pi — **tout a disparu**.
Sans le moindre message d'erreur.

**Cause réelle** : l'**overlayfs** de `raspi-config`, activé pour épargner la
carte SD des coupures de courant. Toutes les écritures vont en RAM.

**Résolution logicielle** : `deploy.sh` **détecte le cas et refuse de
continuer**, plutôt que de réussir en apparence. Lever l'overlay, déployer, le
remettre :

```bash
sudo raspi-config nonint disable_overlayfs && sudo reboot
```

> Une protection qu'on a activée soi-même des semaines plus tôt est le pire
> saboteur : on ne la soupçonne pas.

---

## 8. Le Minitel qui oublie sa vitesse

**Nature** : protocole. La première vraie difficulté logicielle.

**Symptôme** : après avoir éteint puis rallumé le Minitel, plus rien ne
s'affiche — alors que le port série reste ouvert côté Pi, sans erreur.

**Cause réelle** : le 1B **n'a pas d'EEPROM**. Il redémarre systématiquement à
1200 bauds, tandis que le Pi continue d'émettre à 4800. Aucun `EOF`, aucune
exception : juste deux machines qui ne parlent plus la même langue.

**Résolution — code** : un **watchdog** qui sonde l'identification du terminal
toutes les 4 s et, s'il reste muet plus de 10 s, referme le port et renégocie.
L'accueil revient seul ~10 s après le rallumage. La touche Connexion/Fin force un
reset manuel.

**Sous-piège, celui-là vicieux** : la sonde de négociation acceptait d'abord
« n'importe quel octet reçu » comme preuve que 4800 fonctionnait. Or **du bruit de
framing à la mauvaise vitesse produit des octets**. Faux positif systématique. La
sonde exige désormais une trame d'identification complète `SOH…EOT`. La
négociation a fini par demander **3 tentatives, 500 ms de délai de commutation et
1 200 ms de fenêtre de sonde** pour être fiable.

> Leçon : « j'ai reçu quelque chose » n'est pas « ça marche ». Sur une liaison
> série, le bruit est indiscernable des données si on ne valide pas la structure.

---

## 9. La mosaïque qui affiche du charabia

**Nature** : protocole. Le piège le plus coûteux en aller-retours.

**Symptôme** : au lieu d'une image, une bouillie de lettres et de chiffres.

**Cause réelle** : un positionnement de curseur (`1F L C`, comme `CR` et `LF`)
**réinitialise le jeu de caractères en G0**. Envoyer le basculement en mosaïque
(`0E`) *avant* de se positionner revient donc à ne pas basculer du tout : les
octets graphiques sont interprétés comme du texte.

**Résolution — code** : par ligne de mosaïque, `1F L C` **puis** `0E`, puis les
octets. Jamais l'inverse. C'est noté dans le cheat-sheet et dans `AGENTS.md`.

---

## 10. Le logo illisible — et le coût du cycle de vérification

**Nature** : rendu.

**Symptôme** : la bannière de titre, transposée depuis l'art ASCII d'un splash de
terminal (`██╗`), sort en **pâté illisible** qui débordait sur le champ de
saisie.

**Cause réelle** : dans un terminal, les caractères de biseau (`╗ ╚ ═ ╔ ╝`) sont
de la même couleur que les pleins et se distinguent par leur forme. À l'échelle
du Minitel — 80×72 sous-pixels — ils **s'allument comme des blocs entiers** et
soudent les lettres entre elles.

**Résolution — code** : abandon de la transposition, écriture d'une vraie petite
**fonte bitmap 5×5** agrandie (le facteur d'agrandissement fait l'épaisseur du
trait).

**Le vrai enseignement est ailleurs.** Chaque essai de rendu coûtait :
recompiler → déployer sur le Pi → regarder l'écran → **photographier** l'écran
pour pouvoir en discuter. Ce cycle est si lent qu'il a justifié d'écrire un outil
dédié, `tools/vtx-preview.py`, qui décode un flux Vidéotex en ASCII dans le
terminal. **Itérer sans matériel est devenu plus rentable que d'itérer vite.**

---

## 11. Les petites vérités qu'on n'apprend qu'en regardant l'écran

Aucune ne produit d'erreur. Toutes produisent un affichage faux.

- **Les majuscules accentuées ne s'affichent pas.** Un accent est un glyphe
  superposé par un OU binaire sur les rangées hautes de la cellule ; sur une
  capitale, ces pixels sont déjà allumés et l'accent disparaît. `É` sort `E`. La
  cédille, elle, occupe les rangées basses : `Ç` fonctionne. Le code sait les
  encoder — le verre, non.
- **Une séquence d'attribut occupe une case écran.** Une chaîne colorée de 40
  caractères déborde donc sur la ligne suivante. Le pied de page du daemon fait
  exactement 38 caractères pour cette raison, et un test le verrouille.
- **Un caractère double largeur consomme deux colonnes** : le flux ne doit pas
  contenir la case masquée. La double hauteur écrase la ligne du dessus.
- **L'ordre des gris n'est pas intuitif** sur écran monochrome : noir 0, bleu
  40 %, rouge 50, magenta 60, vert 70, cyan 80, jaune 90, blanc 100.
- **Le bit 5 du motif mosaïque vaut `0x40`**, pas `0x20`.
- **La barre de statut est cadrée sur 37 colonnes**, pas 40 : au-delà,
  l'indicateur matériel du terminal apparaît dans le champ.
- **Une ligne de message qui chevauche une autre ligne** ne se voit pas en
  relecture de code. Un indice d'aide écrit en ligne 18 et les messages d'erreur
  écrits *aussi* en ligne 18 : l'erreur s'affichait par-dessus en laissant
  dépasser les deux bouts de l'indice. Trouvé sur l'écran, pas dans le source.

---

## 12. Les pièges d'outillage, pour mémoire

Sans rapport avec le Minitel, mais chacun a coûté sa demi-heure :

- `systemctl enable --now` **ne redémarre pas** un service déjà actif. Un
  déploiement semblait donc réussir sans que le nouveau binaire tourne. Il faut
  `restart`.
- Copier un binaire par-dessus lui-même pendant qu'il tourne → `ETXTBSY`. Arrêter
  le service **avant** le transfert.
- La première bibliothèque série retenue tirait **libudev**, une dépendance C qui
  rendait la cross-compilation pénible. Bascule vers une bibliothèque **en Rust
  pur** : plus de dépendance C, binaire statique, lien par `rust-lld`, ni Docker
  ni toolchain croisée.
- Rediriger les logs vers un fichier les rendait **bufferisés** donc invisibles au
  moment où on en avait besoin. Passage à journald.

---

## Ce que ça dit du travail

Sur onze pannes majeures, **six étaient physiques ou électriques** (câble,
carte SD, position dans la maison, norme de CPL, support réseau, alimentation),
une était un piège de configuration, et quatre seulement relevaient du
protocole ou du code.

Les pannes physiques ont demandé des gestes physiques : changer un câble, flasher
une carte, tirer un lien CPL, déplacer un point d'accès, câbler un connecteur. Un
diagnostic peut suggérer où regarder ; il ne remplace pas la main qui débranche.

Les pannes de protocole, elles, avaient un point commun : **elles ne produisaient
aucune erreur**. Pas d'exception, pas de log, pas de test rouge — juste un écran
qui affiche autre chose que ce qu'on croyait. La seule boucle de vérification qui
valait quelque chose passait par l'œil, devant le verre. D'où le réflexe qui
structure ce dépôt : les invariants d'affichage sont **couverts par des tests**,
pour que ce qui a été payé une fois ne se repaye pas.
