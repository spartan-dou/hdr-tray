// Dessin de l'icône « HDR » (partagé entre l'appli et build.rs pour le .ico).

pub const ON: i32 = 1;
pub const NONE: i32 = -1;

// Noir et blanc : la pastille suit le thème de la barre des tâches (blanche sur sombre, noire sur claire)
const WHITE: [u8; 3] = [255, 255, 255];
const BLACK: [u8; 3] = [0, 0, 0];
// Opacité de la pastille quand le HDR est désactivé / absent
const OFF_ALPHA: f32 = 0.45;
const NONE_ALPHA: f32 = 0.25;

// Police 3x5 : H, D, R
const GLYPHS: [[u8; 5]; 3] = [[0b101, 0b101, 0b111, 0b101, 0b101], [0b110, 0b101, 0b101, 0b101, 0b110], [0b110, 0b101, 0b110, 0b101, 0b101]];

fn in_rrect(x: f32, y: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    if x < x0 || x > x1 || y < y0 || y > y1 {
        return false;
    }
    let cx = x.max(x0 + r).min(x1 - r);
    let cy = y.max(y0 + r).min(y1 - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

fn in_text(x: f32, y: f32, u: f32) -> bool {
    if x < 0.0 || y < 0.0 {
        return false;
    }
    let (col, row) = ((x / u) as usize, (y / u) as usize);
    let (glyph, c) = (col / 4, col % 4);
    row < 5 && glyph < 3 && c < 3 && (GLYPHS[glyph][row] >> (2 - c)) & 1 == 1
}

/// Distance (en unités de la grille 16×16) au segment de la barre « désactivé ».
fn slash_dist(gx: f32, gy: f32) -> f32 {
    let (ax, ay, bx, by) = (1.5f32, 14.5f32, 14.5f32, 1.5f32);
    let (dx, dy) = (bx - ax, by - ay);
    let t = (((gx - ax) * dx + (gy - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
    ((gx - ax - t * dx).powi(2) + (gy - ay - t * dy).powi(2)).sqrt()
}

/// Pixels RGBA (alpha non prémultiplié), de haut en bas, pour l'icône de l'exe :
/// pastille noire, lettres blanches opaques (lisible sur fond clair comme sombre).
#[allow(dead_code)] // utilisé par build.rs
pub fn render(size: u32, state: i32) -> Vec<[u8; 4]> {
    draw(size, state, BLACK, Some(WHITE))
}

/// Icône de la zone de notification. Actif : pastille pleine, lettres évidées.
/// Inactif : pastille atténuée, lettres évidées et barre diagonale. `light` : barre des tâches claire.
pub fn render_themed(size: u32, state: i32, light: bool) -> Vec<[u8; 4]> {
    draw(size, state, if light { BLACK } else { WHITE }, None)
}

/// `letters` : couleur opaque des lettres, ou `None` pour les évider (transparentes).
fn draw(size: u32, state: i32, fg: [u8; 3], letters: Option<[u8; 3]>) -> Vec<[u8; 4]> {
    let s = size as f32;
    let u = s / 16.0;
    let (bw, bh) = (15.0 * u, 9.0 * u);
    let (ox, oy) = (((s - bw) / 2.0).floor(), ((s - bh) / 2.0).floor());
    let r = 2.0 * u;
    let (tx, ty) = (ox + 2.0 * u, oy + 2.0 * u);
    let base = match state {
        ON => 1.0,
        NONE => NONE_ALPHA,
        _ => OFF_ALPHA,
    };
    let slashed = state != ON;
    const SS: usize = 4;

    let mut out = Vec::with_capacity((size * size) as usize);
    for py in 0..size {
        for px in 0..size {
            // Couverture de chaque couche, en sous-pixels
            let (mut badge_n, mut text_n, mut line_n) = (0, 0, 0);
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f32 + (sx as f32 + 0.5) / SS as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SS as f32;
                    let d = slash_dist(x / u, y / u);
                    if slashed && d <= 0.65 {
                        line_n += 1;
                    } else if !(slashed && d <= 1.7) {
                        badge_n += in_rrect(x, y, ox, oy, ox + bw, oy + bh, r) as u32;
                        text_n += in_text(x - tx, y - ty, u) as u32;
                    }
                }
            }
            let n = (SS * SS) as f32;

            let (b, f, l) = (badge_n as f32 / n, text_n as f32 / n, line_n as f32 / n);
            let px = match letters {
                // Lettres opaques par-dessus la pastille
                Some(c) => {
                    let mix = |i: usize| (fg[i] as f32 * (1.0 - f) + c[i] as f32 * f) as u8;
                    [mix(0), mix(1), mix(2), ((b * base + l) * 255.0).min(255.0) as u8]
                }
                // Lettres évidées : transparentes dans la pastille
                None => [fg[0], fg[1], fg[2], (((b - f).max(0.0) * base + l) * 255.0).min(255.0) as u8],
            };
            out.push(px);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(px: &[[u8; 4]], size: u32, x: u32, y: u32) -> u8 {
        px[(y * size + x) as usize][3]
    }

    #[test]
    fn badge_follows_taskbar_theme() {
        let opaque = |px: Vec<[u8; 4]>| px.into_iter().find(|p| p[3] == 255).unwrap();
        assert_eq!(opaque(render_themed(32, ON, false))[..3], WHITE);
        assert_eq!(opaque(render_themed(32, ON, true))[..3], BLACK);
    }

    #[test]
    fn off_is_dimmer_than_on_and_slashed() {
        let on = render_themed(32, ON, false);
        let off = render_themed(32, 0, false);
        // Pastille atténuée hors de la barre (coin haut gauche de la pastille)
        assert!(alpha_at(&off, 32, 3, 8) < alpha_at(&on, 32, 3, 8));
        // La barre est pleinement opaque au centre
        assert_eq!(alpha_at(&off, 32, 16, 16), 255);
    }
}
