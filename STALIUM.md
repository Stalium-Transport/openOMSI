# Stalium Transport - version modifiée d'openOMSI

Projet **indépendant**, basé sur [openOMSI](https://github.com/openOMSI-org/openOMSI)
(licence AGPL-3.0). Ce n'est pas une version officielle d'openOMSI.

Base : openOMSI 0.3.39 (protocole réseau 14, compatible avec openOMSI 0.3.37).

## Modifications

- **Serveur réservé aux membres** : un fichier `membres.txt` à côté de `server.cfg` liste les
  noms autorisés (un par ligne, `#` = commentaire, `message: …` = texte affiché aux joueurs
  refusés). Relu automatiquement quand il change. Sans fichier, le serveur est ouvert à tous.
  Code : `crates/omsi-lan/src/members.rs`.
- **Trajets BCS sur le serveur Stalium** : quand BCS (Busbetrieb-Simulator) lance le jeu
  (fiche `bbs\bbs.odr`) pour un trajet sur une carte Stalium, le jeu rejoint le serveur
  Stalium tout seul, avec le nom du conducteur. Il n'y a rien à régler dans BCS : on choisit
  juste le dossier de cette version. Les autres cartes se jouent en solo, comme avant. La
  liste des serveurs est dans le programme, pas dans un fichier (`scripts/stalium/servers.py`).
  Code : `crates/omsi-app/src/stalium_bcs.rs`.
- **Pas de mise à jour automatique vers la version officielle** (elle ne rejoindrait plus
  les serveurs Stalium).

## Compilation

GitHub Actions, workflow `Stalium` (`.github/workflows/stalium.yml`) : chaque envoi sur la
branche `stalium` compile le jeu et le serveur Windows et publie les zips dans les Releases
(`stalium-0.3.39.<numéro>`).
