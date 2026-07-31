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

## v0.2 — Parcours complets (fait)
- [x] Enchaînement automatique des trous d'un `Course` (1 → 9 → 18) —
      `GameState` garde désormais tous les trous (`holes: Vec<Hole>` +
      `current_hole()`) au lieu de ne jouer que `holes[0]`. Touche `N`
      (trou suivant) disponible dans le panneau Contrôles une fois le trou
      terminé, uniquement s'il en reste un (`GameState::advance_hole`,
      testé unitairement). `save.yaml` persiste désormais le `Scorecard`
      accumulé (reprendre une partie multi-trous ne perd plus les trous
      déjà joués).
- [x] Carte de score complète (`Scorecard`) affichée entre les trous et en fin de partie —
      le panneau Score affiche désormais un total cumulé ("Total: N (±M)")
      dès qu'un parcours a plus d'un trou, en plus du score du trou en
      cours (`SidebarState::scorecard`, `src/tui/sidebar.rs`). Sur le
      dernier trou, `Entrée` ("finir la partie") enregistre le score final
      et bascule vers un nouvel écran plein cadre (`tui::ScorecardView`,
      `src/tui/scorecard.rs`) : détail trou par trou (nom, par, coups,
      label) + total, avant retour au menu. Ce nouvel écran s'affiche pour
      *tout* parcours, y compris à un seul trou (le trou de démo inclus) —
      changement de comportement volontaire pour rester cohérent plutôt que
      de réserver le résumé aux parcours multi-trous.
- [x] Écran de fin de trou (résumé du score, label Birdie/Bogey/etc.) — pas
      un écran séparé : dès `holed`, les panneaux existants (Score, Dernier
      coup) montrent déjà le résumé, et le panneau Contrôles bascule sur
      `R` rejouer / `M` menu / `qq` quitter (jouer/viser/sauvegarder sont
      bloqués). `GameState::finished()`, `run_loop` renvoie un `LoopExit`
      (`Quit`/`BackToMenu`) plutôt que de juste `break`. Rejouer un
      parcours depuis le menu clone la `Course` au lieu de la retirer de
      la liste, pour pouvoir y revenir plusieurs fois dans la même session.
- [x] Menu de sélection de parcours (lecture de `courses/*/course.yaml`)
- [x] Difficulté par parcours (1 à 4 étoiles, champ `difficulty` dans
      `course.yaml`, purement indicative)
- [x] Panneau "Score" façon carte de golf : n° de trou, par du trou, score
      courant, adapté au nombre de trous du parcours (1/3/9/18) — fait en
      même temps que l'écran de scorecard complet ci-dessus (même dépendance
      sur l'enchaînement multi-trous + `Scorecard`) : total cumulé affiché
      dès que `hole_count > 1`, cf. item "Carte de score complète".
- [x] Parcours rapide à 3 trous (`courses/quick3/`, difficulté 2 étoiles,
      par 3/4/5 pour 12 au total) : bon format pour tester l'enchaînement
      multi-trous sans attendre un 9 trous complet. Trois orientations/
      distances distinctes (par 3 vertical avec carry d'eau, par 4
      horizontal avec cluster de bunkers + zone à pénalité, par 5 horizontal
      avec double obstacle eau+bunker et un cluster d'arbres) — bon candidat
      aussi pour exercer le futur format de grille taille variable (trous
      courts = grilles compactes, voir v0.4), pas encore fait ici (toujours
      100x60 plein comme le trou de démo).
- [x] Parcours réels accessibles après un `cargo install` : `courses/demo/`
      et `courses/quick3/` sont désormais embarqués dans le binaire à la
      compilation (`include_str!`, `embedded_courses()` dans `main.rs`,
      nouveau `Course::from_embedded` dans `core::course`), utilisés en
      repli si le dossier `courses/` est absent sur disque — auparavant un
      unique trou générique (`fallback_course()`, gardé mais réservé aux
      tests). Sans ce correctif, un joueur `cargo install` sans le dépôt
      cloné à côté n'aurait jamais vu l'enchaînement multi-trous (v0.2),
      la fonctionnalité phare de cette version — corrigé avant publication
      de 0.2.0. Testé (`embedded_courses_parse_and_match_the_real_courses_on_disk`
      dans `main.rs`, `from_embedded_*` dans `core::course`).

## v0.3 — Visée et feedback
- [x] Vent (direction + force), tiré aléatoirement au chargement du trou
      (`random_wind()` dans `main.rs`, jamais `thread_rng()` dans `core`) et
      appliqué comme une **dérive directionnelle** (pas juste plus de
      dispersion aléatoire) proportionnelle à la distance du coup — un
      putt n'est jamais affecté (balle au sol). Affiché dans le panneau
      Visée, à côté de la boussole de visée du joueur — pas de 8e panneau —
      sous forme d'un libellé Calme/Modéré/Fort (coloré vert/jaune/rouge,
      seuils partagés via `wind_tier()` dans `sidebar.rs`) plutôt qu'un
      chiffre brut de force de dérive, plus parlant pour un joueur qui n'a
      pas besoin de connaître l'unité exacte. Pris en compte par
      `resolve_shot` (le vrai coup), mais **pas** par
      `preview_shot` (l'aperçu) : choix délibéré, revu après coup — corriger
      automatiquement l'aperçu pour le vent reviendrait à faire l'adaptation
      à la place du joueur, alors que lire le vent affiché et compenser à la
      visée est justement la compétence que le vent est censé demander.
      L'aperçu ignore aussi la dispersion aléatoire réelle : c'est un guide
      de visée, pas une prédiction exacte.
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
- [x] Visée lisible sur un très court putt (1-2 cases) : en zoom, la balle
      et le trou n'occupent plus qu'une seule sous-case du bloc zoomé
      (au lieu de le remplir en entier) — sur un putt de 1-2 cases, deux
      blocs pleins 3x3 ne laissaient plus de place pour voir quoi que ce
      soit entre les deux. Le reste du bloc de la balle montre le terrain
      réel en dessous ; celui du trou est désormais un cadre en caractères
      box-drawing à coins arrondis (`╭─╮│⚑│╰─╯`, `hole_frame_glyph()` dans
      `render.rs` — mêmes glyphes que `BorderType::Rounded` utilisé partout
      ailleurs dans l'interface) plutôt qu'un simple remplissage vert, pour
      que la cible ressorte quel que soit le terrain autour (vert, rough...).
      Une flèche de visée (`format::compass_arrow`) apparaît dans le bloc
      de la balle, dans le secteur de la direction visée — dérivée de
      l'angle de visée brut (`ShotPreview::direction`, nouveau champ) plutôt
      que de `expected_landing`/`max_landing` arrondis, qui peuvent
      coïncider avec la case de départ sur un tout petit putt et perdre
      l'info de direction.
      Généralisé ensuite à toute la surimpression d'aperçu (guide
      pointillé, halo de dispersion, repère d'atterrissage) : en zoom, ces
      marqueurs n'occupent plus eux non plus que la sous-case centrale du
      bloc (au lieu de le remplir en entier) — un point de guide devenait
      un bloc plein `···` au lieu d'un pointillé, pareil pour le halo et le
      repère. Le reste du bloc montre systématiquement le terrain réel.
      Chaque point de guide s'affiche maintenant comme un petit segment de
      3 points (horizontal, vertical ou diagonal selon le secteur de
      boussole de la direction visée, `guide_segment_offsets()` dans
      `render.rs`) plutôt qu'un point isolé au centre — les segments de
      cases voisines se rejoignent pile (le point de bord d'une case
      touche le point de bord de la suivante), donnant une ligne continue
      pour une trajectoire horizontale/verticale/diagonale à 45°, sans
      avoir à échantillonner `sample_line` à la résolution du zoom.
      Seul le point de guide en profite (pas le halo, ni le repère
      d'atterrissage, qui restent des points isolés — pas des segments de
      ligne). Le guide (`sample_line`, résolution en cases entières) trace
      toujours au plus un point par case entière, donc 0-1 case marquée sur
      un putt de 1-2 cases — mais cette case unique se lit désormais comme
      un vrai tronçon de trajectoire plutôt qu'un point isolé.
- [x] Force du coup réglable par le joueur (`+`/`-`, 3 à 6, 6 = pleine
      puissance) : le tirage reste uniforme mais borné par le plafond
      choisi (`gen_range(1..=die_strength)`), pour éviter qu'un gros dé
      n'envoie la balle bien au-delà d'un green proche. Nommé "Force du
      coup"/"Shot power" côté affichage plutôt que "plafond de dé" — le
      joueur règle sa puissance de frappe, la mécanique de plafonnement du
      dé reste un détail interne (les identifiants de code, eux,
      restent `die_strength`/`die_cap`, cohérent avec "core reste
      sémantique, tui traduit"). Plancher à 3 plutôt qu'1
      (`DIE_STRENGTH_FLOOR`) : en dessous, même un Putter (portée die*0.5)
      ne parcourrait presque plus rien, rendant le coup inutile plutôt que
      "sûr". Affiché dans le panneau Club comme un curseur à 4 crans
      (`+---` à `---+`, `format::die_cap_bar`) plutôt qu'un texte "N/6",
      coloré en jaune si <6 ; l'aperçu de coup (`preview_shot`) prend désormais
      `die_strength` en paramètre pour que le guide/halo reflète le plafond
      courant plutôt qu'un 6 fixe. Remis à 6 à chaque nouveau trou et à
      chaque changement de club (choix délibéré : réglage fin pour le
      club/trou courant, pas une préférence durable) — voir
      `GameState::adjust_die_strength` dans `main.rs`.
- [x] Rendre le putting moins aléatoire — dispersion du Putter désormais
      fonction de la distance restante au trou plutôt qu'une constante fixe
      (`putter_base_dispersion()` dans `shot.rs`) : un putt court (bien
      placé par un bon coup d'approche) devient quasi automatique, comme un
      "gimme" au golf réel, tandis qu'un long putt (>`PUTTER_PRECISION_RANGE`,
      10 cases — nettement au-delà de la portée max d'un seul coup de
      Putter, 3.0 cases) garde son plein risque. Récompense la qualité de
      l'approche plutôt qu'un tirage indépendant de la situation, cohérent
      avec la direction déjà prise (vent, force du coup). Seul le Putter est
      concerné ; les autres clubs gardent une dispersion fixe (testé).
      Piste écartée pour l'instant : un vrai mini-mécanisme d'input dédié
      (jauge de précision, etc.) — trop de complexité ajoutée pour un seul
      club, alors que ce changement reste une petite formule sur la
      dispersion existante.
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
      6. Estimation des longueurs de coup pendant l'édition, pour placer
         bunkers/rough/eau/zones à pénalité à bon escient et doser la
         difficulté du trou (les distances de club viennent d'être
         recalibrées, voir plus haut — un Driver moyen ≈28 cases, etc.).
         Réutiliser directement `core::preview_shot`/`ShotPreview` (le même
         mécanisme qui affiche le guide/halo en jeu) depuis la position du
         curseur du builder, plutôt que redévelopper un calcul de distance
         séparé côté édition.
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
- [x] Modernisation des 7 panneaux de la sidebar : bordures arrondies
      (`BorderType::Rounded`), une couleur d'accent distincte par panneau
      (Titre blanc, Trou vert, Score jaune, Club bleu clair, Dernier coup
      magenta clair, Visée cyan, Contrôles gris terne), et surtout du
      contenu réactif à l'état du jeu plutôt qu'une couleur statique :
      - Score : doré (Albatros/Eagle), vert (Birdie), neutre (Par), orange
        (Bogey), rouge (Double bogey et pire) — `score_color()`.
      - Dernier coup : vert si dans le trou, rouge si pénalité, cyan si
        sauvegarde confirmée, neutre sinon.
      - Vent : vert calme / jaune modéré / rouge fort — `wind_color()`.
      `panel()` prend désormais des lignes (`Vec<Line>`) individuellement
      stylées plutôt qu'une seule chaîne, pour que seule l'info pertinente
      réagisse en couleur/gras sans teinter tout le panneau.

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
- [x] Publication sur crates.io — `divotty` v0.2.0 en ligne,
      `cargo install divotty` fonctionnel, y compris les vrais parcours
      embarqués (voir `CLAUDE.md`)

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
  - Le "négatif" symétrique pourrait être l'événement "tu casses ton
    club" : le club utilisé au moment de l'événement devient indisponible
    (`cycle_club`/`cycle_club_reverse` le sautent) pour le reste du
    **parcours**, pas juste le trou — cohérent avec le côté "rare et
    marquant" recherché. Points à trancher avant d'implémenter :
    - Sévérité : perdre le Driver est gênant mais jouable autrement ;
      perdre le Putter casserait probablement la capacité à finir
      proprement un trou — possiblement à exclure du tirage, ou à limiter
      la casse aux longs clubs (Driver/Bois/Hybride).
    - Portée réelle du mot "parcours" du joueur : dure jusqu'à la fin de
      la partie en cours (tous les trous restants), remis à neuf au
      prochain parcours choisi au menu — pas persistant dans `save.yaml`
      au-delà de cette partie a priori, à confirmer.
    - Implémentation pressentie : un nouveau champ `GameState` (ex.
      `broken_clubs: HashSet<Club>`), initialisé vide dans `GameState::new`
      et **jamais** réinitialisé par `restart_hole`/`advance_hole`
      (contrairement à `die_strength`) puisque l'effet doit justement
      survivre au changement de trou.
- Récompense (boost) pour un score sous le par sur un trou (birdie ou
  mieux) : contrairement au boost d'événement aléatoire ci-dessus, celui-ci
  récompenserait le skill plutôt que la chance — cohérent avec la
  direction déjà prise avec le vent/le putting (réduire le RNG pur, valoriser
  la compétence). L'enchaînement multi-trous (v0.2, fait) débloque
  maintenant cet item : `GameState::advance_hole` est l'endroit naturel où
  appliquer un boost au trou suivant (ex. dispersion réduite sur le
  premier coup) si le trou qu'on vient de quitter était sous le par.
- Types de boost envisagés (primitives réutilisables pour les deux idées
  de récompense ci-dessus — événement aléatoire et bon score) :
  - **Boost distance** : ajoute 1 ou 2 directement au tirage du dé, sans
    plafond — peut donc dépasser 6 et faire mieux que la meilleure portée
    normalement possible. Volontairement plus simple qu'un multiplicateur
    de distance : une seule addition sur `die_roll` avant l'appel à
    `Club::base_distance`.
  - **Boost précision** : réduit la dispersion effective du prochain coup
    (ex. divisée par deux, déjà évoqué plus haut pour l'idée événement
    aléatoire).
  - Point à trancher le moment venu : le boost distance doit-il ignorer le
    plafond de dé choisi par le joueur (`GameState::die_strength`, voir
    v0.3 "Force du coup") ou s'additionner puis être re-plafonné ? Les deux
    mécanismes touchent le même tirage de dé et n'ont pas encore été pensés
    ensemble.
