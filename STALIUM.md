# Stalium Transport - version modifiée d'openOMSI

Projet **indépendant**, basé sur [openOMSI](https://github.com/openOMSI-org/openOMSI)
(licence AGPL-3.0). Ce n'est pas une version officielle d'openOMSI.

Base : openOMSI 0.3.41 (protocole réseau 15, compatible avec openOMSI 0.3.40 et 0.3.41 ; plus avec
les versions 0.3.39 et plus anciennes).

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
- **Échange de bus entre deux joueurs** : menu du jeu → Liste des joueurs → un joueur →
  « Échanger nos bus ». L'autre accepte ou refuse dans sa liste des joueurs (30 s). Accepté,
  chaque jeu prend le bus de l'autre là où il est, dans l'état où l'autre l'avait (fichier,
  peinture, toutes les variables et textes du script : IBIS, girouette…), avec ses passagers
  à leur place ; le service (ligne, course) reste à chaque joueur. Seulement entre deux bus à
  l'arrêt, à moins de 200 m l'un de l'autre (celui qui demande, au volant, à pied ou dans un autre
  bus, à moins de 50 m du bus de l'autre), hors trajet BCS, et quand le bus de l'autre est
  installé chez soi et proposé par le serveur. Les messages passent par les commandes entre
  joueurs (via le serveur, envoyés 3 fois) : pas de changement de protocole, mais les deux
  joueurs doivent avoir une version qui le connaît.
  Code : `crates/omsi-app/src/stalium_swap.rs` (+ `player_list.rs`, `app_impl/vehicles.rs`,
  `lan_world.rs` `stalium_riders_of`, `omsi-sim` `PeopleSim::seat_handed_riders`).

## Compilation

GitHub Actions, workflow `Stalium` (`.github/workflows/stalium.yml`), lancé à la main
(Actions → Stalium → Run workflow) : il compile le jeu et le serveur Windows et publie les
zips dans les Releases (`stalium-0.3.41.<numéro>`). Un envoi de code ne publie rien.

Version de test : dans Run workflow, choisir la branche à tester et cocher « Version de test ».
Les zips restent dans la page de l'exécution (Artifacts, 14 jours, visibles des membres du
dépôt connectés) : pas de release, donc aucun jeu de joueur ne se met à jour vers elle.

Le texte de la release vient de `NOTES-VERSION.md` (à mettre à jour avant chaque release), ou du
champ « changements » de Run workflow s'il est rempli (changements séparés par `;`).
