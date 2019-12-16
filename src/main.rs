extern crate image;
extern crate num;
extern crate multiqueue;
extern crate indicatif;
use num::complex::Complex;
use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Instant};

const MAX_ITER: u32 = 400;
const RES: u32 = 20000;
const THREAD_COUNT: u32 = 3;

const MIDDLE: (f64, f64) = (-0.761574,-0.0847596);
const SIZE: f64 = 0.00001;

fn main() {
    let start = Instant::now();
    let bounds: (usize, usize) = (RES as usize, RES as usize);

    let mut iterations = vec![0u32; bounds.0 * bounds.1];
    
    // Queueing lines instead of individual pixels to save on memory
    let (work_send, work_receive) = multiqueue::mpmc_queue::<usize>(RES as u64);
    let (result_send, result_receive) = channel();
    
    for _ in 0..THREAD_COUNT {
        let w = work_receive.clone();
        let r = result_send.clone();
        thread::spawn(move || {
            for p in w {
                for x in 0..bounds.0 {
                    let (pos_x, pos_y) = pixel_to_position(x, p);
                    let val = calc_pos(Complex::<f64> {im: pos_y, re: pos_x});
                    r.send((x, p, val)).unwrap();
                }
            }
            println!("Exited thread");
        }); 
    }

    for y in 0..bounds.1 {
        work_send.try_send(y).unwrap();
    }

    work_send.unsubscribe();
    drop(result_send);
    println!("Main thread dropped sender channels.");
    println!("Work is queued, waiting for threads to exit...");
    
    indicatif::ProgressStyle::default_spinner();

    let progress_bar = indicatif::ProgressBar::new(RES as u64 * RES as u64);

    let mut counter: usize = 0;
    for p in result_receive {
        iterations[p.1 * bounds.0 + p.0] = p.2;

        counter += 1;

        if counter == RES as usize * 10 {
            progress_bar.inc(counter as u64);
            counter = 0;
        }
    }
    drop(work_receive);
    println!("Finished rendering, writing to disk...");
    
    println!("Computation took {} seconds", start.elapsed().as_secs());
    
    let mut max_val: u32 = 0;
    
    for i in 0..RES*RES {
        if iterations[i as usize] > max_val {
            max_val = iterations[i as usize];
        }
    }
    
    let mut img = vec![0u8; bounds.0 * bounds.1];
    for i in 0..RES*RES {
        img[i as usize] = (iterations[i as usize] as f32 / max_val as f32 * 254.0) as u8;
    }

    write_image("man.png", &img, bounds).unwrap();
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize))
    -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(&pixels,
                    bounds.0 as u32, bounds.1 as u32,
                    ColorType::Gray(8))?;

    Ok(())
}

fn pixel_to_position(x: usize, y: usize) -> (f64, f64) {
    let step_size:f64 = (SIZE) / RES as f64;
    return (MIDDLE.0 - SIZE / 2 as f64 + x as f64 * step_size, MIDDLE.1 - SIZE / 2 as f64 + y as f64 * step_size);
}

fn calc_pos(pos: Complex<f64>) -> u32 {
    let mut z = Complex {re: 0.0, im: 0.0};
    for i in 0..MAX_ITER {
        z = z * z + pos;
        if z.norm_sqr() > 4.0 {
            return i;
        }
    }

    return 0;
}