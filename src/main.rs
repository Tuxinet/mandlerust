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
const SKIP_LEN: usize = 10;
const IMG_UPDATE_INTERVAL: u128 = 5000000;
const TEXT_UPDATE_INTERVAL: u128 = 10;
const MAX_ITER: u32 = 30000;
const RES: u32 =  128;
const THREAD_COUNT: u32 = 16;
const PREC: u32 = 512;
const SIZE_STR: &str = "1.3843972363826381531814689023957E-100";

const COORDS: (&str, &str) = ("-1.769797003221398115912725130438998327994233694990687460403123213691394762798997343276853841064249384314392735766803307337049665460755808389013248912202462392189032875057823197659362732380873696894875347373595161248407157606303961329755736109322011630746286872455033371782761711152485963814840985495119858112247809563217001440012335481392958891277404641915770292234769570579423526083615869119473397655144269230554048451408287129839729482745812536821304009849356175786421926754317166054095017677737478909629824101459411484678651540446085496579356154087444768864107144068903495747107840142587494964830790373105466387017637804940200093226948331098336564024101191304782846009251093956024054859850114380942506295799272703040122491695848188554900910110348500660088142142935996917999415780413409072318505658318370986389714499389359946017922054389605549307239863818771223517117958828030858448235437369940778504548655809414086286410278094103602829312453365743012069479897322687170061953674357190866700112517607208995688167519085493168568587128984804788006359593471007812934992508284738813218401067186129216920419813413598507086914378451166514659356530201296859316650641129911816376644360695899121978646876258352313348564609725007303215079702633145899631663504174247063662618357201794491755664334581161063251718266469929996804838236903448728496690668143319600874089515125291764268345534981174976291977855698805746925229399729615225109605245345830722655517606147744507997235610446150765888279849316729036292301646101698262415387848655551453813389172582295590171380746790465457505657035692901532708877919123668700059980974391493025", 
"0.004503808149118977453591027370762118116191847489651632102771075493630536031121753213019458488948070234821894347490919 75232128719902266967792409275276218671134664739202538733880630147980377066457243173553858784184258065626405478713476529 94375685863015511904074453632654407731289619946868720085884280405841386804671414034982833768121999000401733388984737998 50835523341852444210373993799979274072458522457971439601401283190488219977380751679864657632594486990141780409069050808 53533679083210095437351400022620788443700681865056074859184889623921225508741770547501475133877301147491846294015630493 19594413147950329230917914373568299313895801070552430312839787385413077643433921434686758800882730741386718858427487804 80173527152642383437688144097648231731279522222357988455250353865370120443546331395472996006556618614941953429666058354 64910451202485512530423175907298924572677884684325102852936015719933302605823099958630951988450410491580664701963842251 46135190645341340161891884063141465638742680614101092435645795624718302058131414609501281021540435472453888874524109018 14702121578711328524425442226752168664749086242203613749999027884515745350840633982861734634138141253642303937961493945 45838176191438823739844915158113285022936463789829746280707055929391192625872076997627990447836359937976951672647199177 81872517689037585583899463944250055017306480718807197254236743510423432718914191161718864625412816080818679138546319519 75989748541205329675986737013154577653006827691952880225127567357459621316524513472420563020300861878311519895655738526 548297377841163569759373958805028572872157804020781688771768375820124065");
const BASE: i32 = 10;

fn get_pixel_order() -> Vec<u32> {
    let to_return = vec![0u32, SKIP_LEN as u32];

    return to_return;
}

//fn pixel_order(to_return: &Vec<u32>, left:      )

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

    histogram[0] = RES*RES;
    
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

    // skip_len=10
    // [0,1,2,3,4,5,6,7,8,9]
    // [0,5,9,]


    for i in 0..SKIP_LEN {
        for y in (0..bounds.1).skip(i).step_by(SKIP_LEN) {
            work_send.try_send(y).unwrap();
        }
    }

    work_send.unsubscribe();
    drop(result_send);
    println!("Main thread dropped sender channels.");
    println!("Work is queued, waiting for threads to exit...");
    
    indicatif::ProgressStyle::default_spinner();

    let progress_bar = indicatif::ProgressBar::new(RES as u64 * RES as u64);

    let mut counter: usize = 0;
    let mut last_image_update = Instant::now();
    let mut last_text_update = Instant::now();
    for p in result_receive {
        iterations[p.1 * bounds.0 + p.0] = (p.2).0;
        iterations_smooth[p.1 * bounds.0 + p.0] = (p.2).1;

        histogram[(p.2).0 as usize] += 1;
        histogram[0] -= 1;

        counter += 1;
        if last_image_update.elapsed().as_millis() > IMG_UPDATE_INTERVAL {
            make_preview_image(&iterations, &iterations_smooth, &histogram, bounds).unwrap();
            last_image_update = Instant::now();
        }
        if last_text_update.elapsed().as_millis() > TEXT_UPDATE_INTERVAL {
                last_text_update = Instant::now();
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

        img[index] = color.1;
        img[index+1] = color.0;
        img[index+2] = color.2;
    }

    write_image("man.png", &img, bounds).unwrap();
}


use image::{GenericImage, GenericImageView, ImageBuffer, RgbImage, RGB};

fn make_preview_image(iterations: &Vec<u32>, iterations_smooth: &Vec<f32>, histogram: &Vec<u32>, bounds: (usize, usize)) -> Result<(), std::io::Error> {

    let mut img = vec![0u8; bounds.0 * bounds.1 * 3];
    let mut ind: usize = 0;
    for i in (0..RES*RES * 3).step_by(3) {
        //img[i as usize] = ((iterations[i as usize] as f32 - min_val as f32) / (max_val as f32 - min_val as f32) * 254.0) as u8;

        // Skipping cells without iterations to not write over copied cells
        if iterations[ind] == 0 {
            ind += 1;
            continue;
        }

        let color = iter_to_color(&iterations, iterations[ind], &iterations_smooth, &histogram, ind as usize);
        let index = i as usize;
        ind += 1;

        img[index] = color.1;
        img[index+1] = color.0;
        img[index+2] = color.2;

        // Copy color downward in buffer with a maximum of SKIP_LEN - 1 pixels
        // This is to artificially fill the image while the renderer is working. Easier
        // To gage what is going on if the screen is more filled even with shitty pixels

        for j in 1..SKIP_LEN {
            if index + j * bounds.1 * 3 < (RES*RES*3) as usize - 3 {
                let pd = index + j * bounds.1 * 3;
                if iterations[ind + bounds.0 * j] == 0 {
                    img[pd] = img[index];
                    img[pd+1] = img[index+1];
                    img[pd+2] = img[index+2];
                }
            }
        }
    }
    
    write_image("man.png", &img, bounds).unwrap();

    Ok(())
}

fn write_image(filename: &str, pixels: &[u8], bounds: (usize, usize))
    -> Result<(), std::io::Error> {
    //let output = File::create("filename.png")?;

    // Construct a new RGB ImageBuffer with the specified width and height.
    let mut img: RgbImage = ImageBuffer::new(RES, RES);
    //img.put_pixel(20, 20, image::Rgb([255,0,0]));
    img.copy_from_slice(pixels);

    img.save("man.png").unwrap();

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
    total_pixel_count -= histogram[0];

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