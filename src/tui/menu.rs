use crate::core::Course;
use crate::tui::format::stars;
use crate::tui::lang::Lang;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

/// Écran de sélection de parcours : liste chaque parcours trouvé avec son
/// nom, sa difficulté (étoiles) et son par total, la sélection courante
/// étant surlignée.
pub struct CourseMenuState<'a> {
    pub lang: Lang,
    pub courses: &'a [&'a Course],
    pub selected: usize,
    /// Une sauvegarde existe : affiche le raccourci pour la reprendre.
    pub has_save: bool,
    /// Deuxième pression sur q en attente de confirmation (voir `qq`).
    pub quit_confirm: bool,
}

fn hole_word(count: usize, lang: Lang) -> &'static str {
    match (count, lang) {
        (1, Lang::En) => "hole",
        (_, Lang::En) => "holes",
        (1, Lang::Fr) => "trou",
        (_, Lang::Fr) => "trous",
    }
}

fn write_line(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    if y >= area.y + area.height {
        return;
    }
    for (x, ch) in text.chars().enumerate() {
        if x as u16 >= area.width {
            break;
        }
        buf.get_mut(area.x + x as u16, y).set_char(ch).set_style(style);
    }
}

impl<'a> Widget for CourseMenuState<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "Divotty — select a course",
            Lang::Fr => "Divotty — choisir un parcours",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let mut y = inner.y;
        if self.has_save {
            let resume_line = match self.lang {
                Lang::En => "[C] Resume saved game",
                Lang::Fr => "[C] Reprendre la partie sauvegardée",
            };
            write_line(buf, inner, y, resume_line, Style::default().fg(Color::Cyan));
            y += 2;
        }

        for (i, course) in self.courses.iter().enumerate() {
            let total_par: u32 = course.holes.iter().map(|h| h.meta.par as u32).sum();
            let line = format!(
                "{}  {}  Par {} · {} {}",
                course.name,
                stars(course.difficulty),
                total_par,
                course.holes.len(),
                hole_word(course.holes.len(), self.lang)
            );
            let (prefix, style) = if i == self.selected {
                ("> ", Style::default().fg(Color::Black).bg(Color::White))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            write_line(buf, inner, y + i as u16, &format!("{prefix}{line}"), style);
        }

        let hint = if self.quit_confirm {
            match self.lang {
                Lang::En => "Press q again to quit",
                Lang::Fr => "Appuyez encore sur q pour quitter",
            }
        } else {
            match self.lang {
                Lang::En => {
                    "↑ ↓ select   Enter play   E new hole   P build course   L language   qq quit"
                }
                Lang::Fr => {
                    "↑ ↓ choisir   Entrée jouer   E nouveau trou   P assembler parcours   L langue   qq quitter"
                }
            }
        };
        let hint_y = inner.y + inner.height.saturating_sub(1);
        write_line(buf, inner, hint_y, hint, Style::default().fg(Color::DarkGray));
    }
}
