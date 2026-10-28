<!-- Ce qui change dans la prochaine version Stalium : ce texte devient la description de la release sur GitHub. À modifier avant chaque Actions -> Stalium -> Run workflow ; le champ « changements » de Run workflow le remplace s'il est rempli. -->
**Mise à jour d'openOMSI 0.3.45**
- La version Stalium passe d'openOMSI 0.3.41 à 0.3.45, avec toutes les nouveautés et corrections d'openOMSI entre les deux. Le protocole réseau ne change pas (15).

**HUD Stalium Transport intégré au jeu**
- Le HUD du plugin Stalium fait maintenant partie du jeu : plus besoin de copier le plugin dans le dossier du jeu. Si vous l'aviez installé, supprimez `main.lua` et `openomsi-plugin.toml` du dossier `plugins` pour ne pas avoir deux HUD, et n'utilisez plus Stalium Companion : sa mise à jour automatique remettrait le plugin dans le jeu.
- Bandeau compact en haut de l'écran, à la place de la barre d'informations, sur deux lignes : la ligne et son terminus, l'heure, la vitesse (en rouge au-dessus de la limitation) et la limitation, le prochain arrêt et sa distance, l'avance ou le retard en clair ; dessous le chauffeur, le bus, la feuille en cours (km et voyageurs), les passagers et le carburant avec leurs jauges.
- Tablette conducteur (F11) : ligne, prochain arrêt, retard, terminus, passagers, caisse.
- Feuille de route (F10, à consulter) : chauffeur, ligne et service, début de la course, distance, voyageurs, billetterie, ponctualité, véhicule.
- À la fin de chaque course du service (terminus desservi), et seulement en jouant sur un serveur Stalium (pas en solo, ni sur une partie hébergée ou un autre serveur), la feuille de route part toute seule sur stalium-transport.fr, au nom du pseudo multijoueur du joueur, sans rien valider ; le jeu affiche la réponse du site et le gain. Elle part aussi au bot Discord du PC. Une nouvelle feuille commence pour la course suivante. Stalium Companion n'est plus nécessaire.
- F7 masque ou réaffiche le HUD. Les touches se changent dans Options → Commandes (si F11, F10 ou F7 sert déjà dans votre keyboard.cfg, le jeu prend F8, F9 ou F6). Le HUD se désactive dans Options → Gameplay (« HUD Stalium Transport »).

**Rappel** : le serveur et les joueurs doivent avoir la même version.
