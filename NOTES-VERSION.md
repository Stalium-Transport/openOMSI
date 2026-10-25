<!-- Ce qui change dans la prochaine version Stalium : ce texte devient la description de la release sur GitHub. À modifier avant chaque Actions -> Stalium -> Run workflow ; le champ « changements » de Run workflow le remplace s'il est rempli. -->
**Nouvelle base : openOMSI 0.3.41** (protocole réseau 15) — le serveur et tous les joueurs doivent passer à cette version.

**Synchronisation entre joueurs**
- Le trafic et les feux sont décidés par le serveur et envoyés à tous les joueurs : tout le monde voit les mêmes feux au même moment.
- Les feux de bus qui ne passaient jamais au vert sont corrigés.
- La synchronisation des passagers et des personnes à pied est refaite.

**Compatibilité**
- Les joueurs avec l'openOMSI officiel 0.3.40 ou plus récent peuvent rejoindre les serveurs Stalium ; les versions 0.3.39 et plus anciennes ne le peuvent plus.

**BCS**
- Rien ne change : les trajets BCS rejoignent le serveur Stalium tout seuls, avec le pseudo BCS comme nom de profil.
