use tenui_core::{CanvasSubviewMut, Modifier};

use crate::theme::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    fn days_in_month(year: i32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    fn weekday_of_first(year: i32, month: u32) -> u32 {
        let y = if month <= 2 { year - 1 } else { year };
        let m = if month <= 2 { month + 12 } else { month };
        let q = 1;
        let k = y % 100;
        let j = y / 100;
        let h = (q + (13 * (m as i32 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
        ((h + 6) % 7) as u32
    }

    fn month_name(month: u32) -> &'static str {
        match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "???",
        }
    }
}

pub struct Calendar {
    pub year: i32,
    pub month: u32,
    pub selected: Option<Date>,
    pub cursor_day: u32,
}

impl Calendar {
    pub fn new(year: i32, month: u32) -> Self {
        Self {
            year,
            month,
            selected: None,
            cursor_day: 1,
        }
    }

    pub fn next_month(&mut self) {
        if self.month == 12 {
            self.month = 1;
            self.year += 1;
        } else {
            self.month += 1;
        }
        self.clamp_cursor();
    }

    pub fn prev_month(&mut self) {
        if self.month == 1 {
            self.month = 12;
            self.year -= 1;
        } else {
            self.month -= 1;
        }
        self.clamp_cursor();
    }

    pub fn select_cursor(&mut self) {
        self.selected = Some(Date::new(self.year, self.month, self.cursor_day));
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let days = Date::days_in_month(self.year, self.month);
        let new = self.cursor_day as i32 + delta;
        self.cursor_day = new.clamp(1, days as i32) as u32;
    }

    pub fn width() -> u16 {
        22
    }

    pub fn height() -> u16 {
        9
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) {
        let w = canvas.width() as usize;

        let header = format!("  {} {}  ", Date::month_name(self.month), self.year);
        let header_pad = (w.saturating_sub(header.len())) / 2;
        for (i, ch) in header.chars().enumerate() {
            let x = header_pad + i;
            if x < w {
                canvas.set_char(x as u16, 0, ch, palette.primary, palette.bg, Modifier::BOLD);
            }
        }

        let day_labels = "Mo Tu We Th Fr Sa Su";
        for (i, ch) in day_labels.chars().enumerate() {
            if i < w {
                canvas.set_char(i as u16, 1, ch, palette.fg_muted, palette.bg, Modifier::empty());
            }
        }

        let first_weekday = Date::weekday_of_first(self.year, self.month);
        let days = Date::days_in_month(self.year, self.month);

        let mut row = 2u16;
        let mut col = first_weekday;

        for day in 1..=days {
            let x = (col * 3) as u16;
            let is_selected = self.selected == Some(Date::new(self.year, self.month, day));
            let is_cursor = day == self.cursor_day;

            let (fg, bg, mods) = if is_selected {
                (palette.bg, palette.primary, Modifier::BOLD)
            } else if is_cursor {
                (palette.fg, palette.surface, Modifier::UNDERLINE)
            } else {
                (palette.fg, palette.bg, Modifier::empty())
            };

            let label = format!("{:>2}", day);
            for (i, ch) in label.chars().enumerate() {
                let draw_x = x + i as u16;
                if (draw_x as usize) < w {
                    canvas.set_char(draw_x, row, ch, fg, bg, mods);
                }
            }

            col += 1;
            if col >= 7 {
                col = 0;
                row += 1;
            }
        }
    }

    fn clamp_cursor(&mut self) {
        let days = Date::days_in_month(self.year, self.month);
        if self.cursor_day > days {
            self.cursor_day = days;
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn calendar_navigation() {
        let mut cal = Calendar::new(2026, 1);
        cal.next_month();
        assert_eq!(cal.month, 2);
        cal.prev_month();
        assert_eq!(cal.month, 1);
        cal.prev_month();
        assert_eq!(cal.month, 12);
        assert_eq!(cal.year, 2025);
    }

    #[test]
    fn calendar_leap_year() {
        assert_eq!(Date::days_in_month(2024, 2), 29);
        assert_eq!(Date::days_in_month(2025, 2), 28);
        assert_eq!(Date::days_in_month(2000, 2), 29);
        assert_eq!(Date::days_in_month(1900, 2), 28);
    }

    #[test]
    fn calendar_select() {
        let mut cal = Calendar::new(2026, 9);
        cal.cursor_day = 15;
        cal.select_cursor();
        assert_eq!(cal.selected, Some(Date::new(2026, 9, 15)));
    }

    #[test]
    fn calendar_renders() {
        let cal = Calendar::new(2026, 9);
        let palette = ThemePalette::catppuccin_mocha();
        let w = Calendar::width();
        let h = Calendar::height();
        let mut buf = Buffer::new(w, h);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, w, h));
        cal.render(&mut canvas, &palette);

        let cell = buf.get(0, 1).unwrap();
        assert_eq!(cell.symbol.as_str(), "M");
    }

    #[test]
    fn calendar_cursor_clamp() {
        let mut cal = Calendar::new(2026, 3);
        cal.cursor_day = 31;
        cal.next_month();
        assert_eq!(cal.cursor_day, 30);
    }
}
