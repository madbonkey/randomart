#![allow(unused_imports)]
#![allow(dead_code)]

pub mod ast;
mod generate;

#[macro_use(info, debug, error, warn)]
extern crate tracing;

use crate::ast::{Ast, BinaryFnNode, ColorNode, Env, Eval, Node, NullaryFnNode, UnaryFnNode};
use crate::generate::generate_ast;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  tracing_subscriber::fmt().without_time().init();

  info!("start");

  {
    let ast = generate_ast()?;
    info!("ast: {ast:?}");

    let env = Env::default()
      .with_var("x", 41.0)
      .with_var("y", 2.3);
    info!("env: {env:?}");

    info!("eval: {:?}", ast.eval(&env)?);
  }

  Ok(())
}
