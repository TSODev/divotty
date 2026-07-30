# CLAUDE.md — Divotty

Contexte de handoff pour une session Claude Code sur ce projet.

Note de langue : `README.md` (public, GitHub) est en anglais ; `CLAUDE.md`
et `ROADMAP.md` (docs internes de handoff) restent en français — choix
délibéré, pas un oubli de traduction. Ne pas re-traduire le README en
français sans demande explicite.

## Ce que c'est

Jeu de golf en TUI, Rust. Mécanique : dé (distance de base) + club choisi +
direction visée par le joueur + modificateurs du terrain de départ (rough,
bunker, etc.) → résolution d'un coup avec dispersion aléatoire → nouvelle
position de balle, éventuelles pénalités (eau, hors-limites), détection du
trou atteint.

## Principes de conception

Trois mots donnés explicitement comme boussole du projet : **simplicité,
rapidité, amusement**. En cas de doute sur une fonctionnalité qui pourrait
tirer vers plus de complexité (matrice de modificateurs, simulation
physique poussée, etc.), préférer la version la plus simple qui reste
amusante, et signaler le compromis plutôt que de foncer vers l'exhaustivité.
Exemples déjà appliqués : la sensibilité au terrain est un seul facteur par
club (pas une matrice club×terrain complète) ; le survol d'obstacle est une
seule règle de zone basse (pas une simulation d'arc de trajectoire).

## Principes directeurs (techniques)

- **`core` est pur** : aucune dépendance à `ratatui`/`crossterm`. Toute la
  logique de jeu (types, résolution de coup, scoring, parsing de carte) doit
  rester testable unitairement sans terminal. Le RNG est toujours injecté
  (`impl Rng`) pour permettre des tests déterministes avec seed fixe — ne
  jamais appeler `rand::thread_rng()` à l'intérieur de `core`, seulement dans
  `app` (la boucle de jeu réelle).
- **`core` reste sémantique, jamais de texte affichable en dur** : depuis
  l'introduction du multilingue, `core` ne retourne que des enums (ex.
  `ScoreLabel`, `TerrainKind`) — c'est `tui` qui traduit vers du texte selon
  la `Lang` choisie. Ne jamais réintroduire une `&str`/`String` figée dans
  `core` pour quelque chose que le joueur voit à l'écran.
- **Extensibilité des terrains** : ajouter un nouveau type de terrain se fait
  uniquement dans `src/core/src/terrain.rs` (variante enum + caractère +
  profil).
  Le moteur de résolution (`shot.rs`) ne doit jamais avoir de `match` sur
  `TerrainKind` — il consomme uniquement `TerrainProfile`.
- **Format `.course` stable** : c'est un contrat avec le contenu (les
  fichiers de parcours). Toute modification du format (nouveaux caractères,
  structure du frontmatter) doit rester rétro-compatible ou s'accompagner
  d'une migration des fichiers existants dans `courses/`. La difficulté
  (1-4 étoiles) vit dans `course.yaml` (niveau parcours), pas dans le
  frontmatter des fichiers `.course` de trou (niveau trou) — elle n'est liée
  qu'au parcours.
- **Grille 25x50 vs terminal** : la carte est presque toujours plus grande
  que le terminal visible. Le rendu passe systématiquement par un `Viewport`
  qui centre sur la balle (`src/tui/src/render.rs`) — ne pas tenter d'afficher
  toute la grille d'un coup.
- **Chemins relatifs au cwd, pas au crate** : `courses/` et `save.yaml` sont
  résolus par `app` relativement au répertoire courant d'exécution, pas à
  l'emplacement du crate. Le jeu doit être lancé depuis la racine du dépôt
  (`cargo run -p divotty`), pas depuis `src/app/`.

## Architecture (rappel rapide)

```
src/core → terrain.rs   (types + profils)
           course.rs     (grille + parser + Course::discover pour lister les parcours d'un dossier)
           shot.rs        (résolution de coup + ShotPreview/preview_shot pour l'aperçu avant de jouer)
           scoring.rs      (HoleScore/Scorecard, ScoreLabel sémantique — pas de texte)

src/tui  → render.rs    (CourseView + Viewport, superpose le guide de trajectoire + halo de dispersion)
           sidebar.rs     (7 panneaux d'info : Titre, Trou, Score, Club, Dernier coup, Visée, Contrôles)
           menu.rs         (CourseMenuState : écran de sélection de parcours)
           lang.rs          (Lang { En, Fr }, défaut En — bascule avec la touche L)
           format.rs         (helpers d'affichage partagés, ex. étoiles de difficulté)

src/app  → main.rs (menu de sélection → GameState → boucle de jeu ; sauvegarde/reprise ; gestion clavier)

courses/demo → parcours d'exemple à 1 trou (course.yaml avec difficulty + hole_01.course)
```

Les trois crates vivent sous `src/` (regroupement demandé explicitement,
non idiomatique pour un workspace Cargo mais choix assumé) ; `Cargo.toml`
à la racine référence `members = ["src/core", "src/tui", "src/app"]`. Les
dépendances relatives entre crates (`path = "../core"` etc.) n'ont pas
changé, seule la liste `members` a bougé.

## État du projet (au moment de ce handoff)

Fait et testé :
- Parsing de fichiers `.course` avec validation (dimensions exactes 50x25,
  une seule case `D`, une seule case `H`, caractères reconnus).
- `course.yaml` porte un champ `difficulty` (1-4, validé) ; `Course::discover`
  scanne un dossier racine et renvoie chaque parcours avec son dossier
  d'origine (pour pouvoir le recharger, ex. reprise de sauvegarde).
- 6 clubs (Driver, Wood, Hybrid, Iron, Wedge, Putter), chacun avec distance
  de base, dispersion de base et une **sensibilité au terrain** propre : un
  terrain difficile (rough, bunker) pénalise proportionnellement plus un
  club long qu'un club court (voir `Club::terrain_sensitivity` et
  `effective_dispersion` dans `shot.rs`).
- Survol d'obstacles : une trajectoire n'est bloquée par un obstacle
  (`TerrainProfile::blocks_trajectory`, ex. arbre) que s'il est rencontré
  dans les premiers ~15% du vol (`LOW_ALTITUDE_FRACTION`) — au-delà, la
  balle est considérée en vol et survole tout, y compris l'eau (qui ne
  bloque jamais la trajectoire, seul l'atterrissage compte).
- Aperçu de coup avant de jouer : `preview_shot` calcule portée min/max/
  moyenne (dé 1/6/4) et rayon de dispersion sans consommer de RNG ; `tui`
  superpose un guide en pointillés, une zone de dispersion (magenta) et un
  repère `✛` sur la carte.
- Résolution de coup avec dé + club + terrain + dispersion aléatoire,
  déterministe sous seed fixe.
- Interface deux colonnes : sidebar à gauche (7 panneaux empilés), carte à
  droite avec viewport dynamique (occupe toute la largeur/hauteur restante).
- Multilingue (anglais par défaut, français en bascule avec `L`) : voir
  principe "core reste sémantique" ci-dessus. Ajouter une langue = ajouter
  une variante à `tui::Lang` + compléter les fonctions de traduction de
  `sidebar.rs`/`menu.rs`, rien à toucher dans `core`.
- Menu de sélection de parcours au lancement (`tui::CourseMenuState`) :
  liste les parcours trouvés sous `courses/` avec nom, étoiles de
  difficulté, par total et nombre de trous ; navigation ↑↓, Entrée pour
  jouer.
- Sauvegarde/reprise d'une partie en cours : `S` sauvegarde dans
  `save.yaml` (dossier du parcours, numéro de trou, coups, position de
  balle, club, direction visée) ; le menu propose `[C]` pour reprendre si
  un fichier de sauvegarde existe. Le parcours de secours généré en mémoire
  (fallback sans dossier) n'est pas sauvegardable.
- Quitter nécessite une double pression sur `q` (`qq`), au menu comme en
  jeu — un indice "Press q again to quit" s'affiche après la première
  pression, annulé par toute autre touche.
- Boucle de jeu jouable sur un seul trou (flèches pour viser, Tab pour
  changer de club, Espace pour jouer, S pour sauvegarder, L pour changer de
  langue, qq pour quitter).

Pas encore fait (voir `ROADMAP.md` pour le détail) :
- Enchaînement de plusieurs trous (1/9/18) avec carte de score complète —
  `GameState` ne joue toujours que `holes[0]` d'un `Course`, même si
  `hole_index`/`hole_count` sont déjà suivis dans l'état (prêts pour cette
  extension).
- Système de "drop" plus réaliste (actuellement : retour pur et simple à la
  position de départ du coup — une vraie implémentation dropperait au dernier
  point de fairway valide le long de la trajectoire).
- Tests d'intégration sur la boucle de jeu complète (`src/app/`) — les tests
  actuels couvrent uniquement `core` (13 tests entre `course.rs` et
  `shot.rs`).

## Contraintes d'environnement rencontrées

Le sandbox de développement utilisé pour le squelette initial avait
`rustc`/`cargo` 1.75 (installés via `apt`, pas `rustup`), d'où le pin
`ratatui = "0.26"` / `crossterm = "0.27"` dans le `Cargo.toml` du workspace
(certaines versions plus récentes de dépendances comme `ratatui` 0.29+
demandent edition2024 / rustc 1.85+).

**Mise à jour** : la machine utilisée depuis a `rustc`/`cargo` 1.97 (via
`rustup`), largement suffisant pour remonter ces pins si besoin — ça n'a
pas été fait dans cette session (non demandé), les pins `0.26`/`0.27`
sont donc toujours d'actualité dans `Cargo.toml`. À faire si une future
fonctionnalité a besoin d'une API `ratatui` plus récente (`Frame::size()`
→ `Frame::area()`, indexation directe `buf[(x,y)]` depuis ~0.27+).
