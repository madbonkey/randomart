// #![allow(unused_imports)]
// #![allow(dead_code)]

#[macro_use(info, error)]
extern crate tracing;

use std::io::Write;
use std::process::Stdio;
use std::time::SystemTime;

use clap::{Parser, Subcommand};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use rayon::iter::ParallelIterator;

use crate::ast::{Env, Eval};
use crate::grammar::Grammar;
use crate::shader::ShaderMaterial;

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
    #[arg(short, long, default_value_t = 55)]
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
  Gui {
    /// Depth guide value of the function generation
    #[arg(short, long, default_value_t = 55)]
    depth: u8,

    /// Seed for the RNG
    #[arg(long, default_value_t = 1337)]
    seed: u64,
  },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt().without_time().init();

  let cli: Cli = Cli::parse();

  match cli.command {
    Command::Generate { depth, size, num_images, seed } => {
      command_generate(depth, size, num_images, seed)?;
    }
    Command::Gui { depth, seed } => {
      command_gui(depth, seed)?;
    }
  }

  Ok(())
}

// ---

fn command_generate(depth: u8, size: u16, num_images: u16, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
  info!("Generation seed: {}", seed);
  info!("Generation depth: {}", depth);
  info!("Image size: {}", size);
  info!("Image count: {}", num_images);

  let mut rng = seeded_rng(seed);
  let grammar = Grammar::new_sample();
  let size = size as u32;
  let scale = 1f32 / size as f32;

  for _ in 0..num_images {
    use image::RgbaImage;

    let mut ast = grammar.generate_ast(&mut rng, depth as isize)?;

    ast.optimize();

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

fn command_gui(depth: u8, seed: u64) -> Result<(), Box<dyn std::error::Error>> {
  let conf = macroquad::window::Conf {
    window_title: "Test".to_owned(),
    window_width: 720,
    window_height: 720,
    high_dpi: true,
    window_resizable: true,
    ..Default::default()
  };

  macroquad::Window::from_config(conf, command_gui_main(depth, seed));

  Ok(())
}

async fn command_gui_main(depth: u8, seed: u64) {
  use macroquad::prelude::*;

  const MAX_DEPTH: isize = 150;
  const TAU: f32 = 6.283185307179586;

  let mut depth = depth as isize;

  tracing::info!("Generation seed: {}", seed);
  tracing::info!("Initial depth: {}", depth);

  let mut rng = seeded_rng(seed);

  let grammar = Grammar::new_sample();

  let mut shader_material = ShaderMaterial::new(&grammar, &mut rng, depth);

  let mut show_hud = true;
  let mut paused = false;
  let mut ffmpeg_recording: Option<Ffmpeg> = None;

  let mut ffmpeg_frame_count = 0usize;
  let ffmpeg_target_frame_count = (TAU * 60.).ceil() as usize;

  let mut t = 0f32;
  let mut fps = 60;

  let seed_text = format!("SEED  : {:0>4}", seed);
  let seed_text_bounds = measure_text(seed_text.as_str(), None, 24, 1.0);

  loop {
    // time processing
    if !paused {
      t += if ffmpeg_recording.is_some() { 1.0 / 60. } else { get_frame_time() };
    }

    // input handling
    if let Some(key) = get_last_key_pressed() {
      match key {
        KeyCode::N => {
          shader_material.next(&grammar, &mut rng, depth);
          t = 0f32;
        }
        KeyCode::Left => {
          shader_material.history_backward();
          t = 0f32;
        }
        KeyCode::Right => {
          shader_material.history_forward();
          t = 0f32;
        }
        KeyCode::Space => {
          paused = !paused;
        }
        KeyCode::RightBracket if depth < MAX_DEPTH => {
          let next_depth = depth + if is_key_down(KeyCode::LeftShift) { 10 } else { 1 };
          depth = next_depth.clamp(1, MAX_DEPTH);
        }
        KeyCode::Slash if depth > 1 => {
          let next_depth = depth - if is_key_down(KeyCode::LeftShift) { 10 } else { 1 };
          depth = next_depth.clamp(1, MAX_DEPTH);
        }
        KeyCode::H => {
          show_hud = !show_hud;
        }
        KeyCode::R if ffmpeg_recording.is_none() => {
          let (_, _, hn) = shader_material.history_info();

          tracing::info!("starting recording for seed {seed}, depth {depth}, history index {hn}");

          let Ok(ffmpeg) = Ffmpeg::start_recording(seed, depth, hn) else {
            tracing::error!("failed to start ffmpeg recording");
            break;
          };

          ffmpeg_recording.replace(ffmpeg);
          t = 0f32;
        },
        KeyCode::D => {
          shader_material.debug_dump();
        },
        _ => {}
      }
    }

    // shader updating
    shader_material.set_elapsed_time(t);

    // main scene drawing
    clear_background(BLACK);

    gl_use_material(shader_material.current_material());
    draw_rectangle(0., 0., screen_width(), screen_height(), BLANK);
    gl_use_default_material();

    // recording
    // {
    //   // flush gl draws (is this needed?)
    //   let mut gl = unsafe { get_internal_gl() };
    //   gl.flush();
    // }
    if let Some(ffmpeg) = &mut ffmpeg_recording {
      if ffmpeg_frame_count < ffmpeg_target_frame_count {
        ffmpeg.write_image(get_screen_data());
        ffmpeg_frame_count += 1;
      }
    }

    if ffmpeg_recording.is_some() && ffmpeg_frame_count >= ffmpeg_target_frame_count {
      tracing::info!("recording target frame count reached ({ffmpeg_frame_count} / {ffmpeg_target_frame_count})");
      let ffmpeg = unsafe { ffmpeg_recording.take().unwrap_unchecked() };
      ffmpeg.finish_recording();
      ffmpeg_frame_count = 0;
    }

    // HUD drawing
    if show_hud {
      if ffmpeg_recording.is_none() {
        fps = get_smooth_fps(&mut fps, get_fps());

        draw_rectangle(4., 4., seed_text_bounds.width + 12., 116., Color::from_rgba(0, 0, 0, 190));

        draw_text(seed_text.as_str(), 10., 26., 24., WHITE);
        let (hc, hm, ht) = shader_material.history_info();
        draw_text(format!("HIST  : {hc}/{hm} ({ht})").as_str(), 10., 46., 24., WHITE);
        draw_text(format!("TIME  : {:.2}", (t.sin() + 1.0) / 2.0).as_str(), 10., 66., 24., WHITE);
        draw_text(format!("DEPTH : {:0>4}", depth).as_str(), 10., 86., 24., WHITE);
        draw_text(format!("FPS   : {:0>4}", fps).as_str(), 10., 106., 24., WHITE);
      } else {
        let color = RED;

        let text = "RECORDING ...";
        let text_bounds = measure_text(text, None, 40, 1.0);

        draw_text(
          text,
          screen_width() / 2.0 - (text_bounds.width / 2.0),
          screen_height() / 2.0 - (text_bounds.height / 2.0),
          40.,
          color
        );

        let recording_progress = (1f32 / ffmpeg_target_frame_count as f32) * ffmpeg_frame_count as f32;
        draw_rectangle_lines(
          screen_width() / 2.0 - (text_bounds.width / 2.0),
          screen_height() / 2.0 + (text_bounds.height / 2.0) + 8.,
          text_bounds.width,
          24.0,
          4.0,
          color,
        );
        draw_rectangle(
          screen_width() / 2.0 - (text_bounds.width / 2.0),
          screen_height() / 2.0 + (text_bounds.height / 2.0) + 8.,
          text_bounds.width * recording_progress,
          24.0,
          color,
        );
      }
    }

    next_frame().await
  }
}

// ---

fn get_smooth_fps(fps: &mut i32, current_fps: i32) -> i32 {
  const ALPHA: f32 = 0.025;

  let smooth = (ALPHA * (current_fps as f32) + (1.0 - ALPHA) * (*fps as f32)).floor();
  let smooth = smooth as i32;

  *fps = smooth;

  smooth
}

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

struct Ffmpeg {
  proc: std::process::Child,
}

impl Ffmpeg {
  fn start_recording(seed: u64, depth: isize, hn: usize) -> Result<Self, std::io::Error> {
    let output_path = format!("output/rec_{seed}_{depth}_{hn}.mp4");

    let cmd = std::process::Command::new("ffmpeg")
      // see https://github.com/tsoding/randomart/blob/main/src/ffmpeg_linux.c#L47
      .args(&[
        // global options
        "-loglevel", "verbose",
        "-y",

        // input #1: piped raw video frames
        "-f", "rawvideo",
        "-pix_fmt", "rgba",
        "-s", "1440x1440",
        "-r", "60",
        "-i", "-",

        // output
        "-vf", "vflip",
        "-c:v", "libx264",
        "-vb", "4500k",
        "-pix_fmt", "yuv420p",
      ])
      .arg(output_path)
      .stdin(Stdio::piped())
      .stderr(Stdio::inherit())
      .stdout(Stdio::inherit())
      .spawn();

    let proc = cmd?;

    if !proc.stdin.is_some() {
      return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
    }

    Ok(Self { proc })
  }

  fn write_image(&mut self, image: macroquad::texture::Image) {
    let pipe = unsafe { self.proc.stdin.as_mut().unwrap_unchecked() };
    if let Err(e) = pipe.write_all(image.bytes.as_slice()) {
      tracing::error!("error writing frame: {e}");
    }
  }

  fn finish_recording(self) {
    match self.proc.wait_with_output() {
      Ok(output) => {
        tracing::info!("recording finished with exit status {:?}", output.status);
      }
      Err(e) => {
        tracing::error!("error finishing recording: {e}");
      }
    }
  }
}
