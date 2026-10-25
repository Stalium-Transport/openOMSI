<!-- Ce qui change dans la prochaine version Stalium : ce texte devient la description de la release sur GitHub. À modifier avant chaque Actions -> Stalium -> Run workflow ; le champ « changements » de Run workflow le remplace s'il est rempli. -->
**Bus de ligne bloqués aux arrêts**
- Un bus de ligne en avance n'attend plus son heure de départ à chaque arrêt pendant de longues minutes (jusqu'à 30 minutes, et sans limite au dernier arrêt) : il bloquait la voie derrière lui et provoquait des bouchons.
- Sur un serveur Stalium, un bus en avance n'attend son heure qu'aux arrêts où l'horaire en donne une, et au plus 2 minutes. Réglable dans `server.cfg` : `ai_wait_timed_stops_only = 1` et `ai_stop_wait_max = 120` (`0` = comme OMSI). Le premier arrêt d'une course ne change pas : un bus n'en part jamais avant son heure.
- La console du serveur signale un bus qui reste plus de 5 minutes à un arrêt.

**Passagers dans le bus d'un autre joueur**
- Un joueur à pied dans le bus d'un autre peut de nouveau appuyer sur les boutons des passagers : demande d'arrêt, poussette, demande de porte (`door_2_wunsch`…), demande de rampe, sonnette. Le conducteur reçoit l'appui dans son bus.
- Les boutons du conducteur (portes, moteur, billettique, SAE, cockpit) restent réservés au conducteur, ainsi que le bouton qui coupe les demandes d'arrêt.

**Rappel** : le serveur et les joueurs doivent avoir la même version (protocole réseau 15).
