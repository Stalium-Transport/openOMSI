# Stalium Transport - version modifiée d'openOMSI

Projet **indépendant**, basé sur [openOMSI](https://github.com/openOMSI-org/openOMSI)
(licence AGPL-3.0). Ce n'est pas une version officielle d'openOMSI.

Base : openOMSI 0.3.39 (protocole réseau 14, compatible avec openOMSI 0.3.37).

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
- **Pas de mise à jour automatique vers la version officielle** (elle ne rejoindrait plus
  les serveurs Stalium).

## Compilation

GitHub Actions, workflow `Stalium` (`.github/workflows/stalium.yml`) : chaque envoi sur la
branche `stalium` compile le jeu et le serveur Windows et publie les zips dans les Releases
(`stalium-0.3.39.<numéro>`).
