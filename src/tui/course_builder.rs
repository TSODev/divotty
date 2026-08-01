//! Widgets de l'écran d'assemblage de parcours (voir `ROADMAP.md`, "Builder
//! de parcours") : choisir/créer un parcours, en éditer le nom/difficulté,
//! assembler une liste ordonnée de trous en piochant dans la bibliothèque
//! (`courses/*/*.course`). Modèle "bibliothèque + duplication" : ajouter un
//! trou à un parcours copie son fichier `.course` dans le dossier du
//! parcours plutôt que de le référencer par pointeur (voir `CourseHoleEntry`
//! et `CourseBuilderState` dans `main.rs`) — un même trou peut ainsi être
//! réutilisé tel quel dans plusieurs parcours, en toute indépendance.

use crate::core::Course;
use crate::tui::builder::{list_scroll_offset, wrap_text, write_line};
use crate::tui::format::stars;
use crate::tui::lang::Lang;
use crate::tui::sidebar::{panel, panel_bottom_aligned};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Widget},
};
use std::path::{Path, PathBuf};

/// Étape d'interaction courante dans l'écran d'assemblage : liste normale,
/// ou l'une des deux saisies de texte ponctuelles (nom du parcours) qui
/// bloque temporairement les raccourcis à une seule lettre le temps de la
/// saisie — même esprit que `crate::tui::builder::BuilderMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseBuilderMode {
    Listing,
    EditingName,
}

fn hole_word(count: usize, lang: Lang) -> &'static str {
    match (count, lang) {
        (1, Lang::En) => "hole",
        (_, Lang::En) => "holes",
        (1, Lang::Fr) => "trou",
        (_, Lang::Fr) => "trous",
    }
}

fn bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

const COURSE_ACCENT: Color = Color::LightGreen;
const HOLES_ACCENT: Color = Color::Cyan;
const CONTROLS_ACCENT: Color = Color::DarkGray;

/// Écran d'entrée du builder de parcours : "+ Nouveau parcours" en premier,
/// puis chaque parcours existant ayant un dossier sur disque — un parcours
/// embarqué (sans dossier, voir `CLAUDE.md`) n'apparaît pas ici, il n'y a
/// rien à réécrire (voir `pick_course_to_build` dans `main.rs`).
pub struct CoursePickerView<'a> {
    pub lang: Lang,
    pub courses: &'a [(PathBuf, &'a Course)],
    pub selected: usize,
}

impl<'a> Widget for CoursePickerView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "Course builder — choose",
            Lang::Fr => "Éditeur de parcours — choisir",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let new_label = match self.lang {
            Lang::En => "+ New course".to_string(),
            Lang::Fr => "+ Nouveau parcours".to_string(),
        };
        let mut entries: Vec<String> = vec![new_label];
        entries.extend(self.courses.iter().map(|(_, course)| {
            format!(
                "{}  {}  · {} {}",
                course.name,
                stars(course.difficulty),
                course.holes.len(),
                hole_word(course.holes.len(), self.lang)
            )
        }));

        let hint_lines: Vec<&str> = match self.lang {
            Lang::En => vec!["↑ ↓  select", "Enter  choose", "L  language", "Esc  back to menu"],
            Lang::Fr => vec!["↑ ↓  choisir", "Entrée  valider", "L  langue", "Échap  retour menu"],
        };
        let available_rows = (inner.height as usize).saturating_sub(hint_lines.len());
        let offset = list_scroll_offset(self.selected, entries.len(), available_rows);
        let mut y = inner.y;
        for (i, label) in entries.iter().enumerate().skip(offset).take(available_rows) {
            let is_selected = i == self.selected;
            let (prefix, style) = if is_selected {
                ("> ", Style::default().fg(Color::Black).bg(Color::White))
            } else if i == 0 {
                ("  ", Style::default().fg(Color::LightGreen))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            write_line(buf, inner, y, &format!("{prefix}{label}"), style);
            y += 1;
        }

        let start_y = (inner.y + inner.height).saturating_sub(hint_lines.len() as u16);
        for (i, line) in hint_lines.iter().enumerate() {
            write_line(buf, inner, start_y + i as u16, line, Style::default().fg(Color::DarkGray));
        }
    }
}

/// Petit formulaire avant de créer un parcours neuf : nom (saisie directe,
/// lettres tapées vont dans le nom — pas de bascule de langue possible ici,
/// même principe que `BuilderMode::EditingName` dans le builder de trous) et
/// difficulté (1 à 4 étoiles). `None` si annulé (Échap, retour au menu).
pub struct CourseSetupView<'a> {
    pub lang: Lang,
    pub name: &'a str,
    pub difficulty: u8,
}

impl<'a> Widget for CourseSetupView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "New course",
            Lang::Fr => "Nouveau parcours",
        };
        let lines = vec![
            Line::styled(
                match self.lang {
                    Lang::En => format!("Name: {}_", self.name),
                    Lang::Fr => format!("Nom : {}_", self.name),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("Difficulty: {}", stars(self.difficulty)),
                    Lang::Fr => format!("Difficulté : {}", stars(self.difficulty)),
                },
                Style::default().fg(Color::White),
            ),
            Line::from(""),
            Line::styled(
                match self.lang {
                    Lang::En => "Type to name the course",
                    Lang::Fr => "Tapez pour nommer le parcours",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "Up/Down  difficulty",
                    Lang::Fr => "Haut/Bas  difficulté",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "Enter  confirm",
                    Lang::Fr => "Entrée  valider",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "Esc  cancel",
                    Lang::Fr => "Échap  annuler",
                },
                Style::default().fg(Color::Gray),
            ),
        ];
        panel(area, buf, title, COURSE_ACCENT, lines);
    }
}

/// Colonne de gauche de l'écran d'assemblage (voir `run_course_builder` dans
/// `main.rs`) : nom/difficulté du parcours, liste ordonnée des trous
/// (fenêtre défilante comme `HolePickerView`), et contrôles.
pub struct CourseBuilderSidebarView<'a> {
    pub lang: Lang,
    pub name: &'a str,
    pub difficulty: u8,
    /// Nom de fichier + indicateur "pas encore copié sur disque" pour
    /// chaque trou de la liste, dans l'ordre du parcours.
    pub holes: &'a [(String, bool)],
    pub selected: usize,
    pub mode: CourseBuilderMode,
    pub text_input: &'a str,
    pub message: Option<&'a str>,
    pub quit_confirm: bool,
    /// Deuxième pression sur `Échap` en attente (retour au menu) — distinct
    /// de `quit_confirm` (quitter l'application entière via `qq`).
    pub exit_confirm: bool,
}

impl<'a> Widget for CourseBuilderSidebarView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(4), Constraint::Length(13)])
            .split(area);

        let course_title = match self.lang {
            Lang::En => "Course",
            Lang::Fr => "Parcours",
        };
        let display_name: &str = if self.name.trim().is_empty() {
            match self.lang {
                Lang::En => "(unnamed)",
                Lang::Fr => "(sans nom)",
            }
        } else {
            self.name
        };
        let course_lines = vec![
            Line::styled(
                match self.lang {
                    Lang::En => format!("Name: {display_name}"),
                    Lang::Fr => format!("Nom : {display_name}"),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("Difficulty: {}", stars(self.difficulty)),
                    Lang::Fr => format!("Difficulté : {}", stars(self.difficulty)),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("{} {}", self.holes.len(), hole_word(self.holes.len(), self.lang)),
                    Lang::Fr => format!("{} {}", self.holes.len(), hole_word(self.holes.len(), self.lang)),
                },
                Style::default().fg(Color::White),
            ),
        ];
        panel(chunks[0], buf, course_title, COURSE_ACCENT, course_lines);

        let holes_title = match self.lang {
            Lang::En => "Holes",
            Lang::Fr => "Trous",
        };
        let holes_block = Block::default().borders(Borders::ALL);
        let holes_inner_height = holes_block.inner(chunks[1]).height as usize;
        let holes_lines: Vec<Line<'static>> = if self.holes.is_empty() {
            vec![Line::styled(
                match self.lang {
                    Lang::En => "No holes yet — press A",
                    Lang::Fr => "Aucun trou — touche A",
                },
                Style::default().fg(Color::DarkGray),
            )]
        } else {
            let offset = list_scroll_offset(self.selected, self.holes.len(), holes_inner_height);
            self.holes
                .iter()
                .enumerate()
                .skip(offset)
                .take(holes_inner_height)
                .map(|(i, (filename, pending))| {
                    let tag = if *pending {
                        match self.lang {
                            Lang::En => " (new)",
                            Lang::Fr => " (nouveau)",
                        }
                    } else {
                        ""
                    };
                    let label = format!("{}. {filename}{tag}", i + 1);
                    let style = if i == self.selected {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else if *pending {
                        Style::default().fg(Color::LightYellow)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::styled(label, style)
                })
                .collect()
        };
        panel(chunks[1], buf, holes_title, HOLES_ACCENT, holes_lines);

        let controls_title = match self.lang {
            Lang::En => "Controls",
            Lang::Fr => "Contrôles",
        };
        let mut controls_lines: Vec<Line<'static>> = Vec::new();
        if let Some(message) = self.message {
            for line in wrap_text(message, 24) {
                controls_lines.push(Line::styled(line, bold(Color::LightRed)));
            }
        }
        if self.quit_confirm {
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "Press q again",
                    Lang::Fr => "Appuyez encore",
                },
                bold(Color::Red),
            ));
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "to quit (unsaved)",
                    Lang::Fr => "sur q pour quitter",
                },
                bold(Color::Red),
            ));
        } else if self.exit_confirm {
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "Esc again: discard",
                    Lang::Fr => "Échap : abandonner",
                },
                bold(Color::LightYellow),
            ));
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "S: save first",
                    Lang::Fr => "S : sauver d'abord",
                },
                bold(Color::LightYellow),
            ));
        } else if self.mode == CourseBuilderMode::EditingName {
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "New name:",
                    Lang::Fr => "Nouveau nom :",
                },
                Style::default().fg(Color::Gray),
            ));
            controls_lines.push(Line::styled(
                format!("{}_", self.text_input),
                Style::default().fg(Color::White),
            ));
        }
        let base: &str = if self.quit_confirm || self.exit_confirm {
            ""
        } else {
            match self.mode {
                CourseBuilderMode::Listing => match self.lang {
                    Lang::En => {
                        "↑↓  select hole\n←→  difficulty\nA  add hole\nX  remove\n\
                         [ ]  reorder\nN  rename\nS  save\nEsc Esc  menu\nqq  quit"
                    }
                    Lang::Fr => {
                        "↑↓  choisir trou\n←→  difficulté\nA  ajouter\nX  retirer\n\
                         [ ]  réordonner\nN  renommer\nS  sauver\nÉchap Échap  menu\nqq  quitter"
                    }
                },
                CourseBuilderMode::EditingName => match self.lang {
                    Lang::En => "Enter  confirm\nEsc  cancel",
                    Lang::Fr => "Entrée  valider\nÉchap  annuler",
                },
            }
        };
        controls_lines.extend(
            base.lines()
                .map(|l| Line::styled(l.to_string(), Style::default().fg(Color::Gray))),
        );
        panel_bottom_aligned(chunks[2], buf, controls_title, CONTROLS_ACCENT, controls_lines);
    }
}

/// Écran de la sous-étape "Ajouter un trou" : liste tous les fichiers
/// `.course` trouvés sous `courses/*/` (bibliothèque comprise), sans entrée
/// "+ Nouveau trou" ni confirmation modifier/dupliquer — choisir une entrée
/// l'ajoute directement au parcours en cours (toujours une duplication, voir
/// le module). Contrairement à `HolePickerView`, dont ces deux différences
/// ne sont pas de simples options d'affichage mais un comportement distinct
/// (pas de mode "modifier sur place" a de sens depuis cet écran).
pub struct HoleAddPickerView<'a> {
    pub lang: Lang,
    pub files: &'a [PathBuf],
    pub selected: usize,
    pub courses_root: &'a Path,
}

impl<'a> Widget for HoleAddPickerView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "Add a hole",
            Lang::Fr => "Ajouter un trou",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        if self.files.is_empty() {
            let msg = match self.lang {
                Lang::En => "No holes found — build one first.",
                Lang::Fr => "Aucun trou trouvé — créez-en un d'abord.",
            };
            write_line(buf, inner, inner.y, msg, Style::default().fg(Color::DarkGray));
            let hint = match self.lang {
                Lang::En => "Esc  back",
                Lang::Fr => "Échap  retour",
            };
            write_line(
                buf,
                inner,
                inner.y + inner.height.saturating_sub(1),
                hint,
                Style::default().fg(Color::DarkGray),
            );
            return;
        }

        let entries: Vec<String> = self
            .files
            .iter()
            .map(|path| {
                path.strip_prefix(self.courses_root)
                    .unwrap_or(path)
                    .display()
                    .to_string()
            })
            .collect();

        let hint_lines: Vec<&str> = match self.lang {
            Lang::En => vec!["↑ ↓  select", "Enter  add", "L  language", "Esc  cancel"],
            Lang::Fr => vec!["↑ ↓  choisir", "Entrée  ajouter", "L  langue", "Échap  annuler"],
        };
        let available_rows = (inner.height as usize).saturating_sub(hint_lines.len());
        let offset = list_scroll_offset(self.selected, entries.len(), available_rows);
        let mut y = inner.y;
        for (i, label) in entries.iter().enumerate().skip(offset).take(available_rows) {
            let is_selected = i == self.selected;
            let (prefix, style) = if is_selected {
                ("> ", Style::default().fg(Color::Black).bg(Color::White))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            write_line(buf, inner, y, &format!("{prefix}{label}"), style);
            y += 1;
        }

        let start_y = (inner.y + inner.height).saturating_sub(hint_lines.len() as u16);
        for (i, line) in hint_lines.iter().enumerate() {
            write_line(buf, inner, start_y + i as u16, line, Style::default().fg(Color::DarkGray));
        }
    }
}
