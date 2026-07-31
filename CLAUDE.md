# CLAUDE.md — Divotty

Contexte de handoff pour une session Claude Code sur ce projet.

Note de langue : `README.md` et `CHANGELOG.md` (publics, GitHub) sont en
anglais ; `CLAUDE.md` et `ROADMAP.md` (docs internes de handoff) restent en
français — choix délibéré, pas un oubli de traduction. Ne pas re-traduire
ces fichiers sans demande explicite.

`CHANGELOG.md` suit le format
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) : penser à y
ajouter une entrée sous `## [Unreleased]` à chaque changement notable pour
l'utilisateur (nouvelle fonctionnalité, comportement modifié, correction),
pas pour du refactoring interne sans impact visible. À bascule de version
dans `Cargo.toml`, renommer `[Unreleased]` en `[x.y.z] - AAAA-MM-JJ`.

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

- **Un seul crate, mais la frontière `core`/`tui` reste une règle à
  respecter** : le workspace à 3 crates (`divotty-core`/`divotty-tui`/
  `divotty`) a été fusionné en un unique package binaire `divotty` pour
  permettre une publication crates.io simple (voir plus bas). La séparation
  n'est donc plus imposée par le compilateur (frontière de crate), mais
  reste une convention de module à respecter par discipline :
  - `src/core/` ne doit jamais importer `ratatui`/`crossterm`. Toute la
    logique de jeu (types, résolution de coup, scoring, parsing de carte)
    doit rester testable unitairement sans terminal. Le RNG est toujours
    injecté (`impl Rng`) pour des tests déterministes — ne jamais appeler
    `rand::thread_rng()` dans `src/core/`, seulement dans `src/main.rs`.
  - `src/core/` reste sémantique, jamais de texte affichable en dur : il ne
    retourne que des enums (ex. `ScoreLabel`, `TerrainKind`) — c'est
    `src/tui/` qui traduit vers du texte selon la `Lang` choisie.
- **Extensibilité des terrains** : ajouter un nouveau type de terrain se fait
  uniquement dans `src/core/terrain.rs` (variante enum + caractère + profil).
  Le moteur de résolution (`src/core/shot.rs`) ne doit jamais avoir de
  `match` sur `TerrainKind` — il consomme uniquement `TerrainProfile`.
- **Format `.course` stable** : c'est un contrat avec le contenu (les
  fichiers de parcours). Toute modification du format (nouveaux caractères,
  structure du frontmatter) doit rester rétro-compatible ou s'accompagner
  d'une migration des fichiers existants dans `courses/`. La difficulté
  (1-4 étoiles) vit dans `course.yaml` (niveau parcours), pas dans le
  frontmatter des fichiers `.course` de trou (niveau trou) — elle n'est liée
  qu'au parcours. Exemple de champ ajouté sans casser l'existant : `width`/
  `height` optionnels dans le frontmatter d'un trou (voir plus bas, "taille
  variable par trou") — absents, le trou reste 100x60 plein comme avant.
- **Grille 100x60 vs terminal** : la carte est presque toujours plus grande
  que le terminal visible. Le rendu passe systématiquement par un `Viewport`
  qui centre sur la balle (`src/tui/render.rs`) — ne pas tenter d'afficher
  toute la grille d'un coup.
  - Grille passée de 50x25 à 100x60 (constantes `COURSE_WIDTH`/
    `COURSE_HEIGHT` dans `src/core/course.rs`) pour permettre des trous de
    par 4 à par 8 : avec le système dé/club actuel, un Driver moyen
    parcourt ~14 cases (jusqu'à 24 au mieux), donc l'ancienne diagonale
    max (~56 cases) plafonnait bien avant un par 6-8 crédible (qui demande
    ~80-100+ cases de trajectoire cumulée). Le ratio n'est plus 2:1 strict
    (hauteur ×2.4 contre largeur ×2) pour laisser plus de marge aux trous
    à dominante verticale.
  - Alternative envisagée et écartée pour l'instant : dimensions variables
    par trou (déclarées dans le frontmatter) plutôt qu'une grille globale
    fixe — plus élégant (pas de remplissage hors-limites inutile sur un
    par 3 dans un immense canevas) mais casse le contrat "dimensions
    fixes" et transformerait `COURSE_WIDTH`/`COURSE_HEIGHT` en champs par
    trou plutôt qu'en constantes globales, utilisées un peu partout dans
    `course.rs`/`shot.rs`. À reconsidérer si le remplissage à vide devient
    gênant en pratique, possiblement en lien avec le futur builder de
    trous.
- **Répertoire de données résolu, pas figé sur le cwd** : `courses/`,
  `save.yaml` et `courses/_library/` sont tous préfixés par une racine de
  données (`resolve_data_root()` dans `src/main.rs`) calculée une fois au
  lancement, pas par un chemin cwd en dur. Chaîne à 3 niveaux :
  1. `./courses` (cwd) si ce dossier existe déjà — workflow de
     développement inchangé, lancer le jeu depuis la racine du dépôt
     (`cargo run`) continue de fonctionner exactement comme avant.
  2. Sinon le dossier de données de la plateforme (`directories::
     ProjectDirs`, `~/.local/share/divotty` sur Linux, équivalents
     macOS/Windows) — corrige le cas `cargo install` : un binaire installé
     et lancé depuis n'importe où obtient un emplacement stable plutôt
     qu'un `courses/`/`save.yaml` différent à chaque répertoire courant.
     Ce dossier reste la cible même s'il est encore vide (rien n'y a
     encore été sauvegardé) — seule la *liste* des parcours affichés
     retombe alors sur l'étape 3, pas l'emplacement d'écriture.
  3. Si aucun des deux n'a de parcours, le jeu bascule sur les vrais
     parcours (`courses/demo/` et `courses/quick3/`) **embarqués dans le
     binaire à la compilation** (`include_str!`, voir `embedded_courses()`
     dans `src/main.rs`) — pas un trou générique de secours (voir
     historique ci-dessous, ça l'a été jusqu'à ce que ce soit corrigé avant
     0.2.0). Ces parcours embarqués ne sont pas sauvegardables (pas de
     dossier associé, comme n'importe quel parcours sans `course_dir`). Un
     test (`embedded_courses_parse_and_match_the_real_courses_on_disk`)
     vérifie que le contenu embarqué reste parsable à chaque `cargo test`.

## Architecture (rappel rapide)

```
src/core/  → terrain.rs   (types + profils)
             course.rs     (grille + parser + Course::discover pour lister les parcours d'un dossier)
             shot.rs        (résolution de coup + ShotPreview/preview_shot pour l'aperçu avant de jouer)
             scoring.rs      (HoleScore/Scorecard, ScoreLabel sémantique — pas de texte)
             mod.rs           (ré-exports publics du module core)

src/tui/   → render.rs    (CourseView + Viewport, superpose le guide de trajectoire + halo de dispersion)
             sidebar.rs     (7 panneaux d'info : Titre, Trou, Score, Club, Dernier coup, Visée, Contrôles)
             menu.rs         (CourseMenuState : écran de sélection de parcours)
             scorecard.rs     (ScorecardView : écran plein cadre de fin de partie, détail trou par trou + total)
             lang.rs           (Lang { En, Fr }, défaut En — bascule avec la touche L)
             format.rs          (helpers d'affichage partagés : étoiles de difficulté, couleur/texte de score)
             mod.rs              (ré-exports publics du module tui)

src/main.rs → `mod core; mod tui;` + menu de sélection → GameState → boucle
              de jeu (enchaînement des trous, écran de scorecard en fin de
              partie) ; sauvegarde/reprise ; gestion clavier

courses/demo   → parcours d'exemple à 1 trou (course.yaml avec difficulty + hole_01.course)
courses/quick3 → parcours à 3 trous (par 3/4/5) pour tester l'enchaînement multi-trous
```

**Historique** : c'était un workspace Cargo à 3 crates (`divotty-core`,
`divotty-tui`, `divotty`) sous `src/core`, `src/tui`, `src/app`, chacun avec
son propre `Cargo.toml`. Fusionné en un seul package pour publier sur
crates.io sans devoir publier 3 crates séparés (une dépendance `path` avec
version doit pointer vers un crate déjà publié — donc publier `divotty` seul
imposait de fusionner). Les anciens `lib.rs` de chaque crate sont devenus de
simples `mod.rs`, et les imports inter-crates (`divotty_core::X`,
`divotty_tui::Y`) sont devenus `crate::core::X`, `crate::tui::Y`.

## État du projet (au moment de ce handoff)

Fait et testé :
- Parsing de fichiers `.course` avec validation (dimensions exactes 100x60,
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
  repère `✛` sur la carte. Volontairement sans vent (voir plus bas). En
  zoom, la balle et le trou n'occupent qu'une seule sous-case du bloc
  zoomé (terrain réel/cadre `╭─╮│⚑│╰─╯` autour) plutôt que de le remplir
  en entier, avec une flèche de visée près de la balle et des points de
  guide en petits segments de 3 (horizontal/vertical/diagonal) plutôt que
  des points isolés — détail complet dans `ROADMAP.md` (v0.3).
- Putting moins aléatoire : la dispersion du Putter dépend désormais de la
  distance restante au trou (`putter_base_dispersion()` dans `shot.rs`) —
  un putt court est quasi automatique, un long putt garde son plein
  risque. Seul le Putter est concerné.
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
  balle, vent, club, direction visée, plafond de dé, scorecard cumulé) ; le
  menu propose `[C]` pour reprendre si un fichier de sauvegarde existe. Un
  parcours sans dossier associé (`course_dir: None` — les parcours
  embarqués, voir plus bas) n'est pas sauvegardable. L'historique des
  points d'arrêt du trou courant (`shot_history`, voir plus bas) n'est
  volontairement pas persisté : une reprise repart avec un historique
  limité à la position actuelle.
- Quitter nécessite une double pression sur `q` (`qq`), au menu comme en
  jeu — un indice "Press q again to quit" s'affiche après la première
  pression, annulé par toute autre touche.
- Boucle de jeu jouable sur un parcours multi-trous complet (flèches pour
  viser, Tab pour changer de club, Espace pour jouer, S pour sauvegarder, L
  pour changer de langue, qq pour quitter). `GameState` garde tous les
  trous du `Course` (`holes: Vec<Hole>` + `current_hole()`), pas juste le
  premier.
- Enchaînement automatique des trous : une fois un trou terminé, `N`
  (visible uniquement s'il en reste un) passe au suivant
  (`GameState::advance_hole`, testé unitairement) — balle, visée, vent,
  coups et club repartent à zéro comme pour un nouveau trou. Sur le
  dernier trou, `Entrée` ("finir la partie") enregistre le score final et
  affiche un écran de scorecard complet plein cadre (`tui::ScorecardView`,
  `src/tui/scorecard.rs`) : détail trou par trou (nom, par, coups, label
  Birdie/Bogey/etc.) + total, avant retour au menu — y compris pour un
  parcours à un seul trou (le trou de démo inclus). `core::scoring::Scorecard`
  (cumul coups/par/écart) est persisté dans `save.yaml`, donc reprendre une
  partie multi-trous ne perd pas les trous déjà joués. Le panneau Score
  affiche un total cumulé ("Total: N (±M)") dès qu'un parcours a plus d'un
  trou.
- Un deuxième parcours, `courses/quick3/` (3 trous, par 3/4/5, difficulté 2
  étoiles), sert de format court pour tester l'enchaînement sans attendre
  un 9 trous complet — voir `courses/demo/` pour l'exemple à un seul trou.
- Drop réaliste sur l'eau/hors-limites (`backtrack_to_safe_landing()` dans
  `shot.rs`) : au lieu d'un retour pur et simple à la position de départ du
  coup, la balle remonte la trajectoire depuis le point d'impact et
  s'arrête à la première case qui ne force pas elle-même un drop (eau,
  hors-limites) ni ne bloque la trajectoire (arbre) — sauter par-dessus un
  arbre rencontré en chemin plutôt que de s'y arrêter, sinon ce serait
  juste remplacer un obstacle par un autre. Le coup de pénalité reste dû
  quel que soit l'endroit où la balle finit par se poser. Repli sur
  l'ancien comportement (retour au départ) si tout le chemin est un
  obstacle. Un arbre qui bloque *directement* un coup (`blocks_trajectory`)
  reste inchangé — décision explicite : la balle reste dans l'arbre, ça ne
  gêne pas tant que le coup suivant est plus difficile (pas de pénalité de
  coup, juste un lie pénalisant). Le panneau Dernier coup affiche
  désormais où la balle a été droppée ("Dropped · the fairway" plutôt
  qu'un message générique), volontairement court (`Dropped ·`/`Droppée ·`
  plutôt que "Penalty, dropped on" / "Pénalité, balle droppée sur") car le
  panneau ne fait que ~24 caractères utiles une fois la marge de gauche
  retirée, et certains noms de terrain (FR "une zone à pénalité") auraient
  rendu une formulation plus longue tronquée — vérifié dans le terminal,
  pas juste en théorie.
- Rappel visuel du parcours une fois le trou terminé : `GameState.
  shot_history` (`Vec<Pos>`, départ inclus, complété après chaque coup,
  remis à `[tee]` sur `restart_hole`/`advance_hole`) donne à
  `CourseView::path` la liste des arrêts. Remplace l'aperçu de visée
  (`preview`, mis à `None` une fois fini) par une balle rouge à chaque
  arrêt intermédiaire et un pointillé de balles jaunes reliant les points
  consécutifs, du départ au trou.
- Les 7 panneaux de la sidebar ont un fond vert sombre partagé
  (`SIDEBAR_BG` dans `sidebar.rs`, clin d'œil golf) plutôt que le noir par
  défaut du terminal, et une amorce colorée (`▏`, `prefix_line()`) entre
  la bordure gauche et le texte de chaque ligne — y compris les lignes
  vides du panneau Contrôles, pour que l'accent forme une "colonne"
  visible sur toute la hauteur du panneau. Le panneau Titre n'a plus de
  libellé de bordure ("Divotty"), juste l'icône + version à l'intérieur.
- Format `.course` à taille variable par trou : `HoleMeta.width`/`height`
  (optionnels, `#[serde(default)]`) déclarent une grille plus petite que
  100x60, centrée automatiquement dans le canevas complet à l'issue du
  parsing (`Hole::parse` dans `src/core/course.rs`) — le reste est rempli
  de hors-limites, et `tee`/`hole_pos` sont translatés du même offset.
  Sans ces champs, comportement inchangé (100x60 plein). Le moteur de
  résolution, le rendu et le `Viewport` ne voient donc jamais qu'un `Hole`
  déjà 100x60 — aucun changement requis en dehors du parsing. Au passage,
  le nombre de lignes de la grille est désormais explicitement validé
  contre la hauteur attendue (`CourseError::BadRowCount`) — ce n'était pas
  le cas avant, un trou avec trop/pas assez de lignes aurait pu parser
  silencieusement avec la mauvaise hauteur.
- Builder de trous (phases 1-9 sur 11, voir `ROADMAP.md` pour le détail et
  ce qui reste, hors scope v1) : depuis le menu (`E`), un sélecteur
  (`pick_hole_to_build`, `tui::HolePickerView`) propose "+ Nouveau trou" ou
  un fichier `.course` existant trouvé sous `courses/*/`
  (`discover_hole_files`, testée — inclut `courses/_library/`). Pour un
  trou neuf, un en-tête minimal (par obligatoire, orientation) suggère une
  taille de grille (`suggested_declared_size`), puis le trou se dessine
  entièrement au clavier dans `BuilderState`
  (`src/main.rs`) — taper un caractère de terrain valide peint la case et
  avance automatiquement le curseur (ligne par ligne ou colonne par
  colonne selon l'orientation), les flèches déplacent librement, `U`
  annule, `N` renomme, `S` sauvegarde. La sauvegarde sérialise l'état
  (`BuilderState::to_course_raw`) et valide via `Hole::parse` avant
  d'écrire sur disque — un fichier produit par le builder est donc
  toujours un fichier que le jeu sait déjà charger, vérifié en tmux
  (trou créé, sauvegardé, rechargé et joué de bout en bout). Emplacement
  de sauvegarde systématique dans `courses/_library/` (`HOLE_LIBRARY_DIR`,
  créé si absent) — le joueur ne choisit jamais l'emplacement, seulement
  un nom de fichier sans extension, nettoyé (`sanitize_hole_filename`,
  testée : ne garde que lettres/chiffres/`-`/`_`, ce qui élimine tout
  risque de séparateur de chemin ou de `..`). En cas de collision,
  `BuilderMode::ConfirmOverwrite` demande confirmation avant d'écraser
  (`Entrée`/`Y` écrase, `Échap`/`N` revient à la saisie du nom, conservée,
  pour la modifier) plutôt qu'un compteur automatique — écarté après
  signalement : ça aurait empêché de mettre à jour un trou déjà
  sauvegardé (chaque sauvegarde aurait créé un nouveau fichier au lieu de
  remplacer l'ancien). Un message de confirmation affiche le chemin final
  et rappelle que le trou n'est pas encore inclus dans un parcours, sans
  quitter automatiquement le builder. La frappe est
  insensible à la casse (`terrain_from_builder_key`, testée — `d` comme
  `D`, `t` comme `T`, etc. ; seul le format `.course` lui-même reste
  strict), et le bandeau d'en-tête affiche en permanence la légende des
  caractères de terrain — chaque entrée colorée avec la couleur exacte de
  ce terrain sur la carte (même source, `terrain_style()` dans
  `render.rs`) — ainsi qu'un indicateur de position ("Position: x=.. y=..",
  coordonnées 0-indexées identiques à celles de la grille du fichier
  `.course` et des axes du canevas PDF, pour naviguer directement vers une
  case repérée sur un plan dessiné à l'avance) accompagné du rappel de
  ligne/colonne courante dans le sens de l'avancée automatique, qui se
  colore en avertissement à l'approche de la dernière ligne/colonne, là où
  l'avancée automatique s'arrête net. `Échap` (retour au menu) a sa propre
  confirmation double, distincte de `qq` (quitter l'application entière) —
  `BuilderState::exit_confirm` — et n'importe quelle touche autre qu'un
  deuxième `Échap` annule, y compris `S` qui bascule normalement vers la
  sauvegarde (permet donc de sauvegarder le travail en cours avant de
  sortir, sans logique dédiée). Charger un fichier existant (choisi dans
  le sélecteur) propose "Modifier" (réécrit directement ce fichier — `S`
  saute alors la saisie de nom et `ConfirmOverwrite`) ou "Dupliquer"
  (comportement d'un trou neuf, nom demandé à chaque sauvegarde). L'état
  d'édition est reconstruit par `BuilderState::from_existing_hole`
  (testée) à partir de `Hole::local_tiles()` (testée, réutilisée aussi par
  `to_course_string` — élimine une duplication de calcul d'offset qui
  existait entre les deux), l'orientation étant déduite du rapport
  largeur/hauteur de la grille chargée. Le sélecteur est scindé en deux
  colonnes (même esprit que sidebar/carte en jeu) : la liste à gauche, un
  panneau `tui::HolePreviewView` à droite qui affiche nom/par/dimensions
  et une mini-carte du trou sous le curseur de sélection, recalculé à
  chaque case survolée (parser un seul petit fichier `.course` par frame
  est négligeable). `BuilderView::cursor` est passé de `Pos` à
  `Option<Pos>` pour permettre ce réemploi en lecture seule (`None` = pas
  de case surlignée, centrage sur le milieu de la grille) sans dupliquer
  la logique de rendu. Généralisation ensuite à tout le builder : l'écran
  de dessin (`run_builder`) est passé d'un bandeau plein largeur à une
  colonne gauche (`tui::BuilderSidebarView`, largeur 28 comme la sidebar du
  jeu) / grille à droite — quatre panneaux ("Hole", "Position", "Legend",
  "Controls") réutilisant les helpers de style de `sidebar.rs` (`panel`/
  `panel_bottom_aligned`/`SIDEBAR_BG`, passés de privés à `pub(crate)`).
  Un premier jet avait retiré la légende de cette colonne (déplacée dans
  l'écran d'en-tête, resté alors un unique panneau plein écran) — signalé
  comme pas satisfaisant, corrigé : la légende revient dans la sidebar de
  dessin via un panneau "Legend" dédié (`terrain_legend_lines`, deux
  entrées de terrain par ligne pour tenir dans la colonne étroite), et
  l'écran d'en-tête (`BuilderSetupView`) devient lui aussi deux colonnes —
  formulaire à gauche, aperçu de la grille vierge à la taille suggérée à
  droite (même `BuilderView` en lecture seule que l'aperçu du sélecteur) —
  cohérent avec les deux autres écrans du builder. Les messages de
  sauvegarde/erreur sont découpés à la limite des mots (`wrap_text`,
  testée) plutôt que tronqués à mi-mot dans la colonne étroite.
- Répertoire de données résolu (`resolve_data_root()`/
  `resolve_data_root_from()`, testée, voir principe directeur ci-dessus) :
  `courses/`, `save.yaml` et `courses/_library/` suivent tous la même
  racine calculée une fois au lancement, avec repli sur le dossier de
  données de la plateforme (`directories::ProjectDirs`) si `courses/` est
  absent du cwd. Nouvelle dépendance `directories`. Vérifié en tmux :
  lancé depuis la racine du dépôt, comportement inchangé (sauvegarde et
  bibliothèque restent locales) ; lancé depuis un dossier sans `courses/`
  (simulant un `cargo install`), sauvegarde et trou du builder atterrissent
  dans `~/.local/share/divotty/` et sont retrouvés à la relance depuis ce
  même dossier vide.

Pas encore fait (voir `ROADMAP.md` pour le détail) :
- Tests d'intégration sur la boucle de jeu complète (`run_loop`, gestion
  clavier réelle). `GameState` (logique pure : `play_shot`, `cycle_club`,
  `nudge_aim`, `restart_hole`, `finished`, `advance_hole`,
  `adjust_die_strength`...) a maintenant 41 tests unitaires dans
  `src/main.rs` (88 au total avec `core` et `tui`), mais rien qui simule un
  vrai terminal/`crossterm`.

## Publication crates.io

Fait : `divotty` v0.2.0 est publié sur crates.io
(https://crates.io/crates/divotty), installable via `cargo install
divotty`. Compte crates.io authentifié en local (`cargo login` fait,
credentials dans `~/.cargo/credentials.toml`).

À savoir pour la suite :
- Une version publiée ne peut pas être supprimée (seulement « yankée » via
  `cargo yank`) — toute future publication d'une nouvelle version doit
  passer par un bump de `version` dans `Cargo.toml` et repasser par
  `cargo publish --dry-run` avant le vrai `cargo publish`, à confirmer
  explicitement avec l'utilisateur avant de l'exécuter (action irréversible
  et visible publiquement).
- Après un `cargo install`, le binaire n'a plus accès à `courses/demo/`
  ni `courses/quick3/` sur disque (restés dans le crate source, pas
  copiés à côté du binaire installé) : les vrais parcours, embarqués dans
  le binaire à la compilation (`embedded_courses()` dans `src/main.rs`,
  voir plus haut), prennent le relais automatiquement — un joueur
  `cargo install` voit donc « Le Ravin » et « Quick 3 », pas un trou
  générique (limitation corrigée avant la publication de 0.2.0 ; c'était
  le cas pour 0.1.0).

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
