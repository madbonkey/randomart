use rand::Rng;

use crate::ast::{Ast, BinaryFnNode, ColorNode, Node, UnaryFnNode, VarNode};

#[derive(Debug, thiserror::Error)]
pub enum GenError {
  #[error("gen: unknown error")]
  Unknown,
  #[error("gen: {0}")]
  Message(String),
}

pub fn generate_ast_hardcoded(rng: &mut impl Rng, d: isize) -> Result<Ast, GenError> {
  let root = generate_node_e(rng, d)?;
  let ast = Ast::new(root);
  Ok(ast)
}

fn generate_node_e(rng: &mut impl Rng, d: isize) -> Result<Node, GenError> {
  let node = Node::color(
    ColorNode::new(
      generate_node_c(rng, d - 1)?,
      generate_node_c(rng, d - 1)?,
      generate_node_c(rng, d - 1)?,
    )
  );

  Ok(node)
}

fn generate_node_a(rng: &mut impl Rng, _d: isize) -> Result<Node, GenError> {
  let r: f32 = rng.random();

  let node = if r > 0.75 {
    Node::val(rng.random::<f32>() * 2. - 1.)
  } else if r > 0.5 {
    Node::var(VarNode::X)
  } else if r > 0.25 {
    Node::var(VarNode::Y)
  } else {
    Node::var(VarNode::T)
  };

  Ok(node)
}

fn generate_node_c(rng: &mut impl Rng, d: isize) -> Result<Node, GenError> {
  let r: f32 = rng.random();

  let node = if d <= 0 || r > 0.75 {
    generate_node_a(rng, d - 1)?
  } else {
    let r: f32 = rng.random();

    if r > 0.9 {
      Node::unary_fn(UnaryFnNode::sqrt(generate_node_c(rng, d - 1)?))
    } else if r > 0.8 {
      Node::unary_fn(UnaryFnNode::sin(generate_node_c(rng, d - 1)?))
    } else if r > 0.6 {
      Node::binary_fn(BinaryFnNode::add(
        generate_node_c(rng, d - 1)?,
        generate_node_c(rng, d - 1)?,
      ))
    } else if r > 0.4 {
      Node::binary_fn(BinaryFnNode::mul(
        generate_node_c(rng, d - 1)?,
        generate_node_c(rng, d - 1)?,
      ))
    } else if r > 0.3 {
      Node::binary_fn(BinaryFnNode::modulo(
        generate_node_c(rng, d - 1)?,
        generate_node_c(rng, d - 1)?,
      ))
    } else {
      generate_node_a(rng, d - 1)?
    }

    // if r > 0.75 {
    //   Node::unary_fn(UnaryFnNode::sqrt(generate_node_c(rng, d - 1)?))
    // } else if r > 0.5 {
    //   Node::binary_fn(BinaryFnNode::add(
    //     generate_node_c(rng, d - 1)?,
    //     generate_node_c(rng, d - 1)?,
    //   ))
    // } else if r > 0.25 {
    //   Node::binary_fn(BinaryFnNode::modulo(
    //     generate_node_c(rng, d - 1)?,
    //     generate_node_c(rng, d - 1)?,
    //   ))
    // } else {
    //   Node::binary_fn(BinaryFnNode::mul(
    //     generate_node_c(rng, d - 1)?,
    //     generate_node_c(rng, d - 1)?,
    //   ))
    // }
  };

  Ok(node)
}

// ---

// pub struct Grammar {
//   rules: Vec<Rule>,
// }
//
// pub struct Rule {
//   name: String,
//   branches: Vec<Branch>,
// }
//
// pub struct Branch {}
//
// pub enum BranchKind {
//   Rule(String),
//   Terminal
// }
