use image::{ImageBuffer, Rgb, RgbImage};
use std::fs;
use std::path::Path;
use rayon::prelude::*;
use image::GenericImageView; // not needed here, just for completeness

// ────────────────────────────────────────────────
//  CONFIG — change these values
// ────────────────────────────────────────────────
const WIDTH: u32 = 2560;
const HEIGHT: u32 = 1440;

const START_CENTER_RE: f64 = -0.743643887037158704752191506114774;
const START_CENTER_IM: f64 = 0.131825904205311970493132056385139;
const START_ZOOM: f64      = 1.0;

const FRAME_COUNT: usize   = 4000;
const ZOOM_PER_FRAME: f64  = 1.01;

const BASE_MAX_ITER: u32   = 200;
const ITER_SCALE_FACTOR: f64 = 50.0;


#[derive(Clone, Copy, Debug)]
enum ColorMap {
    HsvClassic,
    Grayscale,
    Fire,
    Ocean,
}

const COLOR_MAP: ColorMap = ColorMap::HsvClassic;
// ────────────────────────────────────────────────

fn main() {
    println!("Mandelbrot zoom sequence generator (optimized + colormaps)");
    println!("Center: {} + {}i", START_CENTER_RE, START_CENTER_IM);
    println!(
        "Start zoom: {:.2}x    →   {} frames   ×   {:.4}x per step",
        START_ZOOM, FRAME_COUNT, ZOOM_PER_FRAME
    );
    println!(
        "Image size: {}×{}   |   Color map: {:?}",
        WIDTH, HEIGHT, COLOR_MAP
    );
    println!("Base iter: {}  +  scaled by zoom\n", BASE_MAX_ITER);

    let output_dir = "frames";
    fs::create_dir_all(output_dir).expect("Failed to create frames directory");

    let mut current_zoom = START_ZOOM;

    for frame in 0..FRAME_COUNT {
        let extra_iter = (current_zoom.log2().max(0.0) * ITER_SCALE_FACTOR) as u32;
        let max_iter = BASE_MAX_ITER.saturating_add(extra_iter);

        let filename = format!("{}/frame_{:04}.png", output_dir, frame);
        println!(
            "Rendering frame {:4} / {}   zoom ≈ {:.3}x   max_iter = {}",
            frame + 1,
            FRAME_COUNT,
            current_zoom,
            max_iter
        );

        let img = generate_mandelbrot_frame(
            START_CENTER_RE,
            START_CENTER_IM,
            current_zoom,
            max_iter,
        );

        if let Err(e) = img.save(&filename) {
            eprintln!("Failed to save {} : {}", filename, e);
        }

        current_zoom *= ZOOM_PER_FRAME;
    }

    println!("\nDone.");
    println!("Frames saved in ./{}/", output_dir);
    println!("Example ffmpeg command:");
    println!("  ffmpeg -framerate 30 -i frames/frame_%04d.png -c:v libx264 -pix_fmt yuv420p zoom.mp4");
}

fn generate_mandelbrot_frame(
    center_re: f64,
    center_im: f64,
    zoom: f64,
    max_iter: u32,
) -> RgbImage {
    let mut img = RgbImage::new(WIDTH, HEIGHT);

    let aspect = WIDTH as f64 / HEIGHT as f64;
    let scale = 3.5 / zoom;
    let min_re = center_re - scale * aspect * 0.5;
    let min_im = center_im - scale * 0.5;
    let step_re = scale * aspect / (WIDTH as f64 - 1.0);
    let step_im = scale / (HEIGHT as f64 - 1.0);

    // Split image rows into chunks (rayon-friendly mutable slices)
    let height = HEIGHT as usize;
    let rows_per_chunk = (height / rayon::current_num_threads().max(1)).max(1);

    img.rows_mut()
        .collect::<Vec<_>>()
        .par_chunks_mut(rows_per_chunk)
        .enumerate()
        .for_each(|(chunk_idx, chunk_rows)| {
            let start_y = chunk_idx * rows_per_chunk;

            for (local_y, row) in chunk_rows.iter_mut().enumerate() {
                let y = (start_y + local_y) as u32;
                let c_im = min_im + y as f64 * step_im;

                for (x, pixel) in row.enumerate() {
                    let c_re = min_re + x as f64 * step_re;
                    let iter = mandelbrot_iter(c_re, c_im, max_iter);
                    *pixel = color_from_iter(iter, max_iter);
                }
            }
        });

    img
}

fn mandelbrot_iter(cr: f64, ci: f64, max_iter: u32) -> u32 {
    let mut zr = 0.0;
    let mut zi = 0.0;
    let mut n = 0u32;

    while zr * zr + zi * zi <= 4.0 && n < max_iter {
        let temp = zr * zr - zi * zi + cr;
        zi = 2.0 * zr * zi + ci;
        zr = temp;
        n += 1;
    }

    n
}

fn color_from_iter(iter: u32, max_iter: u32) -> Rgb<u8> {
    if iter == max_iter {
        return Rgb([0, 0, 0]);
    }

    let t = iter as f64 / max_iter as f64;

    match COLOR_MAP {
        ColorMap::HsvClassic => {
            let hue = (t * 360.0) as i32 % 360;
            let s = 0.95;
            let v = 0.92;

            let c = v * s;
            let x = c * (1.0 - (((hue as f64 / 60.0) % 2.0) - 1.0).abs());
            let m = v - c;

            let (r, g, b) = match hue / 60 {
                0 => (c, x, 0.0),
                1 => (x, c, 0.0),
                2 => (0.0, c, x),
                3 => (0.0, x, c),
                4 => (x, 0.0, c),
                _ => (c, 0.0, x),
            };

            Rgb([
                ((r + m) * 255.0).clamp(0.0, 255.0) as u8,
                ((g + m) * 255.0).clamp(0.0, 255.0) as u8,
                ((b + m) * 255.0).clamp(0.0, 255.0) as u8,
            ])
        }
        ColorMap::Grayscale => {
            let val = (t * 255.0).clamp(0.0, 255.0) as u8;
            Rgb([val, val, val])
        }
        ColorMap::Fire => {
            let r = (t * 255.0).clamp(0.0, 255.0) as u8;
            let g = (t.sqrt() * 255.0).clamp(0.0, 255.0) as u8;
            let b = (t * 64.0).clamp(0.0, 255.0) as u8;
            Rgb([r, g, b])
        }
        ColorMap::Ocean => {
            let r = (t * 40.0).clamp(0.0, 255.0) as u8;
            let g = (t * 180.0).clamp(0.0, 255.0) as u8;
            let b = (t * 255.0).clamp(0.0, 255.0) as u8;
            Rgb([r, g, b])
        }
    }
}