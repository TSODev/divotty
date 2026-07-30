# Divotty

Un jeu de golf en TUI (terminal), écrit en Rust. Dé + choix de club +
trajectoire visée + conditions du parcours (fairway, rough, bunker, eau,
arbres, green) → résolution d'un coup avec dispersion aléatoire.

## Structure du projet (Cargo workspace)

```
divotty/
├── src/
│   ├── core/            # divotty-core : logique pure, zéro dépendance UI
│   │   └── src/
│   │       ├── terrain.rs   # types de terrain + profils de jeu (distance, dispersion, pénalités)
│   │       ├── course.rs    # grille 25x50, parsing des fichiers .course, Course::discover
│   │       ├── shot.rs       # résolution d'un coup (dé + club + terrain) + aperçu (ShotPreview)
│   │       └── scoring.rs    # score par trou / carte de score (labels sémantiques, pas de texte)
│   ├── tui/             # divotty-tui : rendu ratatui
│   │   └── src/
│   │       ├── render.rs      # vue de la carte, viewport qui suit la balle + aperçu de dispersion
│   │       ├── sidebar.rs      # colonne d'info (titre, trou, score, club, dernier coup, visée, contrôles)
│   │       ├── menu.rs          # écran de sélection de parcours
│   │       ├── lang.rs           # langue d'affichage (anglais par défaut, français en bascule)
│   │       └── format.rs          # helpers d'affichage partagés (étoiles de difficulté...)
│   └── app/             # divotty : binaire principal, boucle de jeu
│       └── src/main.rs   # menu → GameState → boucle de jeu, sauvegarde/reprise
└── courses/
    └── demo/            # exemple de parcours (1 trou)
        ├── course.yaml     # index du parcours (nom, difficulté, ordre des trous)
        └── hole_01.course  # un trou : frontmatter YAML + grille ASCII
```

## Lancer le jeu

Depuis la racine du dépôt (le jeu cherche `courses/` et `save.yaml` relatifs
au répertoire courant, pas au crate) :

```sh
cargo run -p divotty
```

Un écran de sélection de parcours s'affiche d'abord (parcours trouvés sous
`courses/`, avec difficulté en étoiles, par total et nombre de trous). Si
une sauvegarde existe (`save.yaml`), l'option `[C]` permet de la reprendre.

Commandes au menu :
- **↑ / ↓** : changer de parcours sélectionné
- **Entrée** : jouer le parcours sélectionné
- **C** : reprendre la sauvegarde (si disponible)
- **L** : changer de langue (anglais / français)
- **qq** : quitter (double appui pour confirmer)

Commandes en jeu :
- **Flèches gauche/droite** : ajuster l'angle de visée
- **Tab** : changer de club (Driver → Wood → Hybrid → Iron → Wedge → Putter)
- **Espace** : jouer le coup (lance le dé)
- **S** : sauvegarder la partie en cours
- **L** : changer de langue
- **qq** : quitter (double appui pour confirmer)

La carte affiche en permanence un aperçu du coup en préparation : guide de
trajectoire en pointillés jusqu'à la portée maximale, halo de dispersion
autour de l'atterrissage moyen, avant même de lancer le dé.

## Format des fichiers `.course`

Un trou = un frontmatter YAML (`name`, `par`, `description` optionnelle) suivi
de `---` puis une grille ASCII de **50 colonnes x 25 lignes** exactement.

Légende des caractères :

| Caractère | Terrain |
|---|---|
| `D` | Départ (tee) — une seule case par trou |
| `H` | Trou (arrivée) — une seule case par trou |
| `.` | Fairway |
| `=` | Rough |
| `B` | Bunker |
| `~` | Eau |
| `T` | Arbre |
| `G` | Green |
| ` ` (espace) | Hors-limites |

Un parcours (1, 9 ou 18 trous) est un dossier contenant un `course.yaml` qui
donne son nom, sa difficulté (1 à 4, purement indicative) et la liste des
fichiers `.course` dans l'ordre de jeu :

```yaml
name: "Mon parcours"
difficulty: 2
holes:
  - hole_01.course
  - hole_02.course
```

Voir `courses/demo/` pour un exemple complet et fonctionnel.

## État actuel

Moteur de résolution de coup testé (dé + club + terrain + dispersion
aléatoire seedable, sensibilité au terrain par club, survol d'obstacles),
parser de carte validé avec tests unitaires, aperçu de portée/dispersion
avant de jouer, interface deux colonnes multilingue (anglais/français),
menu de sélection de parcours avec difficulté affichée, sauvegarde/reprise
de partie. Pas encore d'enchaînement multi-trous ni de carte de score
complète. Voir `ROADMAP.md` pour la suite, et `CLAUDE.md` pour le contexte
de handoff.
