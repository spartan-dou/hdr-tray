// Dessin de l'icône « HDR » (partagé entre l'appli et build.rs pour le .ico).

pub const ON: i32 = 1;
pub const NONE: i32 = -1;

const ORANGE: [u8; 3] = [255, 176, 0];
const GRAY: [u8; 3] = [160, 160, 160];

// Police 3x5 : H, D, R
const GLYPHS: [[u8; 5]; 3] = [
    [0b101, 0b101, 0b111, 0b101, 0b101],
    [0b110, 0b101, 0b101, 0b101, 0b110],
    [0b110, 0b101, 0b110, 0b101, 0b101],
];

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

/// Pixels RGBA (alpha non prémultiplié), de haut en bas.
/// Actif : pastille orange pleine, texte noir. Inactif : contour et texte gris.
pub fn render(size: u32, state: i32) -> Vec<[u8; 4]> {
    let s = size as f32;
    let u = s / 16.0;
    let (bw, bh) = (15.0 * u, 9.0 * u);
    let (ox, oy) = (((s - bw) / 2.0).floor(), ((s - bh) / 2.0).floor());
    let (r, t) = (2.0 * u, u.max(1.0));
    let (tx, ty) = (ox + 2.0 * u, oy + 2.0 * u);
    const SS: usize = 4;

    let mut out = Vec::with_capacity((size * size) as usize);
    for py in 0..size {
        for px in 0..size {
            let (mut shape_n, mut text_n) = (0, 0);
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f32 + (sx as f32 + 0.5) / SS as f32;
                    let y = py as f32 + (sy as f32 + 0.5) / SS as f32;
                    let outer = in_rrect(x, y, ox, oy, ox + bw, oy + bh, r);
                    let shape = if state == ON {
                        outer
                    } else {
                        outer && !in_rrect(x, y, ox + t, oy + t, ox + bw - t, oy + bh - t, (r - t).max(0.0))
                    };
                    shape_n += shape as u32;
                    text_n += in_text(x - tx, y - ty, u) as u32;
                }
            }
            let n = (SS * SS) as f32;
            let (b, f) = (shape_n as f32 / n, text_n as f32 / n);
            let px = if state == ON {
                let c = |i: usize| (ORANGE[i] as f32 * (1.0 - f)) as u8;
                [c(0), c(1), c(2), (b * 255.0) as u8]
            } else {
                let a = (b + f - b * f) * if state == NONE { 0.5 } else { 1.0 };
                [GRAY[0], GRAY[1], GRAY[2], (a * 255.0) as u8]
            };
            out.push(px);
        }
    }
    out
}
