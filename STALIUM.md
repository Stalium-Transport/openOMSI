# Stalium Transport - version modifiée d'openOMSI

Projet **indépendant**, basé sur [openOMSI](https://github.com/openOMSI-org/openOMSI)
(licence AGPL-3.0). Ce n'est pas une version officielle d'openOMSI.

Base : openOMSI 0.3.39 (protocole réseau 14, compatible avec openOMSI 0.3.37).

## Modifications

- **Serveur réservé aux membres** : un fichier `membres.txt` à côté de `server.cfg` liste les
  noms autorisés (un par ligne, `#` = commentaire, `message: …` = texte affiché aux joueurs
  refusés). Relu automatiquement quand il change. Sans fichier, le serveur est ouvert à tous.
  Code : `crates/omsi-lan/src/members.rs`.

## Compilation

GitHub Actions, workflow `Stalium` (`.github/workflows/stalium.yml`) : chaque envoi sur la
branche `stalium` compile le serveur Windows et publie le zip dans les Releases
(`stalium-0.3.39.<numéro>`).
