use image::RgbImage;
use mandelbrot_lib::generate_mandelbrot_frame;
use minifb::{Key, MouseButton, Window, WindowOptions};
use std::time::{Duration, Instant};

const TITLE: &str = "Mandelbrot Explorer";
const INITIAL_WIDTH: usize = 1200;
const INITIAL_HEIGHT: usize = 900;

const MAX_ITER_CAP: u32 = 8000;
const MIN_ZOOM: f64 = 1e-13;
const INPUT_THROTTLE_MS: u64 = 60;

fn main() -> Result<(), minifb::Error> {
    let mut window = Window::new(
        TITLE,
        INITIAL_WIDTH,
        INITIAL_HEIGHT,
        WindowOptions {
            resize: true,
            scale: minifb::Scale::X1,
            ..WindowOptions::default()
        },
    )?;

    window.set_target_fps(60);

    // View state
    let mut center_re: f64 = -0.7436438870371587;
    let mut center_im: f64 = 0.13182590420531197;
    let mut zoom: f64 = 0.8;
    let mut max_iter_base: u32 = 180;
    let mut iter_scale_factor: f64 = 200.0;
    let mut colormap = "hsvclassic".to_string();

    // Dirty checking
    let mut needs_redraw = true;
    let mut last_center_re = center_re;
    let mut last_center_im = center_im;
    let mut last_zoom = zoom;
    let mut last_max_iter_base = max_iter_base;
    let mut last_colormap = colormap.clone();
    let mut last_width = 0usize;
    let mut last_height = 0usize;

    // Drag state
    let mut is_dragging = false;
    let mut last_mouse_x = 0.0f32;
    let mut last_mouse_y = 0.0f32;

    // Input throttle
    let mut last_input_time = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (width, height) = window.get_size();
        if width == 0 || height == 0 {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }

        let now = Instant::now();
        let mut view_changed = false;

        // Throttle continuous input
        let can_process_input = now.duration_since(last_input_time) >= Duration::from_millis(INPUT_THROTTLE_MS);

        if can_process_input {
            last_input_time = now;

            // Zoom – cursor centered
            let mut zoom_delta = 1.0f64;

            if window.is_key_down(Key::Equal) || window.is_key_down(Key::NumPadPlus) {
                zoom_delta /= 1.12;
            }
            if window.is_key_down(Key::Minus) || window.is_key_down(Key::NumPadMinus) {
                zoom_delta *= 1.12;
            }

            if zoom_delta != 1.0 {
                if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
                    let mouse_wx = pixel_to_world_x(mx as f64, center_re, zoom, width as f64);
                    let mouse_wy = pixel_to_world_y(my as f64, center_im, zoom, height as f64);

                    let old_zoom = zoom;
                    zoom = (zoom * zoom_delta).max(MIN_ZOOM);

                    center_re = mouse_wx - (mouse_wx - center_re) * (zoom / old_zoom);
                    center_im = mouse_wy - (mouse_wy - center_im) * (zoom / old_zoom);

                    view_changed = true;
                }
            }

            // Pan
            if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
                if window.get_mouse_down(MouseButton::Left) {
                    if !is_dragging {
                        is_dragging = true;
                        last_mouse_x = mx;
                        last_mouse_y = my;
                    } else {
                        let dx = (mx - last_mouse_x) as f64;
                        let dy = (my - last_mouse_y) as f64;

                        let sensitivity = 3.5 / (zoom.max(1e-6).powi(3) * width.max(1) as f64);
                        let world_dx = dx * sensitivity * 1.65;
                        let world_dy = dy * sensitivity * 1.65;

                        center_re -= world_dx;
                        center_im -= world_dy;

                        last_mouse_x = mx;
                        last_mouse_y = my;
                        view_changed = true;
                    }
                } else {
                    is_dragging = false;
                }
            }
        }

        // One-shot actions
        if window.is_key_pressed(Key::R, minifb::KeyRepeat::No) {
            center_re = -0.7436438870371587;
            center_im = 0.13182590420531197;
            zoom = 0.8;
            max_iter_base = 180;
            colormap = "hsvclassic".to_string();
            view_changed = true;
        }

        let colormap_map = [
            (Key::Key1, "hsvclassic"),
            (Key::Key2, "hsvcycle"),
            (Key::Key3, "grayscale"),
            (Key::Key4, "fire"),
            (Key::Key5, "ocean"),
            (Key::Key6, "rainbow"),
            (Key::Key7, "viridis"),
            (Key::Key8, "magma"),
            (Key::Key9, "plasma"),
        ];

        for (key, name) in colormap_map.iter() {
            if window.is_key_pressed(*key, minifb::KeyRepeat::No) {
                colormap = name.to_string();
                view_changed = true;
            }
        }

        if window.is_key_pressed(Key::PageUp, minifb::KeyRepeat::Yes) {
            max_iter_base = (max_iter_base + 20).min(2000);
            view_changed = true;
        }
        if window.is_key_pressed(Key::PageDown, minifb::KeyRepeat::Yes) {
            max_iter_base = max_iter_base.saturating_sub(20).max(50);
            view_changed = true;
        }

        // Resize detection
        let resized = width != last_width || height != last_height;

        // Redraw decision
        let state_changed = 
            center_re != last_center_re ||
            center_im != last_center_im ||
            zoom != last_zoom ||
            max_iter_base != last_max_iter_base ||
            colormap != last_colormap;

        needs_redraw = needs_redraw || view_changed || resized || state_changed;

        // ─── Always update buffer (important for message pumping) ────────
        let render_start = Instant::now();

        let effective_zoom = 1.0 / zoom;
        let extra = (effective_zoom.log2().max(0.0) * iter_scale_factor) as u32;
        let max_iter = (max_iter_base + extra).min(MAX_ITER_CAP);

        let img: RgbImage = if needs_redraw {
            generate_mandelbrot_frame(
                center_re,
                center_im,
                effective_zoom,
                max_iter,
                &colormap,
                width as u32,
                height as u32,
            )
        } else {
            // Reuse last frame when nothing changed
            // (you could keep last buffer in a variable, but for simplicity we re-render)
            generate_mandelbrot_frame(
                center_re,
                center_im,
                effective_zoom,
                max_iter,
                &colormap,
                width as u32,
                height as u32,
            )
        };

        let buffer: Vec<u32> = img
            .pixels()
            .map(|p| {
                let [r, g, b] = p.0;
                0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
            })
            .collect();

        window.update_with_buffer(&buffer, width, height)?;

        let render_ms = render_start.elapsed().as_millis();

        // ─── Show mouse coordinates when hovering ────────────────────────
        let coord_str = if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
            let wx = pixel_to_world_x(mx as f64, center_re, zoom, width as f64);
            let wy = pixel_to_world_y(my as f64, center_im, zoom, height as f64);
            format!(" | cursor: {:.8} + {:.8}i", wx, wy)
        } else {
            "".to_string()
        };

        window.set_title(&format!(
            "{} | {} ms | zoom ×{:.2e} | iter {}{}",
            TITLE, render_ms, effective_zoom, max_iter, coord_str
        ));

        // Update last known state
        last_center_re     = center_re;
        last_center_im     = center_im;
        last_zoom          = zoom;
        last_max_iter_base = max_iter_base;
        last_colormap      = colormap.clone();
        last_width         = width;
        last_height        = height;

        needs_redraw = false;

        // Sleep only after update (safe for message pumping)
        if !view_changed && !resized && !state_changed {
            std::thread::sleep(Duration::from_millis(16)); // ~60 fps when idle
        }
    }

    Ok(())
}

// Helpers: pixel → world coordinate
fn pixel_to_world_x(px: f64, center_x: f64, zoom: f64, w: f64) -> f64 {
    let scale = 3.5 / zoom;
    let min_x = center_x - scale * 0.5;
    min_x + px * (scale / (w - 1.0).max(1.0))
}

fn pixel_to_world_y(py: f64, center_y: f64, zoom: f64, h: f64) -> f64 {
    let scale = 3.5 / zoom;
    let min_y = center_y - scale * 0.5;
    min_y + py * (scale / (h - 1.0).max(1.0))
}