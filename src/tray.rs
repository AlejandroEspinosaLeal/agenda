use std::os::windows::process::CommandExt;
use std::process::Command;

use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Actions triggered by clicking the system tray icon or its context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenMini,
    OpenFull,
    NotifyToday,
    Quit,
}

/// Holds the Windows system tray icon and its menu item IDs.
pub struct TrayController {
    tray: TrayIcon,
    id_mini: tray_icon::menu::MenuId,
    id_full: tray_icon::menu::MenuId,
    id_notify: tray_icon::menu::MenuId,
    id_quit: tray_icon::menu::MenuId,
    last_tooltip: String,
    last_count: usize,
}

impl TrayController {
    /// Creates the system tray icon next to the Windows clock.
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();
        let item_mini = MenuItem::new("Mini-Calendario (junto al reloj)", true, None);
        let item_full = MenuItem::new("Abrir Mi Agenda (Completa)", true, None);
        let item_notify = MenuItem::new("Notificación de hoy", true, None);
        let item_quit = MenuItem::new("Salir", true, None);

        let id_mini = item_mini.id().clone();
        let id_full = item_full.id().clone();
        let id_notify = item_notify.id().clone();
        let id_quit = item_quit.id().clone();

        menu.append(&item_mini).map_err(|e| e.to_string())?;
        menu.append(&item_full).map_err(|e| e.to_string())?;
        menu.append(&item_notify).map_err(|e| e.to_string())?;
        menu.append(&item_quit).map_err(|e| e.to_string())?;

        let rgba = build_tray_rgba(32, 0);
        let icon = Icon::from_rgba(rgba, 32, 32).map_err(|e| e.to_string())?;
        let tooltip = "Mi Agenda · Sin tareas pendientes hoy".to_owned();

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(&tooltip)
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            tray,
            id_mini,
            id_full,
            id_notify,
            id_quit,
            last_tooltip: tooltip,
            last_count: 0,
        })
    }

    /// Updates the tray icon badge and hover tooltip with today's pending tasks.
    pub fn sync_today(&mut self, pending_titles: &[String]) {
        let count = pending_titles.len();
        let tooltip = format_tooltip(pending_titles);

        if count != self.last_count {
            let rgba = build_tray_rgba(32, count);
            if let Ok(icon) = Icon::from_rgba(rgba, 32, 32) {
                let _ = self.tray.set_icon(Some(icon));
            }
            self.last_count = count;
        }

        if tooltip != self.last_tooltip {
            let _ = self.tray.set_tooltip(Some(&tooltip));
            self.last_tooltip = tooltip;
        }
    }

    /// Polls pending tray icon clicks and menu events without blocking.
    pub fn poll_action(&self) -> Option<TrayAction> {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                return Some(TrayAction::OpenMini);
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.id_mini {
                return Some(TrayAction::OpenMini);
            }
            if event.id == self.id_full {
                return Some(TrayAction::OpenFull);
            }
            if event.id == self.id_notify {
                return Some(TrayAction::NotifyToday);
            }
            if event.id == self.id_quit {
                return Some(TrayAction::Quit);
            }
        }

        None
    }
}

fn format_tooltip(pending_titles: &[String]) -> String {
    if pending_titles.is_empty() {
        return "Mi Agenda · Todo al día hoy".to_owned();
    }
    let mut out = format!("Hoy ({} pend.):", pending_titles.len());
    for title in pending_titles.iter().take(3) {
        let short: String = title.chars().take(28).collect();
        let line = format!("\n• {short}");
        if out.chars().count() + line.chars().count() > 118 {
            break;
        }
        out.push_str(&line);
    }
    if pending_titles.len() > 3 {
        let extra = format!("\n+{} más", pending_titles.len() - 3);
        if out.chars().count() + extra.chars().count() <= 125 {
            out.push_str(&extra);
        }
    }
    out
}

fn build_tray_rgba(size: u32, badge_count: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    let max = (size - 1) as f32;
    for y in 0..size {
        let ny = y as f32 / max;
        for x in 0..size {
            let nx = x as f32 / max;
            let dx = (nx - 0.5).abs() * 2.0;
            let dy = (ny - 0.5).abs() * 2.0;
            let r_corner = 0.38_f32;
            let qx = (dx - (1.0 - r_corner)).max(0.0);
            let qy = (dy - (1.0 - r_corner)).max(0.0);
            if (qx * qx + qy * qy).sqrt() > r_corner {
                out.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let in_card = (0.18..=0.82).contains(&nx) && (0.22..=0.82).contains(&ny);
            if in_card && ny <= 0.36 {
                out.extend_from_slice(&[55, 42, 191, 255]);
            } else if in_card {
                if (0.42..=0.62).contains(&nx) && (0.46..=0.66).contains(&ny) {
                    out.extend_from_slice(&[185, 5, 56, 255]);
                } else {
                    out.extend_from_slice(&[255, 255, 255, 255]);
                }
            } else {
                out.extend_from_slice(&[67, 56, 202, 255]);
            }
        }
    }

    // Draw coral notification badge in top-right corner when tasks are pending today
    if badge_count > 0 {
        let cx = 23_i32;
        let cy = 8_i32;
        let r_sq = 7 * 7;
        for py in (cy - 7)..=(cy + 7) {
            for px in (cx - 7)..=(cx + 7) {
                if px >= 0 && py >= 0 && px < size as i32 && py < size as i32 {
                    let d2 = (px - cx) * (px - cx) + (py - cy) * (py - cy);
                    if d2 <= r_sq {
                        let idx = ((py as u32 * size + px as u32) * 4) as usize;
                        out[idx] = 220;
                        out[idx + 1] = 38;
                        out[idx + 2] = 38;
                        out[idx + 3] = 255;
                    }
                }
            }
        }

        // Crisp 5x7 digit (1..9 or '+' for 10+) centered inside badge
        let glyph = digit_glyph(badge_count);
        for (gy, row_bits) in glyph.iter().enumerate() {
            for gx in 0..5 {
                if (row_bits >> (4 - gx)) & 1 == 1 {
                    let px = cx - 2 + gx;
                    let py = cy - 3 + gy as i32;
                    if px >= 0 && py >= 0 && px < size as i32 && py < size as i32 {
                        let idx = ((py as u32 * size + px as u32) * 4) as usize;
                        out[idx] = 255;
                        out[idx + 1] = 255;
                        out[idx + 2] = 255;
                        out[idx + 3] = 255;
                    }
                }
            }
        }
    }

    out
}

const fn digit_glyph(n: usize) -> [u8; 7] {
    match n {
        1 => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        2 => [0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111],
        3 => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
        4 => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        5 => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        6 => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        7 => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        8 => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        9 => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        _ => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000], // '+'
    }
}

pub fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn build_daily_notification_body(pending_titles: &[String]) -> String {
    if pending_titles.is_empty() {
        return "No tienes tareas pendientes para hoy.".to_owned();
    }
    let mut lines = vec![format!(
        "Tienes {} pendiente{} hoy:",
        pending_titles.len(),
        if pending_titles.len() == 1 { "" } else { "s" }
    )];
    for title in pending_titles.iter().take(4) {
        lines.push(format!("• {title}"));
    }
    if pending_titles.len() > 4 {
        lines.push(format!("+ {} más...", pending_titles.len() - 4));
    }
    lines.join("\n")
}

/// Fires a native Windows 10/11 Toast notification asynchronously without blocking the UI.
pub fn show_windows_toast(title: &str, body: &str) {
    let title_xml = xml_escape(title).replace('\'', "''");
    let body_xml = xml_escape(body).replace('\'', "''");

    std::thread::spawn(move || {
        let script = format!(
            "$xml = '<toast><visual><binding template=\"ToastGeneric\"><text>{title_xml}</text><text>{body_xml}</text></binding></visual></toast>'; \
             $doc = [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType=WindowsRuntime]::New(); \
             $doc.LoadXml($xml); \
             $appId = '{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\\WindowsPowerShell\\v1.0\\powershell.exe'; \
             [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType=WindowsRuntime]::CreateToastNotifier($appId).Show([Windows.UI.Notifications.ToastNotification, Windows.UI.Notifications, ContentType=WindowsRuntime]::New($doc))"
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_notification_body_should_format_pending_items() {
        assert_eq!(
            build_daily_notification_body(&[]),
            "No tienes tareas pendientes para hoy."
        );
        let items = vec![
            "09:00 Reunión".to_owned(),
            "11:00 Estudiar".to_owned(),
            "15:00 Gimnasio".to_owned(),
            "17:00 Compras".to_owned(),
            "20:00 Leer".to_owned(),
        ];
        let body = build_daily_notification_body(&items);
        assert!(body.contains("Tienes 5 pendientes hoy:"));
        assert!(body.contains("• 09:00 Reunión"));
        assert!(body.contains("+ 1 más..."));
    }

    #[test]
    fn xml_escape_should_sanitize_special_chars() {
        assert_eq!(xml_escape("A & B <C>"), "A &amp; B &lt;C&gt;");
    }
}
