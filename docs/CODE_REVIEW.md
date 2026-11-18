# Revue de code — HDR Tray

Revue effectuée sur la branche `main` au commit `6c21bb5` (version 1.1.1).
Périmètre : l'intégralité du dépôt (`src/*.rs`, `build.rs`, `build.sh`, `installer.nsi`, `Dockerfile`, `.cargo/config.toml`, `.github/workflows/release.yml`, `Cargo.toml`, `Cargo.lock`).

Méthode : lecture statique du code. Rien n'a été compilé ni exécuté sous Windows ; les points qui dépendent d'un comportement de Windows non vérifiable par lecture sont marqués **(hypothèse)**.

Échelle de sévérité : critique / élevée / moyenne / faible.

## Résumé

Le projet est petit (environ 750 lignes de Rust, aucune dépendance), lisible et bien commenté. Les structures FFI relues (`DISPLAYCONFIG_*`, `NOTIFYICONDATAW`, `MSG`, `WNDCLASSW`) correspondent aux définitions Windows, et aucun secret n'est présent dans le dépôt. Aucun constat critique. Les points les plus importants :

1. **Élevée — La mise à jour automatique exécute un installeur sans aucune vérification d'intégrité ni consentement** (`src/update.rs:56-80`). Un binaire non signé est téléchargé puis lancé en silencieux toutes les 6 h : la compromission du compte GitHub, d'un jeton ou du pipeline de release devient une exécution de code sur toutes les installations.
2. **Moyenne — La mise à jour automatique modifie l'installation au-delà du remplacement de l'exe** : elle réactive le lancement au démarrage que l'utilisateur a désactivé (`installer.nsi:64`) et transforme la version portable en version installée (`src/update.rs:76`, `installer.nsi:57-78`).
3. **Moyenne — Chaîne de build non verrouillée alors qu'elle produit les binaires auto-distribués** : `llvm-mingw` téléchargé sans somme de contrôle (`Dockerfile:7-9`), image de base et action référencées par tag mutable (`Dockerfile:1`, `.github/workflows/release.yml:16`), jeton en écriture pour tout le job (`.github/workflows/release.yml:9-10`).
4. **Moyenne — Aucun test et aucune CI hors publication** (`.github/workflows/release.yml:5-7`), alors que tout le code repose sur des déclarations FFI écrites à la main et du `unsafe`.
5. **Moyenne — Vérifications de mise à jour concurrentes non protégées** (`src/update.rs:82-100`) : la vérification manuelle et celle d'arrière-plan partagent le même fichier temporaire et peuvent lancer deux installeurs.

## 1. Performance

L'application est globalement sobre ; les constats ci-dessous sont tous de faible sévérité.

### P1 — Interrogation périodique toutes les 2 s, sans limite de durée — faible

- `src/main.rs:183`, `src/main.rs:396`, `src/main.rs:338-341`, `src/display.rs:124-145`
- Chaque tick appelle `display::state()`, soit `GetDisplayConfigBufferSizes` + `QueryDisplayConfig` + deux allocations de `Vec` (`src/display.rs:131-132`) + un ou deux `DisplayConfigGetDeviceInfo` par écran. Le coût unitaire est faible, mais le processus se réveille 43 200 fois par jour, ce qui nuance le « 0 % de CPU » du README (`README.md:10`) sur portable.
- Recommandation : remplacer `SetTimer` par `SetCoalescableTimer` avec une tolérance large, et allonger l'intervalle (5 à 10 s). **(hypothèse)** Si `WM_DISPLAYCHANGE` (déjà traité en `src/main.rs:338`) ou `WM_SETTINGCHANGE` est bien émis lors d'une bascule HDR faite ailleurs, le timer peut devenir un simple filet de sécurité à 30 s ; à mesurer sous Windows avant de trancher.

### P2 — `Sleep(250)` sur le thread de l'interface — faible

- `src/main.rs:264-268`
- `toggle()` bloque la boucle de messages 250 ms à chaque clic. Un double-clic produit deux `WM_LBUTTONUP` (`src/main.rs:332`), donc deux bascules et 500 ms de blocage, avec un retour à l'état initial.
- Recommandation : après `display::toggle()`, armer un timer à usage unique (`SetTimer` avec un identifiant dédié, 250 ms) qui appelle `refresh(false)`, et ignorer les clics tant qu'il est armé.

### P3 — Installeurs téléchargés jamais supprimés, et retéléchargés si l'installation échoue — faible

- `src/update.rs:70-79`, `src/update.rs:85-88`
- `HDRTray-Setup-<version>.exe` reste dans `%TEMP%` après chaque mise à jour. Si l'installeur silencieux échoue (il ne remonte aucune erreur), la boucle d'arrière-plan retélécharge et relance le même installeur toutes les 6 h, indéfiniment.
- Recommandation : supprimer les anciens `HDRTray-Setup-*.exe` au démarrage, et mémoriser la dernière version tentée pour ne pas la relancer en boucle.

### P4 — Paramètre anti-cache ajouté à chaque téléchargement — faible

- `src/update.rs:41-46`
- **(hypothèse)** `URLDownloadToFileW` passe par le cache WinINet ; l'URL rendue unique par `?t=<millisecondes>` y laisserait une entrée par vérification, installeur compris.
- Recommandation : utiliser WinHTTP (ou `URLDownloadToCacheFileW` suivi de `DeleteUrlCacheEntryW`) avec un en-tête `Cache-Control: no-cache` plutôt qu'une URL unique.

### P5 — Icône régénérée à chaque tick tant que l'ajout échoue — faible

- `src/main.rs:254-262`, `src/main.rs:228-252`, `src/icon.rs:46-62`
- Tant que `ADDED` est faux, chaque tick refait le rendu complet (16 échantillons par pixel, deux tests de rectangle arrondi par échantillon) et recrée les objets GDI. Négligeable à 16-32 px, mais inutile.
- Recommandation : mettre en cache le `HICON` par couple (taille, état) et ne le recréer que si l'un des deux change.

## 2. Sécurité

Aucun secret en dur, aucune dépendance tierce (`Cargo.lock:5-7`), installation par utilisateur sans élévation (`installer.nsi:21`), URL de mise à jour en HTTPS et figée à la compilation (`src/update.rs:10`, `src/update.rs:52-54`), pas de rétrogradation possible (`src/update.rs:67`). Les constats portent sur la chaîne de mise à jour et de build.

### S1 — Installeur de mise à jour exécuté sans vérification d'intégrité ni consentement — élevée

- `src/update.rs:56-80` (téléchargement en `:72`, exécution en `:76`), `src/update.rs:82-90`, `README.md:22`
- Le seul contrôle est le transport HTTPS vers `github.com`. L'installeur n'est pas signé (le README le confirme), aucune empreinte n'est comparée, et il est lancé avec `/S` sans rien demander à l'utilisateur, 60 s après le démarrage puis toutes les 6 h. Il n'existe aucun réglage pour désactiver ce comportement. Quiconque peut publier une release sur le dépôt (compte compromis, jeton volé, étape de build détournée, voir S3) obtient une exécution de code silencieuse sur toutes les machines installées en moins de 6 h.
- Recommandation, par ordre d'efficacité :
  1. Signer `version.txt` + empreinte SHA-256 de l'installeur avec une clé hors ligne (minisign / Ed25519), embarquer la clé publique dans l'exe et refuser tout installeur dont la signature ou l'empreinte ne correspond pas. La clé ne doit pas se trouver dans les secrets GitHub, sinon le gain est nul.
  2. À défaut, signer les binaires en Authenticode et valider la signature et l'éditeur avec `WinVerifyTrust` avant `Command::new`.
  3. Dans tous les cas, proposer un mode « prévenir sans installer » et une option pour couper la mise à jour automatique.

### S2 — Contenu distant utilisé tel quel dans un chemin de fichier — faible

- `src/update.rs:62`, `src/update.rs:70`
- Le contenu de `version.txt` est injecté sans validation dans `HDRTray-Setup-{latest}.exe`. Une valeur contenant `..\` ou un chemin absolu ferait écrire l'installeur hors de `%TEMP%` (`PathBuf::join` remplace la base si l'argument est absolu). L'impact propre est limité, car celui qui contrôle `version.txt` contrôle déjà l'installeur (S1), mais c'est une entrée externe non validée.
- Recommandation : n'accepter que `^\d+(\.\d+){1,3}$` et renvoyer `Check::Failed` sinon.

### S3 — Chaîne de build non épinglée et jeton trop large — moyenne

- `Dockerfile:7-9` : l'archive `llvm-mingw` est téléchargée puis extraite (`curl | tar`) sans vérification d'empreinte. C'est l'éditeur de liens de la version ARM64.
- `Dockerfile:1` : `rust:1.88-slim-bookworm` est un tag mutable, non épinglé par digest. Rust 1.88 date par ailleurs de juin 2025.
- `.github/workflows/release.yml:16` : `actions/checkout@v4` est référencée par tag et non par SHA de commit.
- `.github/workflows/release.yml:9-10` : `contents: write` est accordé à tout le workflow, donc aussi au job qui exécute le build Docker, alors que seule l'étape de publication (`:28-35`) en a besoin.
- Combiné à S1, tout maillon compromis se propage automatiquement aux utilisateurs.
- Recommandation : ajouter un `ARG LLVM_MINGW_SHA256` vérifié par `sha256sum -c` avant extraction ; épingler l'image par `@sha256:` et l'action par SHA ; scinder en un job `build` (`contents: read`, artefact transmis par `upload-artifact`) et un job `publish` (`contents: write`) ; publier un `SHA256SUMS` et une attestation de provenance (`actions/attest-build-provenance`).

### S4 — `taskkill` appelé sans chemin absolu et sur un simple nom d'image — faible

- `installer.nsi:43`
- `taskkill /F /IM HDRTray.exe` tue tout processus de ce nom appartenant à l'utilisateur, pas seulement l'instance installée. De plus, l'exécutable est résolu par l'ordre de recherche de `CreateProcess`. **(hypothèse)** Comme l'installeur de mise à jour s'exécute depuis `%TEMP%` (`src/update.rs:70`), un `taskkill.exe` déposé dans ce dossier serait lancé à sa place ; l'impact reste limité au même utilisateur, sans élévation.
- Recommandation : appeler `"$SYSDIR\taskkill.exe"`, et ne l'utiliser qu'en dernier recours, après avoir attendu la fin du processus (voir D4).

### S5 — Chemin de l'exécutable tronqué en silence dans la clé `Run` — faible

- `src/main.rs:287-290`
- Le tampon est de 1024 unités UTF-16. Si le chemin est plus long, `GetModuleFileNameW` tronque et renvoie la taille du tampon ; ce cas n'est pas testé et une commande tronquée est écrite dans `HKCU\...\Run`. Le code de retour de `RegSetValueExW` est également ignoré. Le passage par `String::from_utf16_lossy` peut en outre altérer un chemin contenant un substitut UTF-16 isolé.
- Recommandation : considérer `len == buf.len()` comme une erreur (ou agrandir le tampon en boucle), construire la valeur directement en UTF-16, et vérifier le code de retour.

### S6 — `std::env::args()` interrompt le processus sur un argument non Unicode — faible

- `src/main.rs:368`
- `args()` panique si un argument n'est pas de l'Unicode valide ; avec `panic = "abort"` (`Cargo.toml:10`) et le sous-système `windows`, l'application s'arrête sans message.
- Recommandation : `std::env::args_os().any(|a| a == "--quit")`.

## 3. Design / architecture

### D1 — La mise à jour silencieuse annule le choix « Lancer au démarrage » — moyenne

- `installer.nsi:64`, `src/main.rs:280-295`, `src/update.rs:76`
- L'installeur écrit la valeur `Run` sans condition. Or l'utilisateur peut la supprimer depuis le menu (`RegDeleteValueW`, `src/main.rs:292`). À chaque mise à jour automatique, l'installeur relancé avec `/S` la recrée : le réglage est perdu sans que l'utilisateur en soit informé.
- Recommandation : en mode silencieux, ne pas toucher à la clé `Run` (`IfSilent` autour de la ligne 64), ou ne l'écrire qu'à la première installation (absence de `UNINST_KEY`).

### D2 — La version portable s'installe toute seule lors d'une mise à jour — moyenne

- `src/update.rs:56-80`, `installer.nsi:57-78`, `README.md:19`
- `check_and_install()` ne vérifie pas d'où tourne l'exe. Lancée en version portable, l'application télécharge l'installeur, qui tue le processus portable (`installer.nsi:43`), installe une copie dans `%LOCALAPPDATA%\Programs\HDRTray`, crée un raccourci, une entrée de désinstallation et une clé `Run`, puis démarre la copie installée. L'exe portable, lui, n'est jamais mis à jour.
- Recommandation : ne lancer l'installeur que si le chemin courant correspond à `InstallLocation` de la clé de désinstallation ; en portable, se limiter à signaler la nouvelle version (ou remplacer l'exe en place).

### D3 — Vérifications de mise à jour concurrentes — moyenne

- `src/update.rs:58`, `src/update.rs:82-100`
- Chaque clic sur « Rechercher une mise à jour » crée un thread (`:93`) sans exclusion mutuelle avec le thread d'arrière-plan (`:83`). Tous écrivent puis suppriment le même `hdrtray-version.txt` (`:58-63`) et le même fichier d'installeur (`:70`) : lecture d'un fichier supprimé par l'autre thread (faux « Impossible de vérifier »), écriture simultanée de l'installeur, ou deux installeurs lancés en parallèle.
- Recommandation : un `AtomicBool` « vérification en cours » pris par `compare_exchange`, ou un unique thread de mise à jour réveillé par un canal ; fichiers temporaires à nom unique.

### D4 — Arrêt de l'instance en cours : attente fixe puis arrêt forcé — faible

- `installer.nsi:38-45`, `src/main.rs:368-374`
- `--quit` poste `WM_CLOSE` et rend la main aussitôt ; l'installeur attend 500 ms puis exécute toujours `taskkill /F`. Si l'application est occupée (par exemple dans le `Sleep` de `toggle()` ou un `QueryDisplayConfig` lent), elle est tuée avant `NIM_DELETE` (`src/main.rs:346-353`) et laisse une icône fantôme.
- Recommandation : faire attendre `--quit` jusqu'à la fin du processus cible (`GetWindowThreadProcessId` + `OpenProcess(SYNCHRONIZE)` + `WaitForSingleObject` avec délai), et ne recourir à `taskkill` qu'en cas d'échec.

### D5 — Aucun test, aucune CI hors release — moyenne

- `.github/workflows/release.yml:5-7` (seul déclencheur : tags `v*`) ; aucun `#[test]` dans `src/`.
- Les erreurs ne sont détectées qu'au moment de publier. Plusieurs fonctions pures sont pourtant testables sans Windows : `parse` (`src/update.rs:48-50`), `icon::render` (`src/icon.rs:36-76`), `ico` (`build.rs:9-50`). Cas limites de `parse` à couvrir : `"1.2"` est considéré inférieur à `"1.2.0"` (comparaison de `Vec` de longueurs différentes), et `"1.2.0-beta"` devient `[1, 2, 0]`, donc égal à la version stable.
- Recommandation : un workflow sur `push`/`pull_request` exécutant `cargo fmt --check`, `cargo clippy --target x86_64-pc-windows-gnu` et `cargo test` ; des assertions de taille à la compilation sur les structures FFI (par exemple `size_of::<NotifyIconData>() == 976`, `size_of::<PathInfo>() == 72` en x64).

### D6 — Déclarations FFI écrites à la main et `unsafe` omniprésent — moyenne

- `src/main.rs:17-154`, `src/display.rs:6-89`, `src/update.rs:14-22`
- Environ 190 lignes de structures, prototypes et constantes recopiés, sans aucune vérification par le compilateur ; presque toutes les fonctions de `main.rs` sont `unsafe fn`. Les définitions relues sont correctes, mais chaque ajout d'API réintroduit le risque (taille, alignement, convention d'appel).
- Recommandation : soit adopter `windows-sys` (déclarations seules, aucun coût à l'exécution, mais abandon du « sans dépendance »), soit conserver l'approche actuelle en isolant le FFI dans un module `ffi` avec des enveloppes sûres et les assertions de taille de D5. C'est un arbitrage à faire par le mainteneur.

### D7 — Codes de retour ignorés, échecs invisibles — moyenne

- `src/display.rs:165` : le résultat de `DisplayConfigSetDeviceInfo` est ignoré ; si la bascule échoue, l'utilisateur ne voit rien.
- `src/main.rs:388`, `src/main.rs:392` : `RegisterClassW` et `CreateWindowExW` non vérifiés ; avec un `hwnd` nul, l'application reste en mémoire sans icône ni moyen de la fermer.
- `src/main.rs:290`, `src/main.rs:292` : écritures registre non vérifiées.
- `src/update.rs:86` : le résultat de la vérification d'arrière-plan est jeté, sans journal.
- `src/main.rs:213-215`, `src/main.rs:230-237` : si `make_icon` renvoie nul, l'icône est tout de même envoyée au shell avec `NIF_ICON`.
- Recommandation : faire renvoyer un `Result` à `display::toggle()` et afficher une bulle de notification en cas d'échec ; quitter proprement si la fenêtre ne peut pas être créée ; journaliser les erreurs de mise à jour (`OutputDebugStringW` ou fichier dans `%LOCALAPPDATA%`).

### D8 — Icône potentiellement figée après un échec de `NIM_MODIFY` — faible **(hypothèse)**

- `src/main.rs:247`, `src/main.rs:254-262`
- `ADDED` reçoit le résultat de `Shell_NotifyIconW`, y compris pour `NIM_MODIFY`. Si une modification échoue alors que l'icône existe toujours (Explorateur momentanément bloqué), les ticks suivants tentent `NIM_ADD`, qui échoue puisque l'icône est déjà enregistrée : l'état affiché ne serait plus mis à jour jusqu'au prochain `TaskbarCreated`.
- Recommandation : après un `NIM_ADD` en échec, tenter `NIM_MODIFY` (ou `NIM_DELETE` puis `NIM_ADD`).

### D9 — État représenté par des entiers magiques et variables globales — faible

- `src/main.rs:189-193`, `src/main.rs:238-242`, `src/main.rs:300-301`, `src/display.rs:147-157`, `src/icon.rs:3-4`
- L'état (-2 non initialisé, -1 aucun écran, 0 inactif, 1 actif) circule en `i32` entre trois modules ; `icon.rs` définit `ON` et `NONE`, mais `main.rs` utilise des littéraux. L'état de l'application est réparti sur cinq variables statiques atomiques.
- Recommandation : un `enum HdrState { Unknown, None, Off, On }` partagé, et une structure d'état unique attachée à la fenêtre (`GWLP_USERDATA`).

### D10 — Duplications et effets de bord du build — faible

- `wide()` est défini deux fois (`src/main.rs:195-197`, `src/update.rs:33-35`).
- L'extraction de la version par `sed` est dupliquée (`build.sh:5`, `.github/workflows/release.yml:20`).
- `build.rs:59-61` écrit `dist/hdr.ico` dans l'arborescence source, hors de `OUT_DIR`, y compris pour un simple `cargo check`.
- Recommandation : mutualiser `wide()` ; faire copier l'icône vers `dist/` par `build.sh` plutôt que par `build.rs`.

### D11 — Prise en charge DPI minimale — faible

- `src/main.rs:381`, `src/main.rs:205`
- `SetProcessDPIAware` n'offre que la prise en compte du DPI système, et la taille d'icône n'est recalculée que lors d'un changement d'état HDR ; il n'y a pas de traitement de `WM_DPICHANGED`. Sur une configuration à DPI mixtes, l'icône est redimensionnée par le shell (rendu flou).
- Recommandation : déclarer `PerMonitorV2` dans un manifeste embarqué via le `.rc` déjà généré (`build.rs:64`) et régénérer l'icône sur `WM_DPICHANGED`.

### D12 — Ancienne API : indicateur de désactivation forcée ignoré — faible

- `src/display.rs:117-121`
- Dans le repli pré-24H2, seuls les bits 0 (`advancedColorSupported`) et 1 (`advancedColorEnabled`) sont lus ; le bit 3 (`advancedColorForceDisabled`) ne l'est pas, donc un écran où le HDR est bloqué est présenté comme basculable.
- Recommandation : exclure la cible si `c.value & (1 << 3) != 0`.

### D13 — Interface entièrement en français, textes en dur — faible

- `src/main.rs:239-241`, `src/main.rs:300-308`, `src/update.rs:95-97`, `installer.nsi:36`, `installer.nsi:51`
- Voir l'idée F6.

## 4. Idées de fonctionnalités

- **F1 — Mise à jour maîtrisée par l'utilisateur.** Entrée de menu à trois états (automatique / prévenir seulement / désactivée), bulle de notification « version X installée », vérification de signature. Répond directement à S1, D1 et D2.
- **F2 — Bascule par écran.** `display::toggle()` agit sur tous les écrans compatibles (`src/display.rs:159-167`). Ajouter un sous-menu listant les écrans (nom obtenu par `DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME`) avec une coche par écran, et afficher le détail dans l'infobulle.
- **F3 — Ligne de commande.** Seul `--quit` existe (`src/main.rs:368`). Ajouter `--on`, `--off`, `--toggle` et `--status` (code de sortie 0/1/2), utilisables depuis un script, un raccourci ou un Stream Deck, en réutilisant `display.rs` sans créer de fenêtre.
- **F4 — Raccourci clavier global configurable.** `RegisterHotKey` + `WM_HOTKEY` dans la fenêtre existante, en complément de `Win + Alt + B`, qui ne vise que l'écran de la fenêtre active.
- **F5 — HDR automatique par application.** Activer le HDR au lancement d'un exécutable d'une liste (jeux, lecteur vidéo) et le couper à sa fermeture ; liste stockée dans `HKCU\Software\HDRTray`.
- **F6 — Localisation.** Choisir français ou anglais selon `GetUserDefaultUILanguage`, et ajouter `MUI_LANGUAGE "English"` à l'installeur.
- **F7 — Icône adaptée au thème.** Le gris `[160, 160, 160]` (`src/icon.rs:7`) est figé ; lire `SystemUsesLightTheme` et suivre `WM_SETTINGCHANGE` pour garder un contraste correct sur une barre des tâches claire.
- **F8 — Notification en cas d'échec ou de changement externe.** Bulle (`NIF_INFO`) quand la bascule échoue (D7) ou quand un jeu modifie l'état HDR.
- **F9 — Distribution.** Signature Authenticode (supprime l'avertissement SmartScreen mentionné dans `README.md:22`) et manifeste winget.
- **F10 — Réglage de la luminosité SDR en mode HDR.** **(hypothèse)** La lecture existe (`DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL`), mais l'écriture n'a pas d'API documentée ; faisabilité à confirmer avant de s'engager.
