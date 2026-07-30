# ROADMAP — Divotty

## v0.1 — Squelette jouable (fait)
- [x] Structure du projet en modules (`core` / `tui`) au sein d'un crate
      binaire unique `divotty` (a été un workspace à 3 crates, fusionné pour
      simplifier la publication crates.io — voir `CLAUDE.md`)
- [x] Types de terrain + profils de jeu (distance, dispersion, pénalités)
- [x] Format `.course` (frontmatter YAML + grille ASCII 100x60) + parser validé
      (grille passée de 50x25 à 100x60 pour permettre des trous de par 4 à
      par 8 — voir `CLAUDE.md`)
- [x] Moteur de résolution de coup (dé + club + terrain + dispersion aléatoire seedable)
- [x] Rendu TUI avec viewport suivant la balle + HUD basique
- [x] Boucle de jeu sur un trou unique
- [x] Clubs étendus (Driver, Wood, Hybrid, Iron, Wedge, Putter) avec une
      sensibilité au terrain par club (un terrain difficile pénalise
      proportionnellement plus un club long qu'un club court)
- [x] Distances de club recalibrées en ratio réaliste du Driver (Bois 90%,
      Hybride 80%, Fer 62.5%, Wedge 35% — putter volontairement laissé de
      côté, régime à part) : un Driver moyen laisse désormais une approche
      à portée de Bois/Fer sur un par 4 typique (~53 cases tee-trou, voir
      le trou de démo), au lieu de nécessiter deux Driver complets.
- [x] Interface deux colonnes (sidebar multi-panneaux + carte) — remplace le
      HUD une ligne initial
- [x] Zone à pénalité (`TerrainKind::PenaltyZone`, caractère `X`, affichée
      en rouge) : coûte un coup mais, contrairement à l'eau/hors-limites,
      ne force pas de drop — la balle reste sur place. Seul terrain à
      combiner pénalité et absence de drop forcé (chemin de code jusque-là
      jamais exercé). Ajoutée de chaque côté du rough dans le trou de démo
      pour la tester en conditions réelles.

## v0.2 — Parcours complets
- [ ] Enchaînement automatique des trous d'un `Course` (1 → 9 → 18) —
      `GameState` suit déjà `hole_index`/`hole_count`, prêt pour cette
      extension, mais ne joue toujours que `holes[0]`
- [ ] Carte de score complète (`Scorecard`) affichée entre les trous et en fin de partie
- [ ] Écran de fin de trou (résumé du score, label Birdie/Bogey/etc.)
- [x] Menu de sélection de parcours (lecture de `courses/*/course.yaml`)
- [x] Difficulté par parcours (1 à 4 étoiles, champ `difficulty` dans
      `course.yaml`, purement indicative)
- [ ] Panneau "Score" façon carte de golf : n° de trou, par du trou, score
      courant, adapté au nombre de trous du parcours (1/3/9/18).
      **Prérequis** : dépend directement des deux items ci-dessus
      (enchaînement multi-trous + `Scorecard`) — impossible d'afficher un
      score sur 9 trous tant que le jeu ne peut jouer qu'un trou. À
      implémenter avec/après eux, pas avant.
- [ ] Parcours rapide à 3 trous (`courses/quick3/`) : bon format pour
      tester l'enchaînement multi-trous sans attendre un 9 trous complet ;
      bon candidat aussi pour exercer le futur format de grille taille
      variable (trous courts = grilles compactes, voir v0.4)

## v0.3 — Visée et feedback
- [x] Vent (direction + force), tiré aléatoirement au chargement du trou
      (`random_wind()` dans `main.rs`, jamais `thread_rng()` dans `core`) et
      appliqué comme une **dérive directionnelle** (pas juste plus de
      dispersion aléatoire) proportionnelle à la distance du coup — un
      putt n'est jamais affecté (balle au sol). Pris en compte à la fois
      par `resolve_shot` et `preview_shot` (l'aperçu reste fiable). Affiché
      dans le panneau Visée, à côté de la boussole de visée du joueur —
      pas de 8e panneau.
- [x] Aperçu visuel de la zone de dispersion avant de jouer (guide de
      trajectoire en pointillés + halo de dispersion + repère d'atterrissage
      moyen, superposés sur la carte)
- [x] Survol d'obstacles : un arbre proche du départ bloque le coup, un
      arbre plus loin sur la trajectoire est survolé (zone basse de vol,
      voir `LOW_ALTITUDE_FRACTION` dans `shot.rs`) ; l'eau ne bloque jamais
      la trajectoire, seul l'atterrissage compte
- [x] Zoom visuel (facteur x3, `ZOOM_FACTOR` dans `render.rs`), activé/
      désactivé manuellement par la touche `Z` (désactivé par défaut,
      pas automatique sur le green) : chaque case s'affiche comme un bloc
      de plusieurs caractères au lieu d'un seul. Purement cosmétique — le
      modèle de position (`Pos`, cases entières) ne change pas.
- [ ] Rendre le putting moins aléatoire — à concevoir maintenant que le
      zoom rend la zone du green lisible. Actuellement `Club::Putter` a la
      dispersion la plus faible (0.2) mais reste un simple tirage aléatoire
      comme les autres clubs ; pistes à explorer : réduire encore la
      dispersion résiduelle, introduire un facteur de "précision" lié à la
      distance restante, ou un mini-mécanisme moins purement RNG (pas encore
      tranché).
- [ ] Pente sur le green (idée candidate pour l'item ci-dessus) : une
      indication d'altitude par case du green qui dévie la trajectoire du
      putt de façon **déterministe** (vers la pente descendante) plutôt que
      purement aléatoire — plus lisible/skill-based qu'un simple ajustement
      de dispersion RNG. Choix de conception à trancher : quelques nouvelles
      variantes de `TerrainKind` (`GreenSlopeN`/`S`/`E`/`O`, cohérent avec le
      modèle d'extension existant — un caractère + un profil chacune) versus
      une vraie couche d'altitude par case (plus expressive/continue, mais
      changement de format `.course` plus lourd). Probablement à combiner
      avec le zoom (v0.3, ci-dessus) pour que le joueur puisse "lire" la
      pente avant de putter.
- [ ] Historique des coups joués sur le trou courant, affiché dans le HUD
- [ ] Amélioration du drop : reprise au dernier point de fairway valide le long
      de la trajectoire plutôt qu'un retour pur à la position de départ
- [ ] Animation simple du déplacement de la balle (interpolation position par position)
- [ ] Panneau "Dernier coup" enrichi : distance déduite + club utilisé, en
      plus du dé déjà affiché (ex. `Driver — Dé: 5 — Distance: 18 — Balle
      sur le fairway` plutôt que les infos séparées d'aujourd'hui).
      **Plan** :
      1. Ajouter `distance: f32` et `club: Club` à `ShotResult`
         (`src/core/shot.rs`) — `resolve_shot` calcule déjà
         `effective_distance`, il suffit de la exposer plutôt que de la
         jeter ; le club vient de `shot.club`, déjà connu au moment de
         l'appel.
      2. `src/tui/sidebar.rs::shot_message` : inclure `club_label(...)` et
         la distance (arrondie) dans le message construit à partir du
         `ShotResult` stocké.
      3. Vérifier les tests existants qui construisent `ShotResult` (tous
         passent par `resolve_shot`, pas de construction manuelle à
         corriger a priori).

## v0.4 — Contenu et outillage
- [ ] Au moins un parcours complet à 9 trous, dessiné et testé — en variant
      l'orientation des trous (horizontal/vertical) plutôt que de figer un
      seul style ; la grille 100x60 (voir v0.1) et le `Viewport` s'adaptent
      déjà aux deux sans changement de code, c'est un choix de dessin par
      trou (voir `CLAUDE.md`, principes de conception)
- [ ] Format `.course` à taille variable par trou, centrée dans le canevas
      100x60 (au lieu de forcer chaque trou, même un par 3, à remplir
      entièrement 100x60 de hors-limites). Isolé entièrement dans le
      parsing, aucun changement pour le moteur de résolution, le rendu, ou
      le `Viewport`, qui continuent de voir un `Hole` toujours 100x60.
      **Plan** :
      1. Ajouter `width: Option<usize>` / `height: Option<usize>` à
         `HoleMeta` (`src/core/course.rs`), `#[serde(default)]` → `None`
         signifie "100x60 complet", rétro-compatible avec les fichiers
         existants qui ne déclarent rien.
      2. `Hole::parse` : valider la grille ASCII contre la taille déclarée
         (`declared_w`/`declared_h`, par défaut `COURSE_WIDTH`/
         `COURSE_HEIGHT`) au lieu des constantes globales directement —
         garde le filet de sécurité actuel contre les erreurs de saisie
         (mismatch = erreur, comme aujourd'hui).
      3. Nouvelle variante d'erreur `CourseError::DeclaredSizeTooLarge` si
         `width`/`height` déclarés dépassent 100x60.
      4. Une fois la petite grille parsée : calculer l'offset de centrage
         (`(100-width)/2`, `(60-height)/2`), construire la grille finale
         100x60 remplie de hors-limites, y copier la petite grille à
         l'offset, et **translater** les positions `tee`/`hole_pos`
         trouvées de ce même offset.
      5. Tests : un trou en petit format se centre correctement (tee/trou
         translatés) ; une taille déclarée trop grande est rejetée ; un
         fichier 100x60 sans `width`/`height` continue de parser à
         l'identique (non-régression).
- [ ] Builder de trous : éditeur visuel intégré au TUI plutôt qu'un
      assistant CLI séparé — le vrai point de friction est de dessiner la
      grille à l'aveugle dans un éditeur de texte, pas de taper 3 lignes de
      métadonnées. Réutilise le rendu existant (`CourseView`) pour peindre
      le terrain en direct ; peut partir d'une grille vierge ou d'un trou
      existant chargé tel quel.
      **Plan (par phases)** :
      1. Nouveau mode/état dans `main.rs` (ex. `BuilderState`), accessible
         depuis le menu (`[E]` pour éditer/créer) ou via un flag CLI simple
         (`divotty edit [fichier]`) — pas de nouveau binaire, cohérent avec
         le choix "un seul crate" pour crates.io.
      2. En-tête simple (pas un vrai wizard) pour `name`/`par` — la
         difficulté reste au niveau `course.yaml`, donc hors du scope d'un
         éditeur de trou unique.
      3. Curseur déplaçable sur la grille (flèches), touche pour cycler le
         type de terrain courant, Espace/Entrée pour peindre la case sous
         le curseur — réutilise `TerrainKind`/`terrain_style` de
         `render.rs`.
      4. Sauvegarde : réutiliser la validation de `Hole::parse` (un seul
         `D`, un seul `H`, dimensions cohérentes) avant d'écrire le
         fichier `.course` — pas de logique de validation dupliquée.
      5. Recouvre une bonne partie du besoin de l'item "Validateur de
         carte" ci-dessous (la sauvegarde de l'éditeur *est* déjà une
         validation) — à réévaluer si un outil de lint séparé reste utile
         une fois le builder construit.
- [ ] Partage de parcours créés avec le builder — pas encore tranché
      comment, mais un fichier `.course`/`course.yaml` est déjà du texte
      brut : le partage "à la main" (copier le fichier dans le dossier
      `courses/` d'un ami) fonctionne déjà aujourd'hui, sans rien coder.
      Paliers possibles, du plus simple au plus ambitieux :
      1. Documenter que c'est déjà possible tel quel (zéro ingénierie).
      2. Action "exporter" dans le builder qui écrit le trou fini à un
         endroit prévisible avec un message clair — toujours zéro réseau.
      3. Code de partage compact (façon Wordle/Baba Is You) : encoder
         grille + métadonnées en une chaîne de texte copiable dans un
         chat, décodable au chargement. Faisable sans serveur ; la grille
         100x60 se compresse bien vu ses longues plages de caractères
         répétés (hors-limites, fairway).
      4. Galerie en ligne partagée (repo communautaire, etc.) —
         déconseillé pour l'instant, infrastructure disproportionnée pour
         le stade actuel du projet ; à réévaluer si le jeu prend de
         l'ampleur.
- [ ] Validateur de carte en outil séparé (`cargo run --bin course-lint`) :
      détecte cases orphelines/inaccessibles, terrain incohérent, dimensions
      invalides — pour sécuriser la création de nouvelles cartes
- [ ] Documentation du format `.course` avec exemples de motifs (dogleg, île, etc.)

## v0.5 — Rendu et interface (fait)

Refonte visuelle des 7 panneaux de la sidebar (gardés tels quels, pas de
réduction — voir `CLAUDE.md`), à partir de notes prises pendant une session
de planification. Chaque item isolé dans `src/tui/`, aucun changement côté
`core`.

- [x] Cadre (bordure) autour du panneau carte (`CourseView`, `render.rs`)
      — le `Viewport` passé depuis `main.rs` est réduit de 2 (largeur et
      hauteur) pour compenser la bordure.
- [x] Carte centrée dans le panneau plutôt que collée en haut à gauche
      quand la grille tient entièrement dans l'espace disponible (terminal
      large) : marge calculée à partir de `grid_w/h` vs `inner.width/
      height` dans `render()`. Suit toujours la balle sans marge quand la
      grille dépasse l'écran (comportement inchangé dans ce cas).
- [x] Mieux identifier tee et trou sur la carte : trou en drapeau ⛳,
      tee en `D` recoloré `LightCyan` (distinct de tout le reste de la
      palette de terrain) — `terrain_style()` dans `render.rs`.
- [x] Fairway plus terne (`Rgb(0,90,0)`) / éléments de visée plus intenses
      (guide en blanc gras, halo en `LightMagenta` gras, repère
      d'atterrissage en `LightYellow` gras) — `terrain_style()`/`render()`
      dans `render.rs`.
- [x] Green (`,` → `O`) en vert vif (`Rgb(0,220,0)`), bien distinct du
      rough (`LightGreen`) — `terrain_style()` dans `render.rs`.
- [x] Panneau Titre : version affichée (`env!("CARGO_PKG_VERSION")`) —
      `sidebar.rs`.
- [x] Panneau Contrôles justifié en bas à gauche — nouvelle fonction
      `panel_bottom_aligned()` dans `sidebar.rs` (préfixe le texte de
      lignes vides calculées depuis la hauteur du panneau).

## v1.0 — Polish
- [x] Sauvegarde/reprise de partie (un seul emplacement, `save.yaml` :
      parcours, trou courant, coups, position de balle, club, visée)
- [x] Interface multilingue (anglais par défaut, français en bascule avec
      `L`) — `core` reste sémantique, `tui` traduit ; architecture prête
      pour d'autres langues
- [x] Confirmation de sortie (double `q`) pour éviter les sorties
      accidentelles, au menu comme en jeu
- [ ] Sons/feedback terminal (bell, ou intégration `cpal` optionnelle)
- [ ] Thème de couleurs configurable
- [ ] Parcours à 18 trous complet et équilibré
- [x] Publication sur crates.io — `divotty` v0.1.0 en ligne,
      `cargo install divotty` fonctionnel (voir `CLAUDE.md`)

## Idées non priorisées
- Mode multijoueur local (tour par tour, même terminal)
- Import de cartes depuis un format tiers
- Statistiques de progression (moyenne de coups par trou sur plusieurs parties)
- Sauvegardes multiples (emplacements nommés) plutôt qu'un seul `save.yaml`
- Événements aléatoires rares et fun (monstre, catastrophe...) : très
  occasionnellement (ex. 1 coup sur 100-200), un effet exceptionnel sur la
  balle ou le terrain — un glissement de terrain crée un bunker, un
  monstre déplace la balle, la pluie inonde une zone, etc. Objectif pur
  amusement/surprise, pas une couche de difficulté de plus.
  - Architecture pressentie : `GameState` possède déjà `hole: Hole` et
    `ball: Pos` par valeur (pas de référence), donc déplacer la balle ou
    muter une case de terrain entre deux coups ne demande aucun
    changement du moteur (`resolve_shot` reste intact) — juste une
    mutation d'état côté `app`.
  - Un enum sémantique `core::RandomEvent` (comme `ScoreLabel`) pour
    rester traduisible par `tui`, mais le tirage aléatoire et la mutation
    d'état se font côté `app` (`main.rs`), jamais dans `core`.
  - Mélanger des effets positifs/négatifs/neutres-drôles plutôt que
    purement punitifs, pour rester dans l'esprit "surprise fun" plutôt que
    "pénalité aléatoire de plus" (à l'opposé de la direction prise avec le
    putting/le vent, qui visent à *réduire* le RNG pur).
  - Réutiliser le panneau "Dernier coup" pour le message plutôt qu'un
    nouveau panneau.
  - Le "positif" du mélange positif/négatif/neutre ci-dessus peut prendre
    la forme d'un boost temporaire (ex: dispersion divisée par deux sur le
    prochain coup) plutôt qu'un simple message flatteur sans effet — un
    seul type de boost simple pour commencer plutôt qu'un système à
    plusieurs boosts dès le départ.
- Récompense (boost) pour un score sous le par sur un trou (birdie ou
  mieux) : contrairement au boost d'événement aléatoire ci-dessus, celui-ci
  récompenserait le skill plutôt que la chance — cohérent avec la
  direction déjà prise avec le vent/le putting (réduire le RNG pur, valoriser
  la compétence). **Dépendance** : n'a de sens que si le boost s'applique
  au trou suivant, donc attend l'enchaînement multi-trous (v0.2,
  "Enchaînement automatique des trous") — pas de vrai "trou suivant"
  aujourd'hui, `GameState` ne joue que `holes[0]`. À implémenter avec ou
  après cet item plutôt qu'avant.
