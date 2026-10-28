<!-- Ce qui change dans la prochaine version Stalium : ce texte devient la description de la release sur GitHub. À modifier avant chaque Actions -> Stalium -> Run workflow ; le champ « changements » de Run workflow le remplace s'il est rempli. -->
**Mise à jour d'openOMSI 0.3.45**
- La version Stalium passe d'openOMSI 0.3.41 à 0.3.45, avec toutes les nouveautés et corrections d'openOMSI entre les deux. Le protocole réseau ne change pas (15).

**HUD Stalium Transport intégré au jeu**
- Le HUD du plugin Stalium fait maintenant partie du jeu : plus besoin de copier le plugin dans le dossier du jeu. Si vous l'aviez installé, supprimez-le du dossier `plugins` pour ne pas avoir deux HUD.
- Bandeau en haut de l'écran, à la place de la barre d'informations : chauffeur, carte, bus, heure, vitesse, kilomètres, odomètre, températures extérieure et intérieure, réservoir, passagers, ligne et terminus, prochain arrêt, retard.
- Tablette conducteur (F11) : ligne, prochain arrêt, retard, terminus, passagers, caisse.
- Feuille de route (F10) : chauffeur, ligne et service, prise de service, distance, voyageurs, billetterie, ponctualité, véhicule. « Valider et envoyer » l'envoie à Stalium Companion et au bot Discord du PC, puis une nouvelle feuille commence.
- F7 masque ou réaffiche le HUD. Les touches se changent dans Options → Commandes (si F11, F10 ou F7 sert déjà dans votre keyboard.cfg, le jeu prend F8, F9 ou F6). Le HUD se désactive dans Options → Gameplay (« HUD Stalium Transport »).

**Rappel** : le serveur et les joueurs doivent avoir la même version.
