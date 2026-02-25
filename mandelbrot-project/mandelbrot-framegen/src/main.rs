// mandel-zoom-frames/src/main.rs
use clap::{Parser};
use image::{ImageBuffer, RgbImage};
use mandelbrot_lib::{generate_mandelbrot_frame};
use rayon::ThreadPoolBuilder;
use std::fs;

// ────────────────────────────────────────────────
//  DEFAULT CONFIG 
// ────────────────────────────────────────────────

const WIDTH:  u32          = 640;     //  3840; 2560; 1920; 800; 640;
const HEIGHT: u32          = 480;     //  2160; 1440; 1080; 600; 480;

const START_CENTER_RE: f64 = -0.743643887037158704752191506114774;     // Starting real part of center
const START_CENTER_IM: f64 =  0.131825904205311970493132056385139;     // Starting imaginary part of center
const START_ZOOM: f64      = 1.0;     // Initial zoom level (1.0 = full view)

const FRAME_COUNT: usize   = 3160;    // Number of frames to generate
const ZOOM_PER_FRAME: f64  = 1.07;    // Zoom multiplier per frame (e.g., 1.065 for smooth)

const BASE_MAX_ITER: u32   = 200;     // 200 is good optimization for limitations of f64 range
const ITER_SCALE_FACTOR: f64 = 200.0; // 200 is good optimization for limitations of f64 range

const THREADS: usize = 0;     // Number of threads to use (0 = auto-detect)

// const OUTPUT_DIR: &str = "frames";

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
    #[arg(long, default_value_t = WIDTH)]  // set const as default
    width: u32,

    /// Height of each frame (pixels)
    #[arg(long, default_value_t = HEIGHT)]  // set const as default
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

    /// Colormap to use (hsvclassic, hsvcycle, grayscale, fire, ocean, rainbow, viridis, magma, plasma)
    #[arg(
        long,
        default_value = "hsvclassic",
        help = "Colormap: hsvclassic, hsvcycle, grayscale, fire, ocean, rainbow, viridis, magma, plasma"
    )]
    colormap: String,

    /// Number of threads to use (0 = auto-detect)
    #[arg(long, default_value_t = THREADS)]
    threads: usize,
}

fn main() {
    let args = Args::parse();

    // Set up global rayon thread pool once
    let num_threads = if args.threads == 0 {
        num_cpus::get().max(1)
    } else {
        args.threads
    };
    ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap_or_else(|e| eprintln!("Warning: Failed to set global thread pool: {}", e));

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

        let data = generate_mandelbrot_frame(
            args.center_re,
            args.center_im,
            current_zoom,
            max_iter,
            &args.colormap,
            args.width,
            args.height,
            args.threads,
        );

        let img: RgbImage = ImageBuffer::from_vec(args.width, args.height, data)
            .expect("Failed to create image buffer");

        if let Err(e) = img.save(&filename) {
            eprintln!("Failed to save {} : {}", filename, e);
        }

        current_zoom *= args.zoom_per_frame;
    }

    println!("\nDone.");
    println!("Frames saved in ./{}/", output_dir);
    println!("ffmpeg example: ffmpeg -framerate 30 -i frames/frame_%04d.png -c:v libx264 -pix_fmt yuv420p movie.mp4");
}