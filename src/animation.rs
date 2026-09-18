/// Wave animation engine.
/// Produces expanding concentric ring effects radiating from a center point,
/// giving a "sonar / radar pulse" visual in the terminal.

use ratatui::style::Color;

/// A single expanding wave ring.
#[derive(Debug, Clone)]
struct WaveRing {
    /// Current radius of the ring (in character cells, aspect-corrected).
    radius: f64,
    /// Maximum radius before the ring expires.
    max_radius: f64,
}

/// Manages all active wave rings and their animation lifecycle.
#[derive(Debug, Clone)]
pub struct WaveState {
    rings: Vec<WaveRing>,
    tick_count: u64,
    /// How many ticks between spawning new rings.
    spawn_interval: u64,
    /// Maximum radius for rings.
    max_radius: f64,
    /// How fast rings expand per tick.
    expand_speed: f64,
}

impl WaveState {
    pub fn new() -> Self {
        Self {
            rings: vec![WaveRing { radius: 0.0, max_radius: 30.0 }],
            tick_count: 0,
            spawn_interval: 12,
            max_radius: 30.0,
            expand_speed: 0.4,
        }
    }

    /// Advance the animation by one tick.
    pub fn tick(&mut self) {
        self.tick_count += 1;

        // Expand existing rings
        for ring in &mut self.rings {
            ring.radius += self.expand_speed;
        }

        // Remove expired rings
        self.rings.retain(|r| r.radius < r.max_radius);

        // Spawn a new ring periodically
        if self.tick_count % self.spawn_interval == 0 {
            self.rings.push(WaveRing {
                radius: 0.0,
                max_radius: self.max_radius,
            });
        }
    }

    /// Set the maximum radius (call when terminal size changes).
    #[allow(dead_code)]
    pub fn set_max_radius(&mut self, r: f64) {
        self.max_radius = r;
        for ring in &mut self.rings {
            ring.max_radius = r;
        }
    }

    /// Check if a wave ring passes through the given canvas coordinate.
    /// Returns the character and color to render, or None if no wave is at this point.
    ///
    /// `x`, `y` are canvas coordinates in character cells.
    /// `cx`, `cy` are the center (router) coordinates in character cells.
    ///
    /// Because terminal characters are ~2× taller than wide, we apply
    /// aspect-ratio correction: horizontal distances are halved so circles
    /// appear round.
    pub fn get_wave_at(
        &self,
        x: f64,
        y: f64,
        cx: f64,
        cy: f64,
    ) -> Option<(char, Color)> {
        // Aspect ratio correction: terminal chars are ~2:1 (height:width)
        let dx = (x - cx) * 0.5; // halve horizontal distance
        let dy = y - cy;
        let dist = (dx * dx + dy * dy).sqrt();

        for ring in &self.rings {
            let r = ring.radius;
            let thickness = 1.2;

            let diff = (dist - r).abs();
            if diff < thickness {
                // Choose character based on how close to the ring center
                let ch = if diff < 0.4 {
                    '●'
                } else if diff < 0.8 {
                    '·'
                } else {
                    '.'
                };

                // Fade color as ring expands
                let progress = r / ring.max_radius; // 0.0 .. 1.0
                let color = if progress < 0.3 {
                    Color::Cyan
                } else if progress < 0.5 {
                    Color::LightBlue
                } else if progress < 0.7 {
                    Color::Blue
                } else {
                    Color::DarkGray
                };

                return Some((ch, color));
            }
        }

        None
    }
}

/// Animated dots traveling along a connection line from router to a device.
#[derive(Debug, Clone)]
pub struct ConnectionAnim {
    /// Current phase offset for the traveling dots (0.0 .. 1.0 repeating).
    phase: f64,
    /// Speed of dot travel per tick.
    speed: f64,
}

impl ConnectionAnim {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            speed: 0.05,
        }
    }

    pub fn tick(&mut self) {
        self.phase = (self.phase + self.speed) % 1.0;
    }

    /// Check if a traveling dot should be rendered at fractional position `t`
    /// along the line (0.0 = router, 1.0 = device).
    /// Returns the character if a dot is here.
    pub fn get_dot_at(&self, t: f64) -> Option<char> {
        let spacing = 0.2; // distance between dots
        let shifted = (t + self.phase) % spacing;
        if shifted < 0.06 {
            Some('•')
        } else {
            None
        }
    }
}
