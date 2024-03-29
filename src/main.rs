extern crate num;
extern crate image;
extern crate crossbeam;
extern crate rayon;

use num::Complex;
use image::ColorType;
use image::png::PNGEncoder;
use std::str::FromStr;
use std::fs::File;
use std::io::Write;
use rayon::prelude::*;

fn escape_time(c: Complex<f64>, limit: u32) -> Option<u32> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        z = z * z + c;
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
    }
    None
}

fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => {
            match (T::from_str(&s[..index]), T::from_str(&s[index+1..])) {
                (Ok(l), Ok(r)) => Some((l, r)),
                _ => None
            }
        }
    }
}

#[test]
fn test_parse_pair(){
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>(",10", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", 'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex {re, im}),
        None => None
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(parse_complex("1.25,-0.0625"), Some(Complex { re: 1.25, im: -0.0625}));
    assert_eq!(parse_complex(",-0.0625"), None);
}

fn pixel_to_point(
    bounds: (usize, usize),
    pixel: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>) -> Complex<f64> {
    let (width, height) = (lower_right.re - upper_left.re, upper_left.im - lower_right.im);
    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(pixel_to_point((100, 100), (25, 75),
                              Complex { re: -1.0, im: 1.0 },
                              Complex { re: 1.0, im: -1.0 }),
    Complex{ re: -0.5, im: -0.5})
}

fn render(pixels: &mut [u8],
    bounds: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>)
{
    for row in 0 .. bounds.1 {
        for column in 0 .. bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            pixels[row * bounds.0 + column] = match escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8
            };
        }
    }
}
fn render_by_crossbeam(pixels: &mut [u8],
                   bounds: (usize, usize),
                   upper_left: Complex<f64>,
                   lower_right: Complex<f64>)
{
    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;
    let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
    crossbeam::scope(|spawner| {
        for (i, band) in bands.into_iter().enumerate() {
            let top = rows_per_band * i;
            let height = band.len() / bounds.0;
            let band_bounds = (bounds.0, height);
            let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
            let band_lower_right = pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);

            spawner.spawn(move |_| {
                render(band, band_bounds, band_upper_left, band_lower_right);
            });

        }
    }).expect("error in crossbeam");
}
fn render_by_rayon(pixels: &mut [u8],
                   bounds: (usize, usize),
                   upper_left: Complex<f64>,
                   lower_right: Complex<f64>)
{
    let bands: Vec<(usize, &mut[u8])> = pixels.chunks_mut(bounds.0).enumerate().collect();
    bands.into_par_iter()
        .for_each(|(i, band)| {
            let top = i;
            let band_bounds = (bounds.0, 1);
            let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
            let band_lower_right = pixel_to_point(bounds, (bounds.0, top + 1), upper_left, lower_right);
            render(band, band_bounds, band_upper_left, band_lower_right);
        });
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize)) -> Result<(), std::io::Error>
{
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;

    Ok(())
}

fn help(args: Vec<String>){
    writeln!(std::io::stderr(), "Usage: mandelbrot FILE PIXELS UPPERLEFT LOWERRIGHT RENDERMETHOD").unwrap();
    writeln!(std::io::stderr(), "Example: {} mandel.png 1280x960 -2.0,1 0.6,-1 rayon", args[0]).unwrap();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let render_method = {
        match args.len() {
            5 => {
                writeln!(std::io::stderr(), "selected render method is rayon").unwrap();
                render_by_rayon
            }
            6 => {
                match &*args[5] {
                    "crossbeam" => {
                        writeln!(std::io::stderr(), "selected render method is crossbeam").unwrap();
                        render_by_crossbeam
                    }
                    "rayon" => {
                        writeln!(std::io::stderr(), "selected render method is rayon").unwrap();
                        render_by_rayon
                    }
                    "single" => {
                        writeln!(std::io::stderr(), "selected render method is single").unwrap();
                        render
                    }
                    _ => {
                        writeln!(std::io::stderr(), "no such RENDERMETHOD: {}", args[5]).unwrap();
                        writeln!(std::io::stderr(), "RENDERMETHOD(single|crossbeam|rayon)").unwrap();
                        help(args);
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                help(args);
                std::process::exit(1);
            }
        }
    };
    let bounds = parse_pair(&args[2], 'x')
        .expect("error parsing image dimensions");
    let upper_left = parse_complex(&args[3])
        .expect("error parsing upper left corner point");
    let lower_right = parse_complex(&args[4])
        .expect("error parsing lower right corner point");

    let mut pixels = vec![0; bounds.0 * bounds.1];
    render_method(&mut pixels, bounds, upper_left, lower_right);

    write_image(&args[1], &pixels, bounds)
        .expect("error writing PNG file");
}

