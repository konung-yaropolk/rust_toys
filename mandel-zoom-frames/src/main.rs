use clap::{Parser, ValueEnum};
use image::{ImageBuffer, Rgb, RgbImage};
use num_cpus;
use rayon::prelude::*;
use std::fs;


// ────────────────────────────────────────────────
//  CONFIG — change these values
// ────────────────────────────────────────────────

// const OUTPUT_DIR: &str = "frames";

const WIDTH: u32 = 640;  //  3840; 2560; 1920; 640;
const HEIGHT: u32 = 480; //  2160; 1440; 1080; 480;

const START_CENTER_RE: f64 = -0.743643887037158704752191506114774;
const START_CENTER_IM: f64 = 0.131825904205311970493132056385139;
const START_ZOOM: f64      = 1.0;

const FRAME_COUNT: usize   = 3160;
const ZOOM_PER_FRAME: f64  = 1.07;

const BASE_MAX_ITER: u32   = 200;
const ITER_SCALE_FACTOR: f64 = 50.0;


#[derive(Parser, Debug)]
#[command(
    name = "mandel-zoom-frames",
    version = "1.0",
    about = "Generate zooming Mandelbrot frames as PNGs",
    long_about = "A command-line tool to render a sequence of zooming Mandelbrot set images.\n\n\
                  Frames are saved to 'frames/' folder.\n\
                  Use ffmpeg to make a video: ffmpeg -framerate 60 -i frames/frame_%04d.png -c:v libx264 -pix_fmt yuv420p movie.mp4"
)]
struct Args {

    // /// Output directory name to save the frames
    // #[arg(long, default_value_t = OUTPUT_DIR)]  // /frames default
    // output_dir: &str,

    /// Width of each frame (pixels)
    #[arg(long, default_value_t = WIDTH)]  // 4K default
    width: u32,

    /// Height of each frame (pixels)
    #[arg(long, default_value_t = HEIGHT)]  // 4K default
    height: u32,

    /// Starting real part of center (e.g., -0.745429)
    #[arg(long, default_value_t = START_CENTER_RE)]
    center_re: f64,

    /// Starting imaginary part of center (e.g., 0.11301)
    #[arg(long, default_value_t = START_CENTER_IM)]
    center_im: f64,

    /// Initial zoom level (1.0 = full view)
    #[arg(long, default_value_t = START_ZOOM)]
    start_zoom: f64,

    /// Number of frames to generate
    #[arg(long, default_value_t = FRAME_COUNT)]
    frame_count: usize,

    /// Zoom multiplier per frame (e.g., 1.065 for smooth)
    #[arg(long, default_value_t = ZOOM_PER_FRAME)]
    zoom_per_frame: f64,

    /// Base maximum iterations
    #[arg(long, default_value_t = BASE_MAX_ITER)]
    base_max_iter: u32,

    /// Iteration scaling factor (added per log2(zoom))
    #[arg(long, default_value_t = ITER_SCALE_FACTOR)]
    iter_scale_factor: f64,

    /// Colormap to use
    #[arg(long, value_enum, default_value_t = ColorMap::HsvClassic)]
    colormap: ColorMap,

    /// Number of threads to use (0 = auto-detect)
    #[arg(long, default_value_t = 0)]
    threads: usize,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorMap {
    HsvClassic,   // Original colorful
    HsvCycle,     // Modified original
    Grayscale,    // Black to white
    Fire,         // Red-orange-yellow
    Ocean,        // Blue-green-cyan
    Rainbow,      // Smooth rainbow cycle
    Viridis,      // Perceptually uniform green-yellow
    Magma,        // Black-red-orange-white
    Plasma,       // Blue-magenta-yellow
}

fn main() {
    let args = Args::parse();

    // Set up rayon thread pool
    let num_threads = if args.threads == 0 {
        num_cpus::get().max(1)
    } else {
        args.threads
    };
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap_or_else(|e| eprintln!("Warning: Failed to set thread pool: {}", e));

    println!("Mandelbrot zoom sequence generator");
    println!("Config: {:?}", args);
    println!("Threads: {}\n", num_threads);

    let output_dir = "frames";
    fs::create_dir_all(output_dir).expect("Failed to create frames directory");

    let mut current_zoom = args.start_zoom;

    for frame in 0..args.frame_count {
        let extra_iter = (current_zoom.log2().max(0.0) * args.iter_scale_factor) as u32;
        let max_iter = args.base_max_iter.saturating_add(extra_iter);

        let filename = format!("{}/frame_{:04}.png", output_dir, frame);
        println!(
            "Rendering frame {:4} / {}   zoom ≈ {:.3}x   max_iter = {}",
            frame + 1,
            args.frame_count,
            current_zoom,
            max_iter
        );

        let img = generate_mandelbrot_frame(
            args.center_re,
            args.center_im,
            current_zoom,
            max_iter,
            args.colormap,
            args.width,
            args.height,
        );

        if let Err(e) = img.save(&filename) {
            eprintln!("Failed to save {} : {}", filename, e);
        }

        current_zoom *= args.zoom_per_frame;
    }

    println!("\nDone.");
    println!("Frames saved in ./{}/", output_dir);
    println!("ffmpeg example: ffmpeg -framerate 30 -i frames/frame_%04d.png -c:v libx264 -pix_fmt yuv420p movie.mp4");
}

fn generate_mandelbrot_frame(
    center_re: f64,
    center_im: f64,
    zoom: f64,
    max_iter: u32,
    colormap: ColorMap,
    width: u32,
    height: u32,
) -> RgbImage {
    let mut img: RgbImage = ImageBuffer::new(width, height);

    let aspect = width as f64 / height as f64;
    let scale = 3.5 / zoom;

    let min_re = center_re - scale * aspect * 0.5;
    let min_im = center_im - scale * 0.5;
    let step_re = scale * aspect / (width as f64 - 1.0);
    let step_im = scale / (height as f64 - 1.0);

    // Split image rows into chunks (rayon-friendly mutable slices)
    let h = height as usize;
    let rows_per_chunk = (h / rayon::current_num_threads().max(1)).max(1);

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
                    *pixel = color_from_iter(iter, max_iter, colormap);
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

fn color_from_iter(iter: u32, max_iter: u32, colormap: ColorMap) -> Rgb<u8> {
    if iter == max_iter {
        return Rgb([0, 0, 0]);
    }

    let t = iter as f64 / max_iter as f64;

    match colormap {
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
        ColorMap::HsvCycle => {
            let hue = t * 360.0;            
            let s = 0.8;
            let v = if t < 0.5 { t * 2.0 } else { 1.0 };
            
            let c = v * s;
            let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
            let m = v - c;
            
            let (r_f, g_f, b_f) = match (hue / 60.0) as i32 {
                0 => (c, x, 0.0),
                1 => (x, c, 0.0),
                2 => (0.0, c, x),
                3 => (0.0, x, c),
                4 => (x, 0.0, c),
                _ => (c, 0.0, x),
            };

            let r = ((r_f + m) * 255.0).clamp(0.0, 255.0) as u8;
            let g = ((g_f + m) * 255.0).clamp(0.0, 255.0) as u8;
            let b = ((b_f + m) * 255.0).clamp(0.0, 255.0) as u8;

            Rgb([r, g, b])
            
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
        ColorMap::Rainbow => {
            let hue = (t * 360.0 + 120.0) % 360.0;  // Shifted cycle
            let s = 1.0;
            let v = 1.0;

            let c = v * s;
            let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
            let m = 0.0;

            let (r, g, b) = match (hue / 60.0) as i32 {
                0 => (c, x, m),
                1 => (x, c, m),
                2 => (m, c, x),
                3 => (m, x, c),
                4 => (x, m, c),
                _ => (c, m, x),
            };

            Rgb([
                (r * 255.0).clamp(0.0, 255.0) as u8,
                (g * 255.0).clamp(0.0, 255.0) as u8,
                (b * 255.0).clamp(0.0, 255.0) as u8,
            ])
        }
        ColorMap::Viridis => {
            // Approximate Viridis: blue-green-yellow
            let r = (1.0 - t).powi(3) * 255.0;
            let g = (1.0 - (1.0 - t).powi(2)) * 255.0;
            let b = t.sqrt() * 128.0;
            Rgb([
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
            ])
        }
        ColorMap::Magma => {
            // Black-red-orange-white
            let r = (t * 255.0 + 50.0).clamp(0.0, 255.0) as u8;
            let g = (t.powf(1.5) * 200.0).clamp(0.0, 255.0) as u8;
            let b = (t.powf(3.0) * 255.0).clamp(0.0, 255.0) as u8;
            Rgb([r, g, b])
        }
        ColorMap::Plasma => {
            // Blue-magenta-red-yellow
            let r = (0.5 + 0.5 * (t * 5.0 - 2.0).sin()) * 255.0;
            let g = (0.5 + 0.5 * (t * 5.0 - 1.0).sin()) * 255.0;
            let b = (0.5 + 0.5 * (t * 5.0).sin()) * 255.0;
            Rgb([
                r.clamp(0.0, 255.0) as u8,
                g.clamp(0.0, 255.0) as u8,
                b.clamp(0.0, 255.0) as u8,
            ])
        }
    }
}