// #![allow(unused_imports)]
// #![allow(dead_code)]

#[macro_use(info, error)]
extern crate tracing;

use std::time::SystemTime;

use clap::{Parser, Subcommand};
use rand::{Rng, rng, SeedableRng};
use rand::rngs::StdRng;
use rayon::iter::ParallelIterator;

use crate::ast::{Env, Eval};
use crate::grammar::Grammar;
use crate::shader::load_shader_material;

pub mod ast;
pub mod grammar;
pub mod shader;

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
  Gui {},
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt().without_time().init();

  let cli: Cli = Cli::parse();

  match cli.command {
    Command::Generate { depth, size, num_images, seed } => {
      command_generate(depth, size, num_images, seed)?;
    }
    Command::Gui { .. } => {
      command_gui()?;
    }
  }

  Ok(())
}

// ---

fn command_generate(depth: u8, size: u16, num_images: u16, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
  info!("Image count: {}", num_images);
  info!("Image size: {}", size);
  info!("Generation depth: {}", depth);
  info!("Generation seed: {}", seed);

  let mut rng = seeded_rng(seed);
  let grammar = Grammar::new_sample();
  let size = size as u32;
  let scale = 1f32 / size as f32;

  for _ in 0..num_images {
    use image::RgbaImage;

    let mut ast = grammar.generate_ast(&mut rng, depth as isize)?;

    // info!("ast (pre-opt): {ast}");
    ast.optimize();
    // info!("ast (post-opt): {ast}");

    let ast_hash = ast.sha256();
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

      let Ok((r, g, b)) = ast.eval(&env) else {
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

fn command_gui() -> Result<(), Box<dyn std::error::Error>> {
  let conf = macroquad::window::Conf {
    window_title: "Test".to_owned(),
    window_width: 720,
    window_height: 720,
    high_dpi: true,
    window_resizable: true,
    ..Default::default()
  };

  macroquad::Window::from_config(conf, command_gui_main());

  Ok(())
}

async fn command_gui_main() {
  use macroquad::prelude::*;

  const MAX_DEPTH: isize = 150;

  let mut depth = 75;

  let seed: u64 = {
    let mut rng = rng();
    rng.random()
  };

  tracing::info!("using seed {seed}");

  let mut rng = seeded_rng(seed);

  let grammar = Grammar::new_sample();

  let (mut ast, mut material) = load_shader_material(&grammar, &mut rng, depth);

  let mut show_hud = true;
  let mut paused = false;
  let mut t = 0f32;

  loop {
    if is_key_pressed(KeyCode::R) {
      let (a, m) = load_shader_material(&grammar, &mut rng, depth);
      tracing::info!("regenerated AST:\n{ast}\n---");
      ast = a;
      material = m;
    } else if is_key_pressed(KeyCode::Space) {
      paused = !paused;
    } else if is_key_pressed(KeyCode::RightBracket) && depth < MAX_DEPTH {
      let next_depth = depth + if is_key_down(KeyCode::LeftShift) { 10 } else { 1 };
      depth = next_depth.clamp(1, MAX_DEPTH);
    } else if is_key_pressed(KeyCode::Slash) && depth > 1 {
      let next_depth = depth - if is_key_down(KeyCode::LeftShift) { 10 } else { 1 };
      depth = next_depth.clamp(1, MAX_DEPTH);
    } else if is_key_pressed(KeyCode::A) {
      tracing::info!("AST:\n{ast}");
    } else if is_key_pressed(KeyCode::H) {
      show_hud = !show_hud;
    }

    clear_background(BLACK);


    if !paused {
      t = get_time() as f32;
      material.set_uniform("elapsed", t);
    }

    gl_use_material(&material);
    draw_rectangle(0., 0., screen_width(), screen_height(), BLANK);
    gl_use_default_material();

    if show_hud {
      draw_rectangle(0., 0., 120., 70., Color::from_rgba(0, 0, 0, 190));
      draw_text(format!("FPS : {:0>4}", get_fps()).as_str(), 4., 20., 24., WHITE);
      draw_text(format!("D   : {:0>4}", depth).as_str(), 4., 40., 24., WHITE);
      draw_text(format!("T   : {:.2}", (t.sin() + 1.0) / 2.0).as_str(), 4., 60., 24., WHITE);
    }

    next_frame().await
  }
}

// ---

fn seeded_rng(seed: u64) -> impl Rng {
  let seed: [u8; 8] = seed.to_be_bytes();

  StdRng::from_seed([
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0,
    seed[0], seed[1], seed[2], seed[3],
    seed[4], seed[5], seed[6], seed[7],
  ])
}

