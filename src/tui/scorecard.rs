use crate::core::HoleScore;
use crate::tui::format::{format_relative, score_color, score_label_text, stars};
use crate::tui::lang::Lang;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
};

/// Écran de fin de partie : détail trou par trou du parcours qui vient de
/// se terminer, plus le total. S'affiche une fois, entre le dernier trou
/// (`Enter` pour "terminer la partie", voir `GameState::advance_hole` /
/// `run_loop` dans `main.rs`) et le retour au menu de sélection.
pub struct ScorecardView<'a> {
    pub lang: Lang,
    pub course_name: &'a str,
    /// Nom + score de chaque trou joué, dans l'ordre du parcours.
    pub entries: &'a [(String, HoleScore)],
    /// Difficulté du parcours (1 à 4), purement indicative — cohérent avec
    /// l'affichage déjà utilisé au menu et dans le panneau Trou en jeu.
    pub course_difficulty: u8,
    /// Deuxième pression sur q en attente de confirmation (voir `qq`).
    pub quit_confirm: bool,
}

/// Marge intérieure (colonnes/lignes) entre la bordure du panneau et son
/// contenu — sans ça, le texte touchait directement le cadre.
const H_PADDING: u16 = 2;
const V_PADDING: u16 = 1;

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

impl<'a> Widget for ScorecardView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => format!("Divotty — round complete: {}", self.course_name),
            Lang::Fr => format!("Divotty — partie terminée : {}", self.course_name),
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let padded = Rect {
            x: inner.x + H_PADDING,
            y: inner.y + V_PADDING,
            width: inner.width.saturating_sub(H_PADDING * 2),
            height: inner.height.saturating_sub(V_PADDING * 2),
        };

        let header = match self.lang {
            Lang::En => format!("{}  ·  {} holes", stars(self.course_difficulty), self.entries.len()),
            Lang::Fr => format!("{}  ·  {} trous", stars(self.course_difficulty), self.entries.len()),
        };
        write_line(buf, padded, padded.y, &header, Style::default().fg(Color::White));

        let mut y = padded.y + 2;
        let mut total_strokes: u32 = 0;
        let mut total_par: u32 = 0;
        for (i, (name, score)) in self.entries.iter().enumerate() {
            let label = score.label();
            let line = format!(
                "{:>2}. {:<28} Par {}   {} strokes   {} ({})",
                i + 1,
                name,
                score.par,
                score.strokes,
                score_label_text(label, self.lang),
                format_relative(score.relative_to_par() as i32)
            );
            write_line(buf, padded, y, &line, Style::default().fg(score_color(label)));
            total_strokes += score.strokes as u32;
            total_par += score.par as u32;
            y += 1;
        }

        y += 1;
        let total_relative = total_strokes as i32 - total_par as i32;
        let total_label = match self.lang {
            Lang::En => format!(
                "TOTAL: {} strokes, par {} ({})",
                total_strokes,
                total_par,
                format_relative(total_relative)
            ),
            Lang::Fr => format!(
                "TOTAL : {} coups, par {} ({})",
                total_strokes,
                total_par,
                format_relative(total_relative)
            ),
        };
        write_line(buf, padded, y, &total_label, Style::default().add_modifier(ratatui::style::Modifier::BOLD));

        let hint = if self.quit_confirm {
            match self.lang {
                Lang::En => "Press q again to quit",
                Lang::Fr => "Appuyez encore sur q pour quitter",
            }
        } else {
            match self.lang {
                Lang::En => "Enter / M  back to menu   L  language   qq  quit",
                Lang::Fr => "Entrée / M  retour au menu   L  langue   qq  quitter",
            }
        };
        let hint_y = padded.y + padded.height.saturating_sub(1);
        write_line(buf, padded, hint_y, hint, Style::default().fg(Color::DarkGray));
    }
}
