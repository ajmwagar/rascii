use std::path::{PathBuf, Path};
use structopt::StructOpt;
use image::{DynamicImage, RgbImage};
use std::error::Error;
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// 10 Levels of grayscale
const GSCALE_10: &[char] = &[' ','.',':','-','=','+','*','#','%','@'];
const GSCALE_70: &str = " .\"`^\",:;Il!i~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$";
const GAMMA: f64 = 2.2;

const LOGO: &str = r"
                    _ _
 _ __ __ _ ___  ___(_|_)
| '__/ _` / __|/ __| | |
| | | (_| \__ \ (__| | |
|_|  \__,_|___/\___|_|_|
by Avery Wagar (@ajmwagar)
";

type RasciiOutput = Vec<Vec<(char, RasciiColor)>>;

#[derive(Debug)]
enum RasciiColor {
    RGB(u8, u8, u8),
    Grayscale(u8)
}

impl RasciiColor {
    fn to_grayscale(&self) -> u8 {
        /*
         * Rlin = R^GAMMA,  Glin = G^GAMMA,  Blin = B^GAMMA
         * Y = .2126 * R^GAMMA + .7152 * G^GAMMA + .0722 * B^GAMMA
         * L* = 116 * Y ^ 1/3 - 16
         */

        match self {
            RasciiColor::RGB(r,g,b) => {
                let rlin = (*r as f64).powf(GAMMA);
                let blin = (*b as f64).powf(GAMMA);
                let glin = (*g as f64).powf(GAMMA);

                let y = (0.2126 * rlin) + (0.7152 * glin) + (0.0722 * blin);

                let l = (116.0 * y.powf(1.0 / 3.0) - 16.0) as u8;
                l
            }
            RasciiColor::Grayscale(l) => {
                *l
            }
        }

    }
}

/// Image to ASCII converter
#[derive(StructOpt, Debug)]
#[structopt(name = "rascii")]
struct Opt {
    /// Enable colored output
    #[structopt(short = "c", long = "color")]
    color: bool,

    /// Enable braille mode
    #[structopt(short = "b", long = "braille")]
    braille: bool,

    #[structopt(short = "w", long = "width", default_value = "80")]
    /// Width in characters of the output
    width: u32,

    #[structopt(short = "d", long = "depth", default_value = "70")]
    /// Lumince depth to use. (Number of unique characters)
    depth: u8,

    #[structopt(short = "h", long = "height")]
    /// Height in characters of the output
    height: Option<u32>,

    #[structopt(long = "bg")]
    /// Enable coloring of background chars
    bg: bool,

    /// Path of image file to convert
    #[structopt(name = "IMAGE", parse(from_os_str))]
    image: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut stdout = StandardStream::stdout(ColorChoice::Always);

    // LOGO
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    writeln!(&mut stdout, "{}", LOGO)?;
    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;

    let opt = Opt::from_args();

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)))?;

    // Load image
    write!(&mut stdout, "Loading Image...")?;
    let mut rascii = Rascii::from_opt(&opt)?;
    writeln!(&mut stdout, "   Done!")?;

    // Convert image to ASCII
    write!(&mut stdout, "ASCIIfying...")?;
    let output = rascii.run()?;
    writeln!(&mut stdout, "   Done!\n")?;

    stdout.flush()?;

    stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)))?;

    for row in output {
        for col in row {
            if opt.color {
                let (r,g,b) = match col.1 {
                    RasciiColor::RGB(r,g,b) => (r,g,b),
                    _ => (0,0,0)
                };

                if opt.bg {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(255 - r, 255 - g, 255 -b))))?;
                    stdout.set_color(ColorSpec::new().set_bg(Some(Color::Rgb(r,g,b))))?;
                }
                else {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Rgb(r,g,b))))?;
                }
            }
            write!(&mut stdout, "{}", col.0)?;
            
        }
        writeln!(&mut stdout, "")?;
    }

    Ok(())

}

struct Rascii {
    /// Image
    pub image: RgbImage,
    /// Image dimensions
    pub dim: (u32, u32),
    /// RasciiColored output
    pub color: bool,
    pub depth: u8,
    /// Enable braille mode
    pub braille: bool
}

impl Rascii {
    /// Convert CLI options to a Rascii instance
    pub fn from_opt(opt: &Opt) -> Result<Self, Box<dyn Error>> {
        let im: DynamicImage = image::open(&Path::new(&opt.image))?;
        let im = im.to_rgb();
        let aspect = im.height() as f64 / im.width() as f64;
        let height = match opt.height {
            Some(height) => height,
            None => (opt.width as f64 * aspect) as u32
        };

        Ok(Rascii {
            image: im,
            dim: (opt.width, height),
            depth: opt.depth,
            color: opt.color,
            braille: opt.braille
        })
    }

    /// Convert the image to rascii based on the settings provided
    pub fn run(&mut self) -> Result<RasciiOutput, Box<dyn Error>> {
        let mut output: RasciiOutput = Vec::new(); 
        // Dimensions of image
        let (width, height) = self.image.dimensions();

        // Get tile dimensions
        let tile_w = width / self.dim.0 as u32;
        let tile_h = height / self.dim.1 as u32;

        
        // Convert image to image chunks based on dimensions.
        for ty in 1..self.dim.1 -1 {
            let mut row_tiles = Vec::new();

            for tx in 1..self.dim.0 - 1 {

                let mut tile_pixel_data = Vec::with_capacity((tile_w * tile_h) as usize);
                // per tile
                for px in 0..tile_w {
                    for py in 0..tile_h {
                        let pixel_data = self.image.get_pixel(px + (tx * tile_w), py + (ty * tile_h)).data;

                        let color: RasciiColor;
                        if self.color {
                            color = RasciiColor::RGB(pixel_data[0], pixel_data[1], pixel_data[2])
                        }
                        else {
                            let y = RasciiColor::RGB(pixel_data[0], pixel_data[1], pixel_data[2]).to_grayscale();
                            color = RasciiColor::Grayscale(y as u8);
                        }

                        tile_pixel_data.push(color);

                    }
                }

                let avg: RasciiColor;
                let ascii_char: char;
                if self.color {
                    avg = RasciiColor::RGB(
                       (tile_pixel_data.iter().fold(0usize, |sum, x| {sum + match x { RasciiColor::RGB(r,_,_)=> *r as usize, _ => 0 }}) / tile_pixel_data.len()) as u8,
                       (tile_pixel_data.iter().fold(0usize, |sum, x| {sum + match x { RasciiColor::RGB(_,g,_)=> *g as usize, _ => 0 }}) / tile_pixel_data.len()) as u8,
                       (tile_pixel_data.iter().fold(0usize, |sum, x| {sum + match x { RasciiColor::RGB(_,_,b)=> *b as usize, _ => 0 }}) / tile_pixel_data.len()) as u8
                    );
                    if self.depth > 10 {
                        let index = (avg.to_grayscale() as f64/ 255.0) * 67.0;
                        let chars = GSCALE_70.chars().collect::<Vec<char>>();
                        ascii_char = chars[index as usize];
                    }
                    else {
                        let index = (avg.to_grayscale() as f64/ 255.0) * 9.0;
                        ascii_char = GSCALE_10[index as usize];
                    }
                }
                else {
                    avg = RasciiColor::Grayscale((tile_pixel_data.iter().fold(0usize, |sum, x| {sum + match x { RasciiColor::Grayscale(x)=> *x as usize, _ => 0 } }) as usize / tile_pixel_data.len()) as u8);
                    let x = match avg {
                        RasciiColor::Grayscale(x) => x,
                        _ => 0
                    };
                    if self.depth > 10 {
                        let index = (x as f64/ 255.0) * 67.0;
                        let chars = GSCALE_70.chars().collect::<Vec<char>>();
                        ascii_char = chars[index as usize];
                    }
                    else {
                        let index = (x as f64/ 255.0) * 9.0;
                        ascii_char = GSCALE_10[index as usize];
                    }
                }

                row_tiles.push((
                        ascii_char, avg
                ));
            } 

            output.push(row_tiles);

        }


        // Convert to grayscale or rgb and extract average colors of each chunk
        
        // Figure out background color and character to show

        Ok(output)
    }
}
