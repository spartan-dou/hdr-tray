# HDR Tray

Petite icône pour la zone de notification de Windows 11 qui montre si le HDR est activé et le bascule d'un clic.

- **Pastille orange « HDR »** : HDR activé. **Contour gris** : désactivé.
- **Clic gauche** : active ou désactive le HDR sur tous les écrans compatibles.
- **Clic droit** : menu (basculer, lancement au démarrage, mise à jour, quitter).
- Suit aussi les changements faits ailleurs (`Win + Alt + B`, Paramètres, jeux).
- Mise à jour automatique depuis les releases GitHub (au démarrage puis toutes les 6 h).
- Écrit en Rust, sans dépendance : un exe de quelques centaines de Ko, 0 % de CPU, environ 10 Mo de RAM.

## Installation

Télécharger **`HDRTray-Setup.exe`** depuis la [dernière release](https://github.com/spartan-dou/hdr-tray/releases/latest) et le lancer.
Pas besoin des droits administrateur. Installation dans `%LOCALAPPDATA%\Programs\HDRTray`, désinstallation depuis « Applications installées ».

L'exe n'est pas signé : Windows SmartScreen peut afficher un avertissement (« Informations complémentaires » → « Exécuter quand même »).

Windows 11 masque les nouvelles icônes derrière la flèche `^` : la glisser dans la barre, ou l'activer dans
Paramètres → Personnalisation → Barre des tâches → Autres icônes de la barre d'état.

## Compilation

Tout se compile dans Docker (Rust + MinGW + NSIS), rien à installer sur Windows :

```sh
docker build -t hdrtray-build .
docker run --rm -v "$PWD":/src hdrtray-build
```

Résultat dans `dist/` : `HDRTray.exe`, `HDRTray-Setup.exe` et `version.txt`.

## Publier une version

1. Changer `version` dans `Cargo.toml`.
2. `git commit -am "vX.Y.Z" && git tag vX.Y.Z && git push --follow-tags`

La GitHub Action compile et publie la release ; les appli installées se mettent à jour toutes seules.

## Fonctionnement

L'état du HDR est lu et modifié via l'API `DisplayConfig` de Windows
(`DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO_2` / `SET_HDR_STATE` sur Windows 11 24H2+, avec repli sur les anciennes API).
