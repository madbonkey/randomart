use crate::ast::{Ast, BinaryFnNode, ColorNode, Node, NullaryFnNode, UnaryFnNode};

pub fn generate_ast<'a>() -> Result<Ast<'a>, GenError> {
  let root = Node::color(ColorNode::new(
    Node::binary_fn(BinaryFnNode::add(Node::nullary_fn(NullaryFnNode::random()), Node::val(1.0))),
    Node::unary_fn(UnaryFnNode::sqrt(Node::var("x"))),
    Node::unary_fn(UnaryFnNode::sqrt(Node::var("y"))),
  ));

  let ast = Ast::new(root);

  Ok(ast)
}

#[derive(Debug, thiserror::Error)]
pub enum GenError {
  #[error("gen: unknown error")]
  Unknown,
  #[error("gen: {0}")]
  Message(String),
}
