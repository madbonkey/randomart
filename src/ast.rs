use std::fmt::{Debug, Display, Formatter, Write};

use sha256::digest;

pub trait Eval {
  type Out: Debug;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError>;
}

#[derive(Debug, Default)]
pub struct Env {
  pub x: f32,
  pub y: f32,
  pub t: f32,
}

impl Env {
  pub fn new(x: f32, y: f32) -> Self {
    Self { x, y, t: 0. }
  }
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
  #[error("eval: unknown error")]
  Unknown,
  #[error("eval: {0}")]
  Message(String),
  #[error("eval: returned non-scalar value")]
  NonScalar,
  #[error("eval: returned non-color value")]
  NonColor,
}

impl EvalError {
  pub fn unknown() -> Self {
    Self::Unknown
  }

  pub fn message(message: impl Into<String>) -> Self {
    Self::Message(message.into())
  }
}

#[derive(Debug)]
pub struct Ast {
  root: ColorNode,
}

impl Ast {
  pub fn new(root: impl Into<ColorNode>) -> Self {
    Self { root: root.into() }
  }

  pub fn root(&self) -> &ColorNode {
    &self.root
  }

  pub fn sha256(&self) -> String {
    let input = format!("{}", self.root);
    digest(input)
  }
}

impl Eval for Ast {
  type Out = <ColorNode as Eval>::Out;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    self.root.eval(env)
  }
}

impl Display for Ast {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.root, f)
  }
}

#[derive(Debug)]
pub enum Node {
  Var(VarNode),
  Scalar(ScalarNode),
  Color(ColorNode),
  Fn(FnNode),
}

impl Eval for Node {
  type Out = NodeValue;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    let val = match self {
      Node::Var(inner) => {
        NodeValue::Scalar(inner.eval(env)?)
      }
      Node::Scalar(inner) => {
        NodeValue::Scalar(inner.eval(env)?)
      }
      Node::Fn(inner) => {
        NodeValue::Scalar(inner.eval(env)?)
      }
      Node::Color(inner) => {
        let (r, g, b) = inner.eval(env)?;
        NodeValue::Color(r, g, b)
      }
    };

    Ok(val)
  }
}

impl Node {}

impl From<VarNode> for Node {
  fn from(value: VarNode) -> Self { Self::Var(value) }
}

impl From<ScalarNode> for Node {
  fn from(value: ScalarNode) -> Self { Self::Scalar(value) }
}

impl From<ColorNode> for Node {
  fn from(value: ColorNode) -> Self { Self::Color(value) }
}

impl From<FnNode> for Node {
  fn from(value: FnNode) -> Self { Self::Fn(value) }
}

impl Display for Node {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Node::Var(inner) => Display::fmt(inner, f),
      Node::Scalar(inner) => Display::fmt(inner, f),
      Node::Color(inner) => Display::fmt(inner, f),
      Node::Fn(inner) => Display::fmt(inner, f),
    }
  }
}

#[derive(Debug)]
pub enum NodeValue {
  Scalar(f32),
  Color(f32, f32, f32),
}

impl NodeValue {
  pub fn scalar_value(&self) -> Result<f32, EvalError> {
    if let Self::Scalar(val) = self {
      Ok(*val)
    } else {
      Err(EvalError::NonScalar)
    }
  }

  pub fn color_value(&self) -> Result<(f32, f32, f32), EvalError> {
    if let Self::Color(a, b, c) = self {
      Ok((*a, *b, *c))
    } else {
      Err(EvalError::NonColor)
    }
  }
}

pub type BoxNode = Box<Node>;

#[derive(Debug)]
pub enum VarNode {
  X,
  Y,
  T,
}

impl VarNode {
  pub fn len(&self) -> usize { 1 }
}

impl Eval for VarNode {
  type Out = f32;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    match self {
      Self::X => Ok(env.x),
      Self::Y => Ok(env.y),
      Self::T => Ok(env.t),
    }
  }
}

impl Display for VarNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      VarNode::X => f.write_char('x'),
      VarNode::Y => f.write_char('y'),
      VarNode::T => f.write_char('t'),
    }
  }
}

#[derive(Debug)]
pub struct ScalarNode {
  value: f32,
}

impl ScalarNode {
  pub fn new(value: f32) -> Self {
    Self { value }
  }

  pub fn value(&self) -> f32 {
    self.value
  }

  pub fn len(&self) -> usize { 1 }
}

impl Eval for ScalarNode {
  type Out = f32;

  fn eval(&self, _env: &Env) -> Result<Self::Out, EvalError> {
    Ok(self.value)
  }
}

impl Display for ScalarNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_fmt(format_args!("{:.5}", self.value))
  }
}

#[derive(Debug)]
pub struct ColorNode {
  r: BoxNode,
  g: BoxNode,
  b: BoxNode,
}

impl ColorNode {
  pub fn new(r: impl Into<BoxNode>, g: impl Into<BoxNode>, b: impl Into<BoxNode>) -> Self {
    Self {
      r: r.into(),
      g: g.into(),
      b: b.into(),
    }
  }

  pub fn nodes(&self) -> (&BoxNode, &BoxNode, &BoxNode) {
    (&self.r, &self.g, &self.b)
  }
}

impl Eval for ColorNode {
  type Out = (f32, f32, f32);

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    let NodeValue::Scalar(r) = self.r.eval(env)? else {
      return Err(EvalError::message("field `r` of color node must evaluate to a scalar value"));
    };

    let NodeValue::Scalar(g) = self.g.eval(env)? else {
      return Err(EvalError::message("field `g` of color node must evaluate to a scalar value"));
    };

    let NodeValue::Scalar(b) = self.b.eval(env)? else {
      return Err(EvalError::message("field `b` of color node must evaluate to a scalar value"));
    };

    Ok((r, g, b))
  }
}

impl Display for ColorNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("  r := ")?;
    Display::fmt(&self.r, f)?;
    f.write_str("\n  g := ")?;
    Display::fmt(&self.g, f)?;
    f.write_str("\n  b := ")?;
    Display::fmt(&self.b, f)?;
    f.write_str("\n")
  }
}

#[derive(Debug)]
pub enum FnNode {
  Sqrt(BoxNode),
  Sin(BoxNode),
  Cos(BoxNode),
  Add(BoxNode, BoxNode),
  Mul(BoxNode, BoxNode),
  Mod(BoxNode, BoxNode),
}

impl FnNode {
  pub fn sqrt(arg: impl Into<BoxNode>) -> Self {
    Self::Sqrt(arg.into())
  }

  pub fn sin(arg: impl Into<BoxNode>) -> Self {
    Self::Sin(arg.into())
  }

  pub fn cos(arg: impl Into<BoxNode>) -> Self {
    Self::Cos(arg.into())
  }

  pub fn add(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Add(lhs.into(), rhs.into())
  }

  pub fn mul(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Mul(lhs.into(), rhs.into())
  }

  pub fn modulo(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Mod(lhs.into(), rhs.into())
  }
}

impl Eval for FnNode {
  type Out = f32;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    match self {
      FnNode::Sqrt(inner) => {
        let v = inner.eval(env)?.scalar_value()?;
        Ok(v.sqrt())
      }
      FnNode::Sin(inner) => {
        let v = inner.eval(env)?.scalar_value()?;
        Ok(v.sin())
      }
      FnNode::Cos(inner) => {
        let v = inner.eval(env)?.scalar_value()?;
        Ok(v.cos())
      }
      FnNode::Add(a, b) => {
        let a = a.eval(env)?.scalar_value()?;
        let b = b.eval(env)?.scalar_value()?;
        Ok(a + b)
      }
      FnNode::Mul(a, b) => {
        let a = a.eval(env)?.scalar_value()?;
        let b = b.eval(env)?.scalar_value()?;
        Ok(a * b)
      }
      FnNode::Mod(a, b) => {
        let a = a.eval(env)?.scalar_value()?;
        let b = b.eval(env)?.scalar_value()?;
        Ok(a % b)
      }
    }
  }
}

impl Display for FnNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      FnNode::Sqrt(inner) => f.write_fmt(format_args!("sqrt({inner})")),
      FnNode::Sin(inner) => f.write_fmt(format_args!("sin({inner})")),
      FnNode::Cos(inner) => f.write_fmt(format_args!("cos({inner})")),
      FnNode::Add(l, r) => f.write_fmt(format_args!("{l} + {r}")),
      FnNode::Mul(l, r) => f.write_fmt(format_args!("{l} * {r}")),
      FnNode::Mod(l, r) => f.write_fmt(format_args!("{l} % {r}")),
    }
  }
}

