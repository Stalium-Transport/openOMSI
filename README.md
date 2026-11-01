# Stalium Transport — version modifiée d'openOMSI

Ce dépôt sert **uniquement aux téléchargements** de la version Stalium Transport d'openOMSI :
les versions sont dans **[Releases](https://github.com/Stalium-Transport/openOMSI/releases)**.
Le lanceur des joueurs y cherche aussi ses mises à jour automatiques.

Projet **indépendant**, basé sur [openOMSI](https://github.com/openOMSI-org/openOMSI) (licence
AGPL-3.0). Ce n'est **pas** une version officielle d'openOMSI. Le code affiché dans ce dépôt est
celui d'openOMSI officiel ; le code de la version Stalium est fourni **sur demande** à ses
utilisateurs, comme le prévoit la licence AGPL-3.0 (contact : Stalium Transport,
[stalium-transport.fr](https://stalium-transport.fr)).

Base actuelle : **openOMSI 0.3.45**, protocole réseau 15.

---

## Installer

- **Joueurs** : télécharger `Stalium-openOMSI-<version>-windows-x64.zip` dans la dernière
  release, le décompresser, puis lancer le lanceur. Pour BCS, choisir ce dossier. Une version
  Stalium déjà installée se met à jour toute seule depuis le lanceur.
- **Serveur** : `Stalium-openOMSI-<version>-server-windows-x64.zip`. En mettant à jour, garder
  son `server.cfg` et son `bcs.txt`.
- Le serveur et les joueurs doivent avoir **la même version**.

---

## Toutes les modifications de la version Stalium

### Lanceur

- **Aux couleurs de Stalium Transport** : logo et nom Stalium Transport, bleus du logo à la place
  de l'orange, fonds bleu marine (thème sombre et thème clair). L'icône du lanceur et du jeu est
  celle de Stalium. La mention « basé sur openOMSI (AGPL-3.0) » apparaît au survol du logo.
- **Mises à jour Stalium** : le lanceur propose et installe les versions Stalium publiées ici,
  jamais la version officielle d'openOMSI (elle ne rejoindrait plus les serveurs Stalium).
- **Heure et date du serveur** : en rejoignant un serveur (depuis la liste, par code ou par
  adresse), le lanceur prend l'heure et la date du serveur, l'heure avance toute seule, et les
  départs proposés sont ceux de l'heure du serveur. Choisir un trajet ne change plus l'heure
  affichée.

### HUD Stalium Transport dans le jeu

Le HUD de l'ancien plugin Stalium fait partie du jeu : plus besoin de plugin ni de Stalium
Companion.

- **Bandeau compact** en haut de l'écran, sur deux lignes : la ligne et son terminus, l'heure et
  la **date du jeu**, la vitesse (en rouge au-dessus de la limitation) et la limitation, le
  prochain arrêt et sa distance, l'avance ou le retard en clair (les secondes à moins d'une
  minute) ; dessous le chauffeur, le bus, les températures extérieure et intérieure, la feuille
  en cours, les passagers et le carburant. La carte n'affiche plus ce que le bandeau montre déjà.
- **Tablette conducteur** (F11) : la date complète et l'heure, ligne et service, prochain arrêt,
  avance ou retard, terminus, passagers, caisse, et la feuille de route de la course. Elle se
  déplace à la souris et se redimensionne par son coin ; sa place et sa taille sont gardées.
- **Touches** modifiables dans Options → Commandes (« Stalium : tablette conducteur »,
  « Stalium : afficher / masquer le HUD », F7 par défaut). Le HUD se désactive dans Options →
  Gameplay.

### Feuille de route automatique

- La feuille de route part **toute seule** sur stalium-transport.fr **dès l'arrivée au terminus**
  de chaque course, en avance comme en retard.
- Seulement sur un **serveur Stalium** (rien en solo, sur une partie hébergée ou un autre
  serveur), et seulement pour une course **allée jusqu'à son terminus** (une course abandonnée
  n'envoie rien).
- Au nom du **compte Stalium relié** : on copie sa clé sur son profil du site, puis tablette (F11)
  → « Coller ma clé ». Sans compte relié, la feuille n'est pas envoyée et le jeu le dit.
- La feuille reprend le **résumé de la course** affiché par le jeu : heure de départ du premier
  arrêt et d'arrivée au terminus, kilomètres de la course, voyageurs, notes de conduite, de
  confort et de vente de billets, et note sur 5. Le jeu affiche la réponse du site et le gain.

### BCS

- Un trajet lancé depuis **BCS** sur une carte Stalium **rejoint tout seul le serveur Stalium**
  de cette carte (une carte sans serveur se joue en solo). Rien à régler dans BCS : on choisit
  juste le dossier de cette version.
- Le joueur porte le **pseudo de son profil openOMSI**, et non plus le nom commun « Bus Company
  Simulator ».
- Les adresses des serveurs se changent côté serveur, sans nouvelle version. Les trajets BCS
  peuvent être réservés au personnel des entreprises choisies.

### Trafic et bus de ligne

- **Bus bloqués aux feux** (Saint-Servan, CHU Pasteur - Cornouaille) : là où le feu qui arrête un
  bus de ligne est à quelques mètres d'un autre feu du même carrefour (le feu de bus, puis celui
  de la voie qu'il rejoint), le bus repart dès qu'il est prêt. Ailleurs, un bus qui attend à un
  feu rouge plus longtemps que son cycle complet (1 à 3 minutes) le considère comme en panne et
  passe.
- **Bus en avance** : sur un serveur, un bus en avance n'attend son heure qu'aux arrêts horaires
  du service, et 2 minutes au plus (réglable dans `server.cfg` : `ai_wait_timed_stops_only`,
  `ai_stop_wait_max`). Les bus ne restent plus de longues minutes à un arrêt en bloquant la voie.
- **Bus de service** : un bus attend là où il a déposé ses voyageurs jusqu'à rejoindre son premier
  arrêt, et un bus en fin de trajet garde son temps d'attente à l'arrêt (correction reprise
  d'openOMSI 0.3.46).
- **Demande d'arrêt** : quand un passager IA veut descendre, la demande part bien vers le bus de
  ligne dans lequel il se trouve (correction reprise d'openOMSI 0.3.46).
- La console du serveur signale tout bus de ligne immobile depuis 2 minutes, avec ce qui le
  retient.

### Multijoueur

- **Girouettes identiques pour tous** : un même bus IA affiche la même ligne chez tous les joueurs
  (il prend le fichier de dépôt de la carte, comme le serveur, et non celui que le joueur a
  choisi pour son propre bus).
- **Passager dans le bus d'un autre joueur** : les passagers, le trafic et les gens aux arrêts ne
  disparaissent plus quand ce bus s'éloigne de son propre bus garé ; le passager peut utiliser
  les boutons passagers (demande d'arrêt, porte).

---

Version Stalium Transport d'openOMSI — projet indépendant, non affilié à openOMSI.
openOMSI est sous licence [AGPL-3.0](LICENSE) ; le fichier de licence et les mentions de
copyright d'openOMSI sont conservés.
