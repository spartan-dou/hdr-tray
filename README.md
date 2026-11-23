# HDR Tray

Petite icône pour la zone de notification de Windows 11 qui montre si le HDR est activé et le bascule d'un clic.

- **Pastille « HDR » noir et blanc** (suit le thème de la barre des tâches) : HDR activé. **Pastille atténuée et barrée** : désactivé.
- **Clic gauche** : active ou désactive le HDR sur tous les écrans compatibles.
- **Clic droit** : menu (basculer, écrans, lancement au démarrage, HDR automatique, mises à jour, quitter).
- **Bascule par écran** : avec plusieurs écrans HDR, le sous-menu « Écrans » (une coche par écran) bascule chacun séparément.
- **HDR automatique par application** : menu « HDR automatique par application » → « Modifier la liste » ouvre `%APPDATA%\HDRTray\apps.txt`
  (un nom d'exe par ligne, `#` pour commenter). Le HDR s'active tant qu'une de ces applications tourne, puis revient à son état initial.
- **Mises à jour maîtrisées** : menu « Mises à jour » → installer automatiquement (défaut), prévenir seulement (bulle de notification puis
  « Installer la version X »), ou désactivées. Une bulle confirme la mise à jour au redémarrage. L'installeur est vérifié par SHA-256 (`SHA256SUMS`).
  La version portable ne s'installe jamais toute seule.
- **Français / anglais** : suit la langue de Windows (application et installeur).
- **Icône adaptée au thème** : blanche sur une barre des tâches sombre, noire sur une barre claire.
- Suit aussi les changements faits ailleurs (`Win + Alt + B`, Paramètres, jeux).
- Écrit en Rust, sans dépendance : un exe de quelques centaines de Ko, 0 % de CPU, environ 10 Mo de RAM.

## Installation

Télécharger l'installeur depuis la [dernière release](https://github.com/spartan-dou/hdr-tray/releases/latest) et le lancer :

- **`HDRTray-Setup.exe`** : PC Intel / AMD (x64)
- **`HDRTray-Setup-arm64.exe`** : PC ARM64 (Snapdragon…)

Les `HDRTray-portable-x64.zip` / `HDRTray-portable-arm64.zip` (ou les `HDRTray.exe` / `HDRTray-arm64.exe` seuls) sont les versions portables (sans installation). `SHA256SUMS` permet de vérifier un téléchargement, et chaque fichier a une attestation de provenance (`gh attestation verify`).
Pas besoin des droits administrateur. Installation dans `%LOCALAPPDATA%\Programs\HDRTray`, désinstallation depuis « Applications installées ».

L'exe n'est pas signé : Windows SmartScreen peut afficher un avertissement (« Informations complémentaires » → « Exécuter quand même »).

Windows 11 masque les nouvelles icônes derrière la flèche `^` : la glisser dans la barre, ou l'activer dans
Paramètres → Personnalisation → Barre des tâches → Autres icônes de la barre d'état.

## Compilation

Tout se compile dans Docker (Rust + MinGW pour x64, llvm-mingw pour ARM64, NSIS), rien à installer sur Windows :

```sh
docker build -t hdrtray-build .
docker run --rm -v "$PWD":/src hdrtray-build
```

Résultat dans `dist/` : les exe et installeurs x64 et ARM64, et `version.txt`.

## Publier une version

1. Changer `version` dans `Cargo.toml`.
2. `git commit -am "vX.Y.Z" && git tag -a vX.Y.Z -m vX.Y.Z && git push --follow-tags`

La GitHub Action compile et publie la release ; les appli installées se mettent à jour toutes seules.

## Fonctionnement

L'état du HDR est lu et modifié via l'API `DisplayConfig` de Windows
(`DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO_2` / `SET_HDR_STATE` sur Windows 11 24H2+, avec repli sur les anciennes API).
