# ROADMAP — Divotty

## v0.1 — Squelette jouable (fait)
- [x] Structure du projet en modules (`core` / `tui`) au sein d'un crate
      binaire unique `divotty` (a été un workspace à 3 crates, fusionné pour
      simplifier la publication crates.io — voir `CLAUDE.md`)
- [x] Types de terrain + profils de jeu (distance, dispersion, pénalités)
- [x] Format `.course` (frontmatter YAML + grille ASCII 50x25) + parser validé
- [x] Moteur de résolution de coup (dé + club + terrain + dispersion aléatoire seedable)
- [x] Rendu TUI avec viewport suivant la balle + HUD basique
- [x] Boucle de jeu sur un trou unique
- [x] Clubs étendus (Driver, Wood, Hybrid, Iron, Wedge, Putter) avec une
      sensibilité au terrain par club (un terrain difficile pénalise
      proportionnellement plus un club long qu'un club court)
- [x] Interface deux colonnes (sidebar multi-panneaux + carte) — remplace le
      HUD une ligne initial

## v0.2 — Parcours complets
- [ ] Enchaînement automatique des trous d'un `Course` (1 → 9 → 18) —
      `GameState` suit déjà `hole_index`/`hole_count`, prêt pour cette
      extension, mais ne joue toujours que `holes[0]`
- [ ] Carte de score complète (`Scorecard`) affichée entre les trous et en fin de partie
- [ ] Écran de fin de trou (résumé du score, label Birdie/Bogey/etc.)
- [x] Menu de sélection de parcours (lecture de `courses/*/course.yaml`)
- [x] Difficulté par parcours (1 à 4 étoiles, champ `difficulty` dans
      `course.yaml`, purement indicative)

## v0.3 — Visée et feedback
- [x] Aperçu visuel de la zone de dispersion avant de jouer (guide de
      trajectoire en pointillés + halo de dispersion + repère d'atterrissage
      moyen, superposés sur la carte)
- [x] Survol d'obstacles : un arbre proche du départ bloque le coup, un
      arbre plus loin sur la trajectoire est survolé (zone basse de vol,
      voir `LOW_ALTITUDE_FRACTION` dans `shot.rs`) ; l'eau ne bloque jamais
      la trajectoire, seul l'atterrissage compte
- [ ] Historique des coups joués sur le trou courant, affiché dans le HUD
- [ ] Amélioration du drop : reprise au dernier point de fairway valide le long
      de la trajectoire plutôt qu'un retour pur à la position de départ
- [ ] Animation simple du déplacement de la balle (interpolation position par position)

## v0.4 — Contenu et outillage
- [ ] Au moins un parcours complet à 9 trous, dessiné et testé
- [ ] Validateur de carte en outil séparé (`cargo run --bin course-lint`) :
      détecte cases orphelines/inaccessibles, terrain incohérent, dimensions
      invalides — pour sécuriser la création de nouvelles cartes
- [ ] Documentation du format `.course` avec exemples de motifs (dogleg, île, etc.)

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
