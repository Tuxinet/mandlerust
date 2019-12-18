extern crate image;
extern crate num;
extern crate multiqueue;
extern crate indicatif;
extern crate rug;
extern crate palette;
use palette::{Hsv};
use image::ColorType;
use image::ImageRgb8;
use image::png::PNGEncoder;
use std::fs::File;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Instant};
use rug::{Float, Complex};
use rug::float::Round;
use std::cmp::Ordering;
const MAX_ITER: u32 = 30000;
const RES: u32 = 128;
const THREAD_COUNT: u32 = 4;
const PREC: u32 = 160;
const SIZE_STR: &str = "1.3843972363826381531814689023957E-40";

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

    let mut histogram = vec![0u32; MAX_ITER as usize];
    let mut iterations = vec![0u32; bounds.0 * bounds.1];
    let mut iterations_smooth = vec![0f32; bounds.0 * bounds.1];
    
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
        iterations[p.1 * bounds.0 + p.0] = (p.2).0;
        iterations_smooth[p.1 * bounds.0 + p.0] = (p.2).1;

        histogram[(p.2).0 as usize] += 1;

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
    let mut min_val: u32 = MAX_ITER;
    
    for i in 0..RES*RES {
        if iterations[i as usize] > max_val {
            max_val = iterations[i as usize];
        }
        if iterations[i as usize] < min_val {
            min_val = iterations[i as usize];
        }
        
    }

    println!("Max value: {}, Min value: {}", max_val, min_val);
    
    let mut img = vec![0u8; bounds.0 * bounds.1 * 3];
    let mut ind: usize = 0;
    for i in (0..RES*RES * 3).step_by(3) {
        //img[i as usize] = ((iterations[i as usize] as f32 - min_val as f32) / (max_val as f32 - min_val as f32) * 254.0) as u8;

        let color = iter_to_color(&iterations, iterations[ind], &iterations_smooth, &histogram, ind as usize);
        let index = i as usize;
        ind += 1;

        img[index] = color.0;
        img[index+1] = color.1;
        img[index+2] = color.2;
    }

    write_image("man.png", &img, bounds).unwrap();
}


use image::{GenericImage, GenericImageView, ImageBuffer, RgbImage, RGB};

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize))
    -> Result<(), std::io::Error> {
    //let output = File::create("filename.png")?;

    // Construct a new RGB ImageBuffer with the specified width and height.
    let mut img: RgbImage = ImageBuffer::new(RES, RES);
    //img.put_pixel(20, 20, image::Rgb([255,0,0]));
    img.copy_from_slice(pixels);

    img.save("man.png");

    //let encoder = PNGEncoder::new(output);
    //encoder.encode(&pixels,
                    //bounds.0 as u32, bounds.1 as u32,
                    //ColorType::Gray(8))?;
    
    
    Ok(())
}

fn iter_to_color(iterations: &Vec<u32>, iter: u32, iter_smooth: &Vec<f32>, histogram: &Vec<u32>, idx: usize) -> (u8, u8, u8) {
    let mut total_pixel_count: u32 = RES*RES;

    let mut total: u32 = 0;
    for i in 1..(iter) {
        total += histogram[i as usize];
    }

    total_pixel_count -= histogram[(MAX_ITER as usize) - 1];

    let mut color_value: (u8, u8, u8) = (0, 0, 0);
    if iter == 0 {
        return (0, 0, 0);
    }
    else if iter < MAX_ITER - 1 {
        let total_p: u32 = total + histogram[(iter) as usize];

        let val1: u8 = ((total as f32 / total_pixel_count as f32) * 254.0) as u8;
        let val2: u8 = ((total_p as f32 / total_pixel_count as f32) * 254.0) as u8;

        // Lerp between the values
        let diff: u8 = val2 - val1;
        let offset: f32 = diff as f32 * iter_smooth[idx];

        //println!("{}, {}", iter_smooth[idx], offset);

        color_value.0 = (val1 as u8) + (offset as u8) as u8;
        //color_value.0 = (255.0 * iter_smooth[idx]) as u8;
        //color_value.1 = (255.0 * iter_smooth[idx]) as u8;

        //let c = palette::Hsv{hue: (val1 as u8) + (offset as u8) as u8,
    } 
    else {
        color_value.0 = 0;
    }

    return color_value;
}

fn pixel_to_position(middle: (Float, Float), size: Float, x: usize, y: usize) -> (Float, Float) {
    let step_size:Float = (size.clone()) / RES;
    return (middle.0 - size.clone() / 2 + Float::with_val(PREC, x) * step_size.clone(), middle.1 - size.clone() / 2 + Float::with_val(PREC, y) * step_size);
}
//n + 1 - log(log2(abs(z)))
//log(log(length(z))/log(B))/log(2.0);
// iter_count - (log (log (modulus)))/ log (2.0)
fn calc_pos(pos: &Complex) -> (u32, f32) {
    let mut z = Complex::with_val(PREC, (0.0, 0.0));
    let mut old_z = z.clone();
    for i in 0..(MAX_ITER) {
        old_z = z.clone();
        //z = Complex::with_val(PREC, &z * &z + pos);

        z = z.clone().mul_add(&z, pos);
        
        let esc = 2000.0;

        if Float::with_val(PREC,z.real() * z.real() + z.imag() * z.imag()) > esc * esc
        {
            let mut log2 = Float::with_val(PREC, 2.0);
            log2 = log2.ln();
            //return (i, ((z.abs().clone().real().clone().log2().log10().to_f32())));
            //return (i, ((((z.abs().clone().real().clone().clone().ln().ln() / b)) / log2).to_f32()));
            let a = old_z.clone().abs();
            let b = a.real().clone();
            let c = b.ln();
            let d = c.ln();

            let e = Float::with_val(PREC, esc);
            let f = e.ln();
            let g = f.ln();
            return (i, ((((g - d) / log2).to_f32())));
        }
    }

    return (MAX_ITER - 1, 0.0);
}