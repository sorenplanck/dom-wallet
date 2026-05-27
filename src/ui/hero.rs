// Hero section. The centerpiece of the Início view.
//
// Composition, drawn entirely with egui's painter (no image asset needed):
//   - dark cinematic background gradient (top deep black → faint amber base)
//   - distant atmospheric fog (low-alpha bands)
//   - reflective dark water surface across the lower half
//   - soft circular halo around the coin
//   - the official DOM coin glyph at the center (a geometric "Ⓓ" mark
//     rendered with two concentric rings and a stylized "D" — placeholder
//     for the official PNG when assets/dom-coin.png is present)
//   - subtle reflection of the coin on the water
//   - philosophical text block at left-center
//
// Everything is drawn deterministically — no random sparkles, no animated
// flicker that would feel cheap. The only motion is a very slow halo
// breathing tied to `time_seconds`.

use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, Vec2};

use super::theme::*;

pub fn render(ui: &mut egui::Ui, time_seconds: f32) {
    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
    let painter = ui.painter_at(rect);

    paint_background(&painter, rect);
    paint_water(&painter, rect);
    paint_fog(&painter, rect);

    // Coin position: centered, slightly above the water horizon.
    let horizon_y = rect.top() + rect.height() * 0.58;
    let coin_center = Pos2::new(rect.center().x, horizon_y - 110.0);
    let coin_radius = 95.0;

    paint_halo(&painter, coin_center, coin_radius, time_seconds);
    paint_coin(&painter, coin_center, coin_radius);
    paint_coin_reflection(&painter, coin_center, coin_radius, horizon_y);

    paint_philosophical_text(&painter, rect);
}

fn paint_background(painter: &egui::Painter, rect: Rect) {
    // Vertical gradient — emulated with horizontal bands since egui's painter
    // has no native gradient. Top: very deep black-blue. Bottom: warmer.
    let bands = 64;
    let h = rect.height() / bands as f32;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let r = lerp_u8(0x04, 0x0A, t.powf(1.6));
        let g = lerp_u8(0x06, 0x09, t.powf(1.4));
        let b = lerp_u8(0x0B, 0x0E, t.powf(1.2));
        let color = Color32::from_rgb(r, g, b);
        let band = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + h * i as f32),
            Vec2::new(rect.width(), h + 0.5),
        );
        painter.rect_filled(band, egui::Rounding::ZERO, color);
    }

    // Subtle vignette via corner darkening.
    let vignette_alpha = 90u8;
    let v = Color32::from_rgba_unmultiplied(0, 0, 0, vignette_alpha);
    // Top
    painter.rect_filled(
        Rect::from_min_size(rect.left_top(), Vec2::new(rect.width(), rect.height() * 0.18)),
        egui::Rounding::ZERO,
        Color32::from_rgba_unmultiplied(0, 0, 0, 40),
    );
    // Bottom
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - rect.height() * 0.12),
            rect.right_bottom(),
        ),
        egui::Rounding::ZERO,
        v,
    );
}

fn paint_water(painter: &egui::Painter, rect: Rect) {
    let horizon_y = rect.top() + rect.height() * 0.58;
    let water_rect = Rect::from_min_max(
        Pos2::new(rect.left(), horizon_y),
        rect.right_bottom(),
    );
    // Smooth darken below horizon.
    let bands = 32;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let r = lerp_u8(0x06, 0x02, t);
        let g = lerp_u8(0x08, 0x03, t);
        let b = lerp_u8(0x0E, 0x06, t);
        let color = Color32::from_rgb(r, g, b);
        let h = water_rect.height() / bands as f32;
        let band = Rect::from_min_size(
            Pos2::new(water_rect.left(), water_rect.top() + h * i as f32),
            Vec2::new(water_rect.width(), h + 0.5),
        );
        painter.rect_filled(band, egui::Rounding::ZERO, color);
    }

    // Horizon glow — a very thin amber line softened into a band.
    let glow_y = horizon_y;
    for offset in 0..8 {
        let alpha = (24 - offset * 3).max(0) as u8;
        let color = Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha);
        painter.line_segment(
            [
                Pos2::new(rect.left(), glow_y + offset as f32),
                Pos2::new(rect.right(), glow_y + offset as f32),
            ],
            Stroke::new(1.0, color),
        );
    }

    // Reflection ripples — deterministic horizontal hairlines.
    for i in 0..14 {
        let yy = horizon_y + 10.0 + i as f32 * 14.0;
        if yy > rect.bottom() {
            break;
        }
        let alpha = (10 - i / 2).max(2) as u8;
        let color = Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha);
        let inset = 80.0 + i as f32 * 18.0;
        painter.line_segment(
            [
                Pos2::new(rect.center().x - inset * 1.6, yy),
                Pos2::new(rect.center().x + inset * 1.6, yy),
            ],
            Stroke::new(1.0, color),
        );
    }
}

fn paint_fog(painter: &egui::Painter, rect: Rect) {
    // Soft horizontal fog bands above the horizon — very low alpha.
    let horizon_y = rect.top() + rect.height() * 0.58;
    for i in 0..6 {
        let y = horizon_y - 220.0 + i as f32 * 38.0;
        let alpha = (8 - i).max(2) as u8;
        let color = Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha);
        let band = Rect::from_min_size(
            Pos2::new(rect.left(), y),
            Vec2::new(rect.width(), 26.0),
        );
        painter.rect_filled(band, egui::Rounding::ZERO, color);
    }
}

fn paint_halo(painter: &egui::Painter, center: Pos2, coin_r: f32, t: f32) {
    // Very slow breathing: scale +/-2% over ~8 seconds.
    let breath = 1.0 + (t * std::f32::consts::TAU / 8.0).sin() * 0.02;

    // Concentric halos with decreasing alpha.
    let rings = 22;
    for i in 0..rings {
        let r = coin_r * (1.15 + i as f32 * 0.10) * breath;
        let alpha = ((rings - i) as f32 / rings as f32 * 30.0) as u8;
        let color = Color32::from_rgba_unmultiplied(0xF0, 0xC6, 0x74, alpha);
        painter.circle_stroke(center, r, Stroke::new(1.0, color));
    }
    // Inner soft glow (filled, very transparent).
    for i in 0..6 {
        let r = coin_r * (1.0 + i as f32 * 0.06);
        let alpha = (12 - i * 2).max(0) as u8;
        let color = Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha);
        painter.circle_filled(center, r, color);
    }
}

fn paint_coin(painter: &egui::Painter, center: Pos2, r: f32) {
    // Outer rim — warm amber.
    painter.circle_filled(center, r, Color32::from_rgb(0x1A, 0x12, 0x06));
    painter.circle_stroke(center, r, Stroke::new(2.5, AMBER));
    painter.circle_stroke(center, r - 6.0, Stroke::new(1.0, AMBER_DIM));

    // Inner face — a subtle radial darken effect via concentric rings.
    for i in 0..14 {
        let rr = r - 8.0 - i as f32 * 0.6;
        let t = i as f32 / 14.0;
        let shade = lerp_u8(0x2A, 0x10, t);
        let color = Color32::from_rgb(shade, (shade as f32 * 0.7) as u8, (shade as f32 * 0.35) as u8);
        painter.circle_stroke(center, rr, Stroke::new(0.9, color));
    }

    // "D" mark — drawn from two arcs + a vertical bar so we don't depend on
    // a particular font having the required glyph weight.
    let bar_h = r * 1.1;
    let bar_w = r * 0.22;
    let bar_left = center.x - r * 0.42;
    let bar_rect = Rect::from_min_size(
        Pos2::new(bar_left, center.y - bar_h / 2.0),
        Vec2::new(bar_w, bar_h),
    );
    painter.rect_filled(bar_rect, egui::Rounding::same(2.0), AMBER_BRIGHT);

    // Right arc of the D — drawn as a thick stroked half-circle.
    let arc_center = Pos2::new(bar_left + bar_w * 0.9, center.y);
    let arc_r = r * 0.62;
    painter.circle_stroke(arc_center, arc_r, Stroke::new(bar_w, AMBER_BRIGHT));
    // Mask the left half of the arc by overdrawing with the coin face color.
    let mask_rect = Rect::from_min_max(
        Pos2::new(arc_center.x - arc_r - bar_w, arc_center.y - arc_r - bar_w),
        Pos2::new(bar_left + bar_w * 0.5, arc_center.y + arc_r + bar_w),
    );
    painter.rect_filled(
        mask_rect,
        egui::Rounding::ZERO,
        Color32::from_rgb(0x1A, 0x12, 0x06),
    );
    // Redraw bar over mask.
    painter.rect_filled(bar_rect, egui::Rounding::same(2.0), AMBER_BRIGHT);

    // Small specular highlight — a thin amber crescent at top-left.
    painter.circle_stroke(
        Pos2::new(center.x - r * 0.35, center.y - r * 0.35),
        r * 0.55,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xF0, 0xC6, 0x74, 70)),
    );
}

fn paint_coin_reflection(
    painter: &egui::Painter,
    coin_center: Pos2,
    coin_r: f32,
    horizon_y: f32,
) {
    // Reflection sits below the horizon, vertically squashed, very faint,
    // broken up by horizontal ripple lines.
    let refl_center = Pos2::new(coin_center.x, horizon_y + (horizon_y - coin_center.y) * 0.45);
    let refl_r_x = coin_r * 0.95;
    let refl_r_y = coin_r * 0.35;

    // Faint amber ellipse approximated with filled circles squashed via stroke
    // bands. Simpler: draw a soft amber elliptical shadow as horizontal lines.
    let bands = 22;
    for i in 0..bands {
        let t = i as f32 / (bands - 1) as f32;
        let y = refl_center.y - refl_r_y + 2.0 * refl_r_y * t;
        let dy = (y - refl_center.y) / refl_r_y;
        let span = refl_r_x * (1.0 - dy * dy).max(0.0).sqrt();
        let alpha = ((1.0 - dy.abs()) * 24.0) as u8;
        let color = Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, alpha);
        painter.line_segment(
            [
                Pos2::new(refl_center.x - span, y),
                Pos2::new(refl_center.x + span, y),
            ],
            Stroke::new(1.5, color),
        );
    }

    // Hairline horizontal breaks across the reflection.
    for i in 0..5 {
        let y = refl_center.y - refl_r_y * 0.6 + i as f32 * (refl_r_y * 0.3);
        painter.line_segment(
            [
                Pos2::new(refl_center.x - refl_r_x, y),
                Pos2::new(refl_center.x + refl_r_x, y),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0x06, 0x07, 0x0A, 110)),
        );
    }
}

fn paint_philosophical_text(painter: &egui::Painter, rect: Rect) {
    // Left-center placement, restrained typography, two-line aphorism.
    let x = rect.left() + rect.width() * 0.06;
    let y = rect.top() + rect.height() * 0.34;

    painter.text(
        Pos2::new(x, y),
        Align2::LEFT_TOP,
        "A ordem nasce do determinismo.",
        FontId::proportional(20.0),
        Color32::from_rgba_unmultiplied(0xF3, 0xF4, 0xF6, 230),
    );
    painter.text(
        Pos2::new(x, y + 30.0),
        Align2::LEFT_TOP,
        "A moeda é a medida da liberdade.",
        FontId::proportional(20.0),
        Color32::from_rgba_unmultiplied(0xD6, 0xA8, 0x5F, 220),
    );

    // Thin amber underline as a stroke beneath the second line.
    painter.line_segment(
        [Pos2::new(x, y + 64.0), Pos2::new(x + 64.0, y + 64.0)],
        Stroke::new(1.0, AMBER),
    );

    // Discreet caption above.
    painter.text(
        Pos2::new(x, y - 22.0),
        Align2::LEFT_TOP,
        "DETERMINISTIC  ·  SOVEREIGN  ·  PERMANENT",
        FontId::proportional(10.0),
        Color32::from_rgba_unmultiplied(0x8B, 0x94, 0x9E, 180),
    );
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t.clamp(0.0, 1.0)).round() as u8
}
