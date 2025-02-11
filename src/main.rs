#![allow(unused_imports)]
#![allow(dead_code)]

#[macro_use(info, debug, error, warn)]
extern crate tracing;

use image::RgbaImage;
use rand::{rng, Rng};
use crate::ast::{Env, Eval, NodeValue};
use crate::generate::generate_ast;

pub mod ast;
mod generate;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt().without_time().init();

  info!("start");

  // let mut rng = rng();
  // for _ in 0..10 {
  //   println!("{}", rng.random::<f32>() * 2. - 1.);
  // }
  //
  // return Ok(());

  for _ in 0..20 {
    use image::{Pixel, Rgba, RgbaImage};

    let depth = 10;
    let size = 400u32;
    let scale = 1f32 / size as f32;

    let ast = generate_ast(depth)?;
    let ast_hash = ast.sha256();

    info!("ast: {ast}");
    info!("ast length: {}", ast.len());
    info!("ast hash: {ast_hash:?}");

    let output = format!("output/ast_{ast_hash}.png");

    // return Ok(());

    let mut env = Env::default()
      .with_var("x", 0.0)
      .with_var("y", 0.0);

    let mut img = RgbaImage::new(size, size);

    fn to_rgb(v: f32) -> u8 {
      let v = v * 255.0;
      v.clamp(0.0, 255.0) as u8
    }

    for (x, y, pix) in img.enumerate_pixels_mut() {
      env.set_var("x", x as f32 * scale);
      env.set_var("y", y as f32 * scale);

      let Ok(NodeValue::Triple(r, g, b)) = ast.eval(&env) else {
        error!("ast did not evaluate to a triple!");
        continue;
      };

      let r = to_rgb(r);
      let g = to_rgb(g);
      let b = to_rgb(b);

      // println!("({x}, {y}) -> ({r}, {g}, {b})");

      pix.0[0] = r;
      pix.0[1] = g;
      pix.0[2] = b;
      pix.0[3] = 255;
    }

    if let Err(e) = img.save(&output) {
      error!("error saving image: {e}");
    } else {
      info!("saved image to {output}");
    }
  }

  Ok(())
}
