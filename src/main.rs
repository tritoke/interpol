use std::{
    fmt::Debug,
    fs::{self, File},
    io::BufWriter,
    path::{Path, PathBuf},
};

use anyhow::{ensure, Context, Result};
use rayon::prelude::*;
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

    let total_frames = (images.len() - 1) * opt.n_frames;
    let n_frames = opt.n_frames;
    let outdir = opt.outdir.to_str().unwrap();

    (0..=total_frames)
        .into_par_iter()
        .map(|n| {
            let image_no = n / n_frames;
            let frame_no = n % n_frames;

            // this shouldn't panic because image_no is derived from the length of the images
            // array and the case for the final "end" image is handled
            let result_image = if frame_no == 0 {
                images[image_no].clone()
            } else {
                let mu = frame_no as f64 / n_frames as f64;
                interpolate(mu, &images[image_no], &images[image_no + 1])?
            };

            result_image.save(&format!("{}/frame_{:09}.png", outdir, n))
        })
        .collect()
}

fn interpolate(mu: f64, im1: &Image, im2: &Image) -> Result<Image> {
    let new_image_data: Vec<_> = im1
        .data
        .iter()
        .zip(im2.data.iter())
        .map(|(s, e)| smooth(mu, *s, *e))
        .collect();
    Image::new_from_parts(new_image_data, im1.width, im1.height)
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

        Self::new_from_parts(buf.as_rgb().into(), info.width, info.height)
    }

    fn new_from_parts(data: Vec<Pixel>, width: u32, height: u32) -> Result<Self> {
        ensure!(
            data.len() as u32 == width * height,
            "Data must match the dimensions given in width and height."
        );

        Ok(Self {
            data,
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
