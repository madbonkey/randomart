#![allow(unused_imports)]
#![allow(dead_code)]

#[macro_use(info, debug, error, warn)]
extern crate tracing;

use std::time::SystemTime;
use clap::{Parser, Subcommand};
use image::RgbaImage;
use rand::{Rng, rng, SeedableRng};
use rand::rngs::StdRng;
use rayon::iter::ParallelIterator;

use crate::ast::{Env, Eval, NodeValue};
use crate::generate::generate_ast;

pub mod ast;
mod generate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Generate random art images and save to disk
  Generate {
    /// Depth guide value of the function generation
    #[arg(short, long, default_value_t = 10)]
    depth: u8,

    /// Size (width and height) of the generated image(s)
    #[arg(short, long, default_value_t = 1024)]
    size: u16,

    /// Number of images to generate
    #[arg(short, long, default_value_t = 10)]
    num_images: u16,

    /// Seed for the RNG
    #[arg(long, default_value_t = 1337)]
    seed: u64,
  },
  /// Launch random art GUI to play around with
  Gui {}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt().without_time().init();

  let cli: Cli = Cli::parse();

  match cli.command {
    Command::Generate {  depth, size, num_images, seed } => {
      command_generate(depth, size, num_images, seed)?;
    }
    Command::Gui { .. } => {
      todo!()
    }
  }

  Ok(())
}

fn seeded_rng(seed: u64) -> impl Rng {
  let seed: [u8; 8] = seed.to_be_bytes();

  StdRng::from_seed([
    0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,
    0,0,0,0,0,0,0,0,
    seed[0], seed[1], seed[2], seed[3],
    seed[4], seed[5], seed[6], seed[7],
  ])
}

fn command_generate(depth: u8, size: u16, num_images: u16, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
  info!("Image count: {}", num_images);
  info!("Image size: {}", size);
  info!("Generation depth: {}", depth);
  info!("Generation seed: {}", seed);

  let mut rng = seeded_rng(seed);
  let depth = depth;
  let size = size as u32;
  let scale = 1f32 / size as f32;

  for _ in 0..num_images {
    use image::{Pixel, RgbaImage};

    let ast = generate_ast(&mut rng, depth as isize)?;
    let ast_hash = ast.sha256();

    info!("ast length: {}", ast.len());
    info!("ast hash: {ast_hash:?}");

    let output = format!("output/ast_{ast_hash}.png");

    let start = SystemTime::now();

    let mut img = RgbaImage::new(size, size);

    fn to_rgb(v: f32) -> u8 {
      let v = v * 255.0;
      (v.abs() % 255.0) as u8
    }

    img.par_enumerate_pixels_mut().for_each(move |(x, y, pix)| {
      let env = Env::new(x as f32 * scale, y as f32 * scale);

      let Ok(NodeValue::Triple(r, g, b)) = ast.eval(&env) else {
        error!("ast did not evaluate to a triple!");
        return;
      };

      let r = to_rgb(r);
      let g = to_rgb(g);
      let b = to_rgb(b);

      pix.0[0] = r;
      pix.0[1] = g;
      pix.0[2] = b;
      pix.0[3] = 255;
    });

    // let mut env = Env::default();
    // for (x, y, pix) in img.enumerate_pixels_mut() {
    //   env.x = x as f32 * scale;
    //   env.y = y as f32 * scale;
    //
    //   let Ok(NodeValue::Triple(r, g, b)) = ast.eval(&env) else {
    //     error!("ast did not evaluate to a triple!");
    //     continue;
    //   };
    //
    //   let r = to_rgb(r);
    //   let g = to_rgb(g);
    //   let b = to_rgb(b);
    //
    //   // println!("({x}, {y}) -> ({r}, {g}, {b})");
    //
    //   pix.0[0] = r;
    //   pix.0[1] = g;
    //   pix.0[2] = b;
    //   pix.0[3] = 255;
    // }

    let dur = SystemTime::now().duration_since(start).unwrap_or_default();
    info!("generation took {dur:?}");

    if let Err(e) = img.save(&output) {
      error!("error saving image: {e}");
    } else {
      info!("saved image to {output}");
    }
  }

  Ok(())
}
