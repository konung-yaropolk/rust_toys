use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use rayon::prelude::*;
use std::f64::consts::PI;
use std::time::{Duration, Instant};

const WIDTH: usize = 960;
const HEIGHT: usize = 720;

const BASE_MAX_ITER: usize = 300;           // used when zoom is low
const MAX_MAX_ITER: usize = 800;           // cap when deeply zoomed
const ZOOM_SPEED: f64 = 1.015;              // lower = slower but smoother
const MIN_ZOOM: f64 = 0.4;                  // starting zoom level
const INITIAL_CENTER_X: f64 = -0.5;
const INITIAL_CENTER_Y: f64 = 0.0;

const COLOR_CYCLE_SPEED: f64 = 0.8;         // higher = faster color cycling

fn main() {
    let mut window = Window::new(
        "Mandelbrot Zoom - ESC to exit, SPACE to pause/resume",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .unwrap_or_else(|e| panic!("{}", e));

    window.limit_update_rate(Some(Duration::from_micros(16600))); // ~60 fps target

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let mut center_x = INITIAL_CENTER_X;
    let mut center_y = INITIAL_CENTER_Y;
    let mut zoom = MIN_ZOOM;

    let mut zooming = true;
    let mut last_frame = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();

        // Auto-zoom
        if zooming && now - last_frame >= Duration::from_millis(16) {
            zoom *= ZOOM_SPEED;
            last_frame = now;
        }

        // Toggle pause/resume
        if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
            zooming = !zooming;
        }

        // Click to recenter
        if window.get_mouse_down(MouseButton::Left) {
            if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
                let aspect = WIDTH as f64 / HEIGHT as f64;
                let scale_x = if aspect > 1.0 { aspect } else { 1.0 } / zoom;
                let scale_y = 1.0 / zoom;

                let dx = (mx as f64 / WIDTH as f64 - 0.5) * 2.0 * scale_x;
                let dy = (my as f64 / HEIGHT as f64 - 0.5) * 2.0 * scale_y;

                center_x += dx;
                center_y -= dy; // flip y-axis (screen y grows downward)
            }
        }

        // Dynamic iteration limit — more iterations when deeply zoomed
        let iter_limit = (BASE_MAX_ITER as f64 + (zoom.log2().max(0.0) * 220.0).min((MAX_MAX_ITER - BASE_MAX_ITER) as f64)) as usize;

        // Parallel render
        let lines: Vec<_> = (0..HEIGHT).collect();

        let row_colors: Vec<Vec<u32>> = lines.par_iter().map(|&y| {
            let mut row = vec![0u32; WIDTH];

            let aspect = WIDTH as f64 / HEIGHT as f64;
            let scale_x = if aspect > 1.0 { aspect } else { 1.0 } / zoom;
            let scale_y = 1.0 / zoom;

            for x in 0..WIDTH {
                let cx = center_x + (x as f64 / WIDTH as f64 - 0.5) * 2.0 * scale_x;
                let cy = center_y + (y as f64 / HEIGHT as f64 - 0.5) * 2.0 * scale_y;

                let mut zx = 0.0;
                let mut zy = 0.0;
                let mut iter = 0usize;

                while iter < iter_limit {
                    let zz = zx * zx + zy * zy;
                    if zz > 4.0 {
                        break;
                    }
                    let xtemp = zx * zx - zy * zy + cx;
                    zy = 2.0 * zx * zy + cy;
                    zx = xtemp;
                    iter += 1;
                }

                let color = if iter == iter_limit {
                    [0u8, 0, 0]
                } else {
                    // Smooth coloring
                    let log_zn = (zx * zx + zy * zy).ln() * 0.5;
                    let nu = iter as f64 + 1.0 - log_zn.ln() / PI.ln();
                    twilight_color(nu * COLOR_CYCLE_SPEED)
                };

                let r = color[0] as u32;
                let g = color[1] as u32;
                let b = color[2] as u32;
                row[x] = (r << 16) | (g << 8) | b;
            }
            row
        }).collect();

        // Flatten results into buffer
        for (y, row) in row_colors.into_iter().enumerate() {
            let offset = y * WIDTH;
            buffer[offset..offset + WIDTH].copy_from_slice(&row);
        }

        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .unwrap();
    }
}

fn twilight_color(t: f64) -> [u8; 3] {
    let t = t.fract(); // make it cyclic
    let r = (0.5 + 0.5 * (2.0 * PI * (t + 0.00)).cos()) * 255.0;
    let g = (0.5 + 0.5 * (2.0 * PI * (t + 0.33)).cos()) * 255.0;
    let b = (0.5 + 0.5 * (2.0 * PI * (t + 0.67)).cos()) * 255.0;
    [r.round() as u8, g.round() as u8, b.round() as u8]
}
