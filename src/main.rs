use std::{
    fmt::Debug,
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context, Result};
use rgb::{ComponentBytes, FromSlice};
use structopt::StructOpt;

type Pixel = rgb::RGB<u8>;

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let images: Vec<_> = opt
        .images
        .into_iter()
        .map(Image::new_from_path)
        .collect::<Result<_>>()?;

    ensure!(
        images[1..]
            .iter()
            .all(|im| im.width == images[0].width && im.height == images[0].height),
        "All of the images must have the same width and height."
    );

    fs::create_dir(opt.outdir.clone()).context("Failed to create the output directory.")?;

    for (i, image) in Interpolator::new(images, opt.n_frames).enumerate() {
        image.save(&format!(
            "{}/frame_{:05}.png",
            opt.outdir.to_str().unwrap(),
            i
        ))?;
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
struct Opt {
    /// The images to interpolate between in the output frames
    #[structopt(required(true), min_values(2))]
    images: Vec<PathBuf>,

    /// The directory to save the interpolated frames to
    #[structopt(short, long, default_value = "frames")]
    outdir: PathBuf,

    /// The number of frames between each target image in the output frames
    #[structopt(short, long, default_value = "50")]
    n_frames: usize,
}

/// This structure holds the information for generating each frame of the interpolation.
#[derive(Debug, Clone)]
struct Interpolator {
    /// the images to interpolate between
    images: Vec<Image>,
    /// the "start" image of the current interpolation
    image_no: usize,
    /// the frame of the current interpolation
    frame_no: usize,
    /// the number of steps to do per interpolation
    steps_per_interpolation: usize,
}

impl Interpolator {
    fn new(images: Vec<Image>, steps: usize) -> Self {
        Self {
            images,
            image_no: 0,
            frame_no: 0,
            steps_per_interpolation: steps,
        }
    }
}

impl Iterator for Interpolator {
    type Item = Image;

    fn next(&mut self) -> Option<Self::Item> {
        // increment the frame number
        let frame_num = self.frame_no;
        self.frame_no += 1;

        //dbg!(frame_num, self.frame_no, self.image_no);

        if frame_num >= self.steps_per_interpolation {
            // we are about to generate frame 0 of the next set so set it to 1
            self.frame_no = 1;
            self.image_no += 1;

            // get the next image if it exists and clone it inside the option
            self.images.get(self.image_no).map(Image::clone)
        } else if let Some(start) = self.images.get(self.image_no) {
            if let Some(end) = self.images.get(self.image_no + 1) {
                // interpolate between start and end images
                let mu = frame_num as f64 / self.steps_per_interpolation as f64;

                let data: Vec<_> = start
                    .data
                    .iter()
                    .zip(end.data.iter())
                    .map(|(c1, c2)| smooth(mu, *c1, *c2))
                    .collect();

                Some(
                    Image::new_from_parts(&data, start.width, start.height)
                        .context("Failed to create new image from parts.")
                        .unwrap(),
                )
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// This func takes 2 pixels and a float in [0.0..1.0]
/// which represents how far to interpolate between the two
fn smooth(mu: f64, c1: Pixel, c2: Pixel) -> Pixel {
    let t2 = mu - mu.trunc();
    let t1 = 1.0 - t2;

    Pixel {
        r: (c1.r as f64 * t1 + c2.r as f64 * t2) as u8,
        g: (c1.g as f64 * t1 + c2.g as f64 * t2) as u8,
        b: (c1.b as f64 * t1 + c2.b as f64 * t2) as u8,
    }
}

/// This structure represents an image
/// underneath its just a vector of pixels
/// plus a width and height
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Image {
    data: Vec<Pixel>,
    width: u32,
    height: u32,
}

impl Image {
    fn new_from_path<P>(p: P) -> Result<Self>
    where
        P: AsRef<Path> + Debug,
    {
        let path = p.as_ref();
        let file =
            File::open(path).with_context(|| format!("Failed to open image file {:?}", path))?;
        let decoder = png::Decoder::new(file);
        let (info, mut reader) = decoder
            .read_info()
            .with_context(|| format!("Decoder failed to read information from {:?}", path))?;
        let mut buf = vec![0; info.buffer_size()];
        reader
            .next_frame(&mut buf)
            .with_context(|| format!("Reader failed to read any frames from {:?}", path))?;

        Self::new_from_parts(buf.as_rgb(), info.width, info.height)
    }

    fn new_from_parts(data: &[Pixel], width: u32, height: u32) -> Result<Self> {
        ensure!(
            data.len() as u32 == width * height,
            "Data must match the dimensions given in width and height."
        );

        Ok(Self {
            data: data.into(),
            width,
            height,
        })
    }

    fn save<P>(&self, p: P) -> Result<()>
    where
        P: AsRef<Path> + Debug,
    {
        let path = p.as_ref();
        let file = File::create(path).with_context(|| {
            format!("Failed to create file at {:?} to save the image to.", path)
        })?;
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, self.width, self.height);
        encoder.set_color(png::ColorType::RGB);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .with_context(|| format!("Failed to write the header to image file: {:?}", path))?;

        writer
            .write_image_data(self.data.as_bytes())
            .with_context(|| format!("Failed to write the data to image file: {:?}", path))?;

        Ok(())
    }
}
