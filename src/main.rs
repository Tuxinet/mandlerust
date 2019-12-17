extern crate image;
extern crate num;
extern crate multiqueue;
extern crate indicatif;
extern crate rug;
use image::ColorType;
use image::png::PNGEncoder;
use std::fs::File;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Instant};
use rug::{Float, Complex};
use rug::float::Round;
use std::cmp::Ordering;

const MAX_ITER: u32 = 3000;
const RES: u32 = 128;
const THREAD_COUNT: u32 = 4;
const PREC: u32 = 256;
const SIZE_STR: &str = "0.0000000000000000000000000000000000001";

const COORDS: (&str, &str) = ("-1.7685736563152709932817429153295447129341200534055498823375111352827765533646353820119779335363321986478087958745766432300344486098206084588445291690832853792608335811319613234806674959498380432536269122404488847453646628324959064543", 
                              "-0.0009642968513582800001762427203738194482747761226565635652857831533070475543666558930286153827950716700828887932578932976924523447497708248894734256480183898683164582055541842171815899305250842692638349057118793296768325124255746563");
const BASE: i32 = 10;

fn main() {
    let start = Instant::now();
    let bounds: (usize, usize) = (RES as usize, RES as usize);

    let radX = Float::parse_radix(COORDS.0, BASE);
    let floatX = Float::with_val(PREC, radX.unwrap());

    let radY = Float::parse_radix(COORDS.1, BASE);
    let floatY = Float::with_val(PREC, radY.unwrap());

    let coords: (Float, Float) = (floatX, floatY);

    let radS = Float::parse_radix(SIZE_STR, BASE);
    let size = Float::with_val(PREC, radS.unwrap());

    let mut iterations = vec![0u32; bounds.0 * bounds.1];
    
    // Queueing lines instead of individual pixels to save on memory
    let (work_send, work_receive) = multiqueue::mpmc_queue::<usize>(RES as u64);
    let (result_send, result_receive) = channel();
    
    for _ in 0..THREAD_COUNT {
        let w = work_receive.clone();
        let r = result_send.clone();
        let c = coords.clone();
        let s = size.clone();
        thread::spawn(move || {
            for p in w {
                for x in 0..bounds.0 {
                    let (pos_x, pos_y) = pixel_to_position(c.clone(), s.clone(), x, p);
                    let val = calc_pos(&Complex::with_val(PREC, (pos_x, pos_y)));
                    r.send((x, p, val)).unwrap();
                }
            }
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
        println!("{}", p.2);

        counter += 1;

        if counter == RES as usize / 4 {
            progress_bar.inc(counter as u64);
            counter = 0;
        }
    }
    progress_bar.finish_with_message("Image computation finished!");
    progress_bar.abandon();
    drop(work_receive);
    
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

fn pixel_to_position(middle: (Float, Float), size: Float, x: usize, y: usize) -> (Float, Float) {
    let step_size:Float = (size.clone()) / RES;
    return (middle.0 - size.clone() / 2 + Float::with_val(PREC, x) * step_size.clone(), middle.1 - size.clone() / 2 + Float::with_val(PREC, y) * step_size);
}

fn calc_pos(pos: &Complex) -> u32 {
    let mut z = Complex::with_val(PREC, (0.0, 0.0));
    for i in 0..MAX_ITER {
        z = Complex::with_val(PREC, &z * &z + pos);
        

        if *Complex::with_val(PREC, &z * &z).abs().real() > 4
        {
            return i;
        }
    }

    return 0;
}