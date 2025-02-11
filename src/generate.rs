use rand::{Rng, rng};
use crate::ast::{Ast, BinaryFnNode, ColorNode, Node, NullaryFnNode, UnaryFnNode};

#[derive(Debug, thiserror::Error)]
pub enum GenError {
  #[error("gen: unknown error")]
  Unknown,
  #[error("gen: {0}")]
  Message(String),
}

pub fn generate_ast<'a>(d: isize) -> Result<Ast<'a>, GenError> {
  let mut rng = rng();
  let root = generate_node_e(&mut rng, d)?;
  let ast = Ast::new(root);
  Ok(ast)
}

fn generate_node_e<'a>(rng: &mut impl Rng, d: isize) -> Result<Node<'a>, GenError> {
  let node = Node::color(
    ColorNode::new(
      generate_node_c(rng, d - 1)?,
      generate_node_c(rng, d - 1)?,
      generate_node_c(rng, d - 1)?,
    )
  );

  Ok(node)
}

fn generate_node_a<'a>(rng: &mut impl Rng, _d: isize) -> Result<Node<'a>, GenError> {
  let r: f32 = rng.random();

  let node = if r > 0.66 {
    Node::val(rng.random::<f32>() * 2. - 1.)
  } else if r > 0.33 {
    Node::var("x")
  } else {
    Node::var("y")
  };

  Ok(node)
}

fn generate_node_c<'a>(rng: &mut impl Rng, d: isize) -> Result<Node<'a>, GenError> {
  let r: f32 = rng.random();

  let node = if d <= 0 || r > 0.75 {
    generate_node_a(rng, d - 1)?
  } else if r > 0.375 {
    Node::binary_fn(
      BinaryFnNode::add(
        generate_node_c(rng, d - 1)?,
        generate_node_c(rng, d - 1)?,
      )
    )
  } else {
    Node::binary_fn(
      BinaryFnNode::mul(
        generate_node_c(rng, d - 1)?,
        generate_node_c(rng, d - 1)?,
      )
    )
  };

  Ok(node)
}
