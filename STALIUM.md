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
  (fiche `bbs\bbs.odr`), le jeu demande au serveur Stalium où aller pour cette carte, avec
  le nom du profil conducteur choisi dans le lanceur openOMSI (la fiche BCS porte le même nom
  « Bus Company Simulator » pour tous). Le serveur répond avec son fichier `bcs.txt` (une
  carte par ligne : `maps/st-servan/global.cfg = 27017`) et `membres.txt` : un membre sur une
  carte de la liste rejoint le serveur, sinon le trajet se fait en solo. Rien à régler dans
  BCS (on choisit juste le dossier de cette version), et pas de nouvelle version quand on
  change une carte, un port ou un membre. Seule l'adresse du serveur qui répond est dans le
  programme (`scripts/stalium/servers.py`).
  Code : `crates/omsi-app/src/stalium_bcs.rs`, `crates/omsi-lan/src/bcs.rs`.
- **Pas de mise à jour automatique vers la version officielle** (elle ne rejoindrait plus
  les serveurs Stalium).

## Compilation

GitHub Actions, workflow `Stalium` (`.github/workflows/stalium.yml`) : chaque envoi sur la
branche `stalium` compile le jeu et le serveur Windows et publie les zips dans les Releases
(`stalium-0.3.39.<numéro>`).
