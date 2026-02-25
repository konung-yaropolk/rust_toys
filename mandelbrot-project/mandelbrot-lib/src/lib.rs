// mandelbrot-lib/src/lib.rs
use image::{ImageBuffer, Rgb, RgbImage};
use rayon::prelude::*;

// Optimized render

pub fn generate_mandelbrot_frame(
    center_re: f64,
    center_im: f64,
    zoom: f64,
    max_iter: u32,
    colormap: &str,
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

    // Symmetry optimization
    if center_im.abs() < 1e-10 {
        // Compute only upper half (y from 0 to mid inclusive)
        let h = height as usize;
        let mid = h / 2;  // last upper row index

        // Parallel compute upper half (including middle row if height odd)
        img.par_enumerate_pixels_mut()
            .filter(|(_, y, _)| (*y as usize) <= mid)
            .for_each(|(x, y, pixel)| {
                let c_re = min_re + x as f64 * step_re;
                let c_im = min_im + y as f64 * step_im;
                let iter = mandelbrot_iter(c_re, c_im, max_iter);
                *pixel = color_from_iter(iter, max_iter, colormap);
            });

        // Mirror lower half by copying rows (raw buffer access)
        let bytes_per_row = width as usize * 3;
        let data: &mut [u8] = img.as_mut();

        for y in (mid + 1)..h {
            let src_y = h - 1 - y;
            let dst_start = y * bytes_per_row;
            let src_start = src_y * bytes_per_row;

            // Split the mutable borrow into two non-overlapping mutable slices
            let (left, right) = data.split_at_mut(dst_start);
            let dst_slice = &mut right[0..bytes_per_row];

            let src_slice = &left[src_start..src_start + bytes_per_row];  // immutable borrow from left part

            dst_slice.copy_from_slice(src_slice);
        }
    } else {
        // No symmetry — full parallel fill
        img.par_enumerate_pixels_mut()
            .for_each(|(x, y, pixel)| {
                let c_re = min_re + x as f64 * step_re;
                let c_im = min_im + y as f64 * step_im;
                let iter = mandelbrot_iter(c_re, c_im, max_iter);
                *pixel = color_from_iter(iter, max_iter, colormap);
            });
    }

    img
}

fn color_from_iter(iter: u32, max_iter: u32, colormap_name: &str) -> Rgb<u8> {
    if iter == max_iter {
        return Rgb([0, 0, 0]);
    }

    let t = iter as f64 / max_iter as f64;

    match colormap_name.to_lowercase().as_str() {
        "hsvclassic" | "classic" => {
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
        "hsvcycle" => {
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
        "grayscale" | "grey" => {
            let val = (t * 255.0).clamp(0.0, 255.0) as u8;
            Rgb([val, val, val])
        }
        "fire" => {
            let r = (t * 255.0).clamp(0.0, 255.0) as u8;
            let g = (t.sqrt() * 255.0).clamp(0.0, 255.0) as u8;
            let b = (t * 64.0).clamp(0.0, 255.0) as u8;
            Rgb([r, g, b])
        }
        "ocean" => {
            let r = (t * 40.0).clamp(0.0, 255.0) as u8;
            let g = (t * 180.0).clamp(0.0, 255.0) as u8;
            let b = (t * 255.0).clamp(0.0, 255.0) as u8;
            Rgb([r, g, b])
        }
        "rainbow" => {
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
        "viridis" => {
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
        "magma" => {
            // Black-red-orange-white
            let r = (t * 255.0 + 50.0).clamp(0.0, 255.0) as u8;
            let g = (t.powf(1.5) * 200.0).clamp(0.0, 255.0) as u8;
            let b = (t.powf(3.0) * 255.0).clamp(0.0, 255.0) as u8;
            Rgb([r, g, b])
        }
        "plasma" => {
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
        &_ => {
            panic!("Unexpected invalid colormap name")
        }
    }
}

fn mandelbrot_iter(cr: f64, ci: f64, max_iter: u32) -> u32 {

    // Optimization 1: Check main cardioid
    if is_in_main_cardioid(cr, ci) {
        return max_iter;
    }

    // Optimization 2: Check period-2 bulb
    if is_in_period2_bulb(cr, ci) {
        return max_iter;
    }

    // Optimization for period-3 bulbs
    if is_in_period3_bulb(cr, ci) {
        return max_iter;
    }

    let mut zr = 0.0;
    let mut zi = 0.0;
    let mut n = 0u32;

    // Optimization 6: Already using squared magnitude (<= 4.0 instead of sqrt <= 2.0)
    while zr * zr + zi * zi <= 4.0 && n < max_iter {
        let temp = zr * zr - zi * zi + cr;
        zi = 2.0 * zr * zi + ci;
        zr = temp;
        n += 1;
    }

    n
}

fn is_in_main_cardioid(re: f64, im: f64) -> bool {
    let q = (re - 0.25).powi(2) + im.powi(2);
    q * (q + (re - 0.25)) <= (im.powi(2) / 4.0 ) + 1e-14   // tiny epsilon to catch fp rounding
}

fn is_in_period2_bulb(re: f64, im: f64) -> bool {
    let p = (re + 1.0).powi(2) + im.powi(2);
    p+1.0 <= 0.25 + 1e-14   // tiny epsilon to catch fp rounding
}

fn complex_sqrt(re: f64, im: f64) -> (f64, f64) {
    let modulus = (re * re + im * im).sqrt();
    let arg = (im.atan2(re)) / 2.0;
    let sqrt_mod = modulus.sqrt();
    (sqrt_mod * arg.cos(), sqrt_mod * arg.sin())
}

fn is_in_period3_bulb(cr: f64, ci: f64) -> bool {
    // Compute λ^2
    let l2re = cr * cr - ci * ci;
    let l2im = 2.0 * cr * ci;

    // Compute λ^3 = λ^2 * λ
    let l3re = l2re * cr - l2im * ci;
    let l3im = l2re * ci + l2im * cr;

    // 2 λ^2
    let twol2re = 2.0 * l2re;
    let twol2im = 2.0 * l2im;

    // λ^3 + 2 λ^2 + λ + 1
    let coef_cre = l3re + twol2re + cr + 1.0;
    let coef_cim = l3im + twol2im + ci;

    // b = -(λ + 2)
    let bre = -(cr + 2.0);
    let bim = -ci;

    // D = b^2 - 4 * coef_c
    let b2re = bre * bre - bim * bim;
    let b2im = 2.0 * bre * bim;
    let dre = b2re - 4.0 * coef_cre;
    let dim = b2im - 4.0 * coef_cim;

    // sqrt(D)
    let (sqrt_dre, sqrt_dim) = complex_sqrt(dre, dim);

    // a1 = [-b + sqrt(D)] / 2
    let a1re = ( -bre + sqrt_dre ) / 2.0;
    let a1im = ( -bim + sqrt_dim ) / 2.0;
    let mod1 = (a1re * a1re + a1im * a1im).sqrt();

    // a2 = [-b - sqrt(D)] / 2
    let a2re = ( -bre - sqrt_dre ) / 2.0;
    let a2im = ( -bim - sqrt_dim ) / 2.0;
    let mod2 = (a2re * a2re + a2im * a2im).sqrt();

    mod1 <= 0.125 || mod2 <= 0.125
}