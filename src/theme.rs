//! Galaxy Mode: a professional space-themed visual style plus an animated
//! starfield rendered behind the whole UI.

use eframe::egui::{self, Color32, LayerId, Mesh, Pos2, Rect, Vec2};

/// Tiny deterministic PRNG (xorshift64) so the starfield looks the same each
/// run without pulling in the `rand` crate.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + self.unit() * (b - a)
    }
}

struct Star {
    /// Position in normalized 0..1 screen space.
    pos: Vec2,
    radius: f32,
    brightness: f32,
    twinkle_speed: f32,
    twinkle_phase: f32,
    drift: Vec2,
    color: Color32,
}

struct Nebula {
    center: Vec2,
    radius: f32,
    color: Color32,
}

pub struct Starfield {
    stars: Vec<Star>,
    nebulae: Vec<Nebula>,
}

impl Starfield {
    pub fn new() -> Self {
        let mut rng = Rng(0x9E3779B97F4A7C15);

        // Bluish-white star palette with a few warm outliers.
        let palette = [
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(200, 220, 255),
            Color32::from_rgb(170, 200, 255),
            Color32::from_rgb(220, 210, 255),
            Color32::from_rgb(255, 240, 220),
            Color32::from_rgb(180, 255, 245),
        ];

        let stars = (0..150)
            .map(|_| {
                let small = rng.unit() < 0.8;
                Star {
                    pos: Vec2::new(rng.unit(), rng.unit()),
                    radius: if small {
                        rng.range(0.5, 1.3)
                    } else {
                        rng.range(1.3, 2.4)
                    },
                    brightness: rng.range(0.4, 1.0),
                    twinkle_speed: rng.range(0.6, 2.5),
                    twinkle_phase: rng.range(0.0, std::f32::consts::TAU),
                    // Very slow drift, mostly horizontal.
                    drift: Vec2::new(rng.range(-0.004, 0.004), rng.range(-0.002, 0.002)),
                    color: palette[(rng.unit() * palette.len() as f32) as usize % palette.len()],
                }
            })
            .collect();

        // Soft nebula clouds tinted purple / magenta / blue / teal.
        let nebula_colors = [
            Color32::from_rgba_unmultiplied(120, 60, 200, 34),
            Color32::from_rgba_unmultiplied(200, 60, 160, 28),
            Color32::from_rgba_unmultiplied(50, 90, 220, 30),
            Color32::from_rgba_unmultiplied(40, 180, 190, 22),
            Color32::from_rgba_unmultiplied(150, 80, 230, 26),
        ];
        let nebulae = nebula_colors
            .iter()
            .map(|&color| Nebula {
                center: Vec2::new(rng.unit(), rng.unit()),
                radius: rng.range(0.35, 0.65),
                color,
            })
            .collect();

        Self { stars, nebulae }
    }

    /// Paints the deep-space background, nebulae, and twinkling stars onto the
    /// background layer, then requests a repaint to keep the animation going.
    pub fn paint(&mut self, ctx: &egui::Context) {
        let screen = ctx.content_rect();
        let time = ctx.input(|i| i.time) as f32;
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let painter = ctx.layer_painter(LayerId::background());

        // Deep-space base.
        painter.rect_filled(screen, 0.0, Color32::from_rgb(7, 5, 16));

        // Nebula glow clouds.
        for neb in &self.nebulae {
            let center = to_screen(neb.center, screen);
            radial_glow(&painter, center, neb.radius * screen.width().max(screen.height()), neb.color, 28);
        }

        // Stars.
        for star in &mut self.stars {
            star.pos += star.drift * dt;
            star.pos.x = star.pos.x.rem_euclid(1.0);
            star.pos.y = star.pos.y.rem_euclid(1.0);

            let twinkle = ((time * star.twinkle_speed + star.twinkle_phase).sin() * 0.5 + 0.5)
                .clamp(0.0, 1.0);
            let alpha = (star.brightness * (0.35 + 0.65 * twinkle)).clamp(0.0, 1.0);
            let pos = to_screen(star.pos, screen);

            let [r, g, b, _] = star.color.to_array();
            let core = Color32::from_rgba_unmultiplied(r, g, b, (alpha * 255.0) as u8);
            let glow = Color32::from_rgba_unmultiplied(r, g, b, (alpha * 90.0) as u8);

            radial_glow(&painter, pos, star.radius * 4.0, glow, 12);
            painter.circle_filled(pos, star.radius, core);
        }

        // Keep animating.
        ctx.request_repaint();
    }
}

fn to_screen(norm: Vec2, screen: Rect) -> Pos2 {
    Pos2::new(
        screen.left() + norm.x * screen.width(),
        screen.top() + norm.y * screen.height(),
    )
}

/// Draws a soft radial gradient (solid center fading to transparent edge)
/// using a triangle-fan mesh.
fn radial_glow(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, segments: usize) {
    let mut mesh = Mesh::default();
    let edge = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 0);

    let center_idx = mesh.vertices.len() as u32;
    mesh.colored_vertex(center, color);
    for i in 0..=segments {
        let angle = i as f32 / segments as f32 * std::f32::consts::TAU;
        mesh.colored_vertex(center + Vec2::angled(angle) * radius, edge);
    }
    for i in 0..segments as u32 {
        mesh.add_triangle(center_idx, center_idx + 1 + i, center_idx + 2 + i);
    }
    painter.add(mesh);
}

/// Applies either the galaxy style (translucent dark panels, cosmic accents so
/// the starfield shows through) or a clean default dark style.
pub fn apply_style(ctx: &egui::Context, galaxy: bool) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    if galaxy {
        v.dark_mode = true;
        v.panel_fill = Color32::from_rgba_unmultiplied(12, 10, 26, 150);
        v.window_fill = Color32::from_rgba_unmultiplied(18, 14, 38, 235);
        v.window_stroke = egui::Stroke::new(1.0, Color32::from_rgb(120, 90, 200));
        v.extreme_bg_color = Color32::from_rgba_unmultiplied(8, 6, 20, 180);
        v.faint_bg_color = Color32::from_rgba_unmultiplied(255, 255, 255, 8);
        v.hyperlink_color = Color32::from_rgb(150, 190, 255);
        v.selection.bg_fill = Color32::from_rgba_unmultiplied(120, 90, 220, 130);
        v.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(180, 160, 255));
        v.override_text_color = Some(Color32::from_rgb(220, 222, 240));

        let w = &mut v.widgets;
        w.noninteractive.bg_fill = Color32::from_rgba_unmultiplied(20, 16, 40, 120);
        w.inactive.bg_fill = Color32::from_rgba_unmultiplied(40, 32, 70, 160);
        w.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(30, 24, 56, 140);
        w.hovered.bg_fill = Color32::from_rgba_unmultiplied(80, 60, 150, 180);
        w.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(60, 46, 110, 170);
        w.active.bg_fill = Color32::from_rgba_unmultiplied(110, 80, 200, 200);
    } else {
        *v = egui::Visuals::dark();
    }

    ctx.set_style(style);
}
