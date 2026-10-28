# Stalium Transport - version modifiée d'openOMSI

Projet **indépendant**, basé sur [openOMSI](https://github.com/openOMSI-org/openOMSI)
(licence AGPL-3.0). Ce n'est pas une version officielle d'openOMSI.

Base : openOMSI 0.3.45 (protocole réseau 15, le même qu'en 0.3.40 à 0.3.45 ; plus avec les
versions 0.3.39 et plus anciennes). Serveur et joueurs : de préférence la même version.

## Modifications

- **Trajets BCS sur le serveur Stalium** : quand BCS (Busbetrieb-Simulator) lance le jeu
  (fiche `bbs\bbs.odr`), le jeu demande au serveur Stalium où aller pour cette carte. Le
  serveur répond avec son fichier `bcs.txt`, chiffré avec la clé Windows de la machine
  (DPAPI : illisible sur un autre ordinateur, aucune clé dans le code) : la carte rejoint son
  serveur, une carte absente se joue en solo. Rien à régler dans BCS (on choisit juste le
  dossier de cette version), et pas de nouvelle version quand on change une carte ou un port :
  `openomsi.exe --stalium-bcs bcs.txt` l'ouvre lisible dans le Bloc-notes et le chiffre à
  nouveau à la fermeture (sur un nouveau serveur, on le recrée avec la même commande). L'adresse du serveur qui répond vient du secret GitHub
  `STALIUM_ANNUAIRE` (`adresse:port` de son port web) au moment de la compilation : elle
  n'est jamais écrite dans le code public.
  Avec une ligne `cle: <clé API BCS>` par entreprise dans `bcs.txt`, seuls les employés de
  ces entreprises rejoignent (le nom du profil openOMSI doit être le pseudo BCS ; liste
  `staffjson` de pedepe.de, revue toutes les 5 minutes, les conducteurs IA exclus).
  Code : `crates/omsi-app/src/stalium_bcs.rs`, `crates/omsi-lan/src/bcs.rs`.
- **Mises à jour Stalium dans le lanceur** : le lanceur propose et installe les releases
  `stalium-…` de github.com/Stalium-Transport/openOMSI, comme openOMSI le fait pour les siennes, et jamais la version
  officielle (elle ne rejoindrait plus les serveurs Stalium).
- **HUD Stalium Transport dans le jeu** (il était le plugin Lua `fr.stalium.route` d'AlexLiveFr,
  à copier dans le dossier du jeu) : un bandeau en haut de l'écran à la place de la barre
  d'informations (chauffeur, carte, bus, heure, vitesse, km, odomètre, températures, réservoir,
  passagers, ligne, prochain arrêt, retard), la tablette conducteur (`stalium_tablet`, F11) et la
  feuille de route (`stalium_sheet`, F10, à consulter), envoyée toute seule à la fin de chaque
  course du service (le service passe à la course suivante ou se termine) directement au site
  (`POST https://omsi.stalium-transport.fr/api/omsi_trips.php`, le JSON `SUBMIT_DRIVER_SHEET` du
  plugin avec `saved_at`, comme Stalium Companion le faisait ; réponse `success` / `earned` affichée
  en jeu) et au bot en UDP (127.0.0.1:47800), plus à Companion (le site l'aurait reçue deux fois) ;
  `stalium_hud` (F7) le masque. Touches dans keyboard.cfg (F8, F9, F6 si les premières sont
  prises), réglage `stalium_hud` (Options → Gameplay). Les contrôles de titres, aléas, SOS et
  la télémétrie du plugin n'en font pas partie.
  Code : `crates/omsi-app/src/game_panels/stalium_hud.rs` (les panneaux du jeu, `game_panels`).

## Compilation

GitHub Actions, workflow `Stalium` (`.github/workflows/stalium.yml`), lancé à la main
(Actions → Stalium → Run workflow) : il compile le jeu et le serveur Windows et publie les
zips dans les Releases (`stalium-0.3.45.<numéro>`). Un envoi de code ne publie rien.

Version de test : dans Run workflow, choisir la branche à tester et cocher « Version de test ».
Les zips restent dans la page de l'exécution (Artifacts, 14 jours, visibles des membres du
dépôt connectés) : pas de release, donc aucun jeu de joueur ne se met à jour vers elle.

Le texte de la release vient de `NOTES-VERSION.md` (à mettre à jour avant chaque release), ou du
champ « changements » de Run workflow s'il est rempli (changements séparés par `;`).
