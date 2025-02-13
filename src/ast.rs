use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter, Write};
use sha256::digest;

use rand::{rng, Rng};

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
  // pub fn set_var(&mut self, name: impl Into<String>, value: f32) {
  //   self.vars.insert(name.into(), value);
  // }
  //
  // pub fn with_var(mut self, name: impl Into<String>, value: f32) -> Self {
  //   self.set_var(name, value);
  //   self
  // }
}

#[derive(Debug, thiserror::Error)]
pub enum EvalError {
  #[error("eval: unknown error")]
  Unknown,
  #[error("eval: {0}")]
  Message(String),
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
  root: Node,
}

impl Ast {
  pub fn new(root: Node) -> Self {
    Self { root }
  }

  pub fn root(&self) -> &Node {
    &self.root
  }

  pub fn len(&self) -> usize {
    self.root.len()
  }

  pub fn sha256(&self) -> String {
    let input = format!("{}", self.root);
    digest(input)
  }
}

impl Eval for Ast {
  type Out = NodeValue;

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
  Val(ValNode),
  Var(VarNode),
  Color(ColorNode),
  NullaryFn(NullaryFnNode),
  UnaryFn(UnaryFnNode),
  BinaryFn(BinaryFnNode),
}

impl Eval for Node {
  type Out = NodeValue;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    let val = match self {
      Node::Val(node) => NodeValue::Scalar(node.eval(env)?),
      Node::Var(node) => NodeValue::Scalar(node.eval(env)?),
      Node::NullaryFn(node) => NodeValue::Scalar(node.eval(env)?),
      Node::UnaryFn(node) => NodeValue::Scalar(node.eval(env)?),
      Node::BinaryFn(node) => NodeValue::Scalar(node.eval(env)?),
      Node::Color(node) => {
        let (r, g, b) = node.eval(env)?;
        NodeValue::Triple(r, g, b)
      }
    };

    Ok(val)
  }
}

impl Node {
  pub fn var(inner: impl Into<VarNode>) -> Self {
    Self::Var(inner.into())
  }

  pub fn val(value: f32) -> Self {
    Self::Val(ValNode { value })
  }

  pub fn color(inner: ColorNode) -> Self {
    Self::Color(inner)
  }

  pub fn nullary_fn(inner: NullaryFnNode) -> Self {
    Self::NullaryFn(inner)
  }

  pub fn unary_fn(inner: UnaryFnNode) -> Self {
    Self::UnaryFn(inner)
  }

  pub fn binary_fn(inner: BinaryFnNode) -> Self {
    Self::BinaryFn(inner)
  }

  pub fn len(&self) -> usize {
    match self {
      Node::Val(inner) => inner.len(),
      Node::Var(inner) => inner.len(),
      Node::Color(inner) => inner.len(),
      Node::NullaryFn(inner) => inner.len(),
      Node::UnaryFn(inner) => inner.len(),
      Node::BinaryFn(inner) => inner.len(),
    }
  }
}

impl Display for Node {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Node::Val(inner) => Display::fmt(inner, f),
      Node::Var(inner) => Display::fmt(inner, f),
      Node::Color(inner) => Display::fmt(inner, f),
      Node::NullaryFn(inner) => Display::fmt(inner, f),
      Node::UnaryFn(inner) => Display::fmt(inner, f),
      Node::BinaryFn(inner) => Display::fmt(inner, f),
    }
  }
}

#[derive(Debug)]
pub enum NodeValue {
  Scalar(f32),
  Triple(f32, f32, f32),
}

impl NodeValue {
  pub fn scalar_value(&self) -> Option<f32> {
    if let Self::Scalar(val) = self {
      Some(*val)
    } else {
      None
    }
  }

  pub fn triple_value(&self) -> Option<(f32, f32, f32)> {
    if let Self::Triple(a, b, c) = self {
      Some((*a, *b, *c))
    } else {
      None
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
pub struct ValNode {
  value: f32,
}

impl ValNode {
  pub fn value(&self) -> f32 {
    self.value
  }

  pub fn len(&self) -> usize { 1 }
}

impl Eval for ValNode {
  type Out = f32;

  fn eval(&self, _env: &Env) -> Result<Self::Out, EvalError> {
    Ok(self.value)
  }
}

impl Display for ValNode {
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

  pub fn value(&self) -> (&BoxNode, &BoxNode, &BoxNode) {
    (&self.r, &self.g, &self.b)
  }

  pub fn len(&self) -> usize {
    self.r.len() + self.g.len() + self.b.len()
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
pub enum NullaryFnNode {
  Random,
}

impl NullaryFnNode {
  pub fn random() -> Self { Self::Random }

  pub fn len(&self) -> usize { 1 }
}

impl Eval for NullaryFnNode {
  type Out = f32;

  fn eval(&self, _env: &Env) -> Result<Self::Out, EvalError> {
    match self {
      Self::Random => {
        let mut rng = rng();
        Ok(rng.random())
      }
    }
  }
}

impl Display for NullaryFnNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let name = match self {
      Self::Random => "rand",
    };

    f.write_str(name)?;
    f.write_str("()")
  }
}

#[derive(Debug)]
pub enum UnaryFnNode {
  Sqrt(BoxNode),
  Sin(BoxNode),
  Cos(BoxNode),
}

impl UnaryFnNode {
  pub fn sqrt(arg: impl Into<BoxNode>) -> Self {
    Self::Sqrt(arg.into())
  }

  pub fn sin(arg: impl Into<BoxNode>) -> Self {
    Self::Sin(arg.into())
  }

  pub fn cos(arg: impl Into<BoxNode>) -> Self {
    Self::Cos(arg.into())
  }

  pub fn len(&self) -> usize {
    let inner = match self {
      Self::Sqrt(inner) => inner.len(),
      Self::Sin(inner) => inner.len(),
      Self::Cos(inner) => inner.len(),
    };

    1 + inner
  }
}

impl Eval for UnaryFnNode {
  type Out = f32;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    match self {
      Self::Sqrt(node) => {
        let out = node.eval(env)?;

        match out {
          NodeValue::Scalar(val) => Ok(val.sqrt()),
          _ => Err(EvalError::Message(format!("cannot apply sqrt() to node value {out:?}"))),
        }
      },
      Self::Sin(node) => {
        let out = node.eval(env)?;

        match out {
          NodeValue::Scalar(val) => Ok(val.sin()),
          _ => Err(EvalError::Message(format!("cannot apply sin() to node value {out:?}"))),
        }
      },
      Self::Cos(node) => {
        let out = node.eval(env)?;

        match out {
          NodeValue::Scalar(val) => Ok(val.cos()),
          _ => Err(EvalError::Message(format!("cannot apply cos() to node value {out:?}"))),
        }
      }
    }
  }
}

impl Display for UnaryFnNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Sqrt(inner) => f.write_fmt(format_args!("sqrt({inner})")),
      Self::Sin(inner) => f.write_fmt(format_args!("sin({inner})")),
      Self::Cos(inner) => f.write_fmt(format_args!("cos({inner})")),
    }
  }
}

#[derive(Debug)]
pub enum BinaryFnNode {
  Add(BoxNode, BoxNode),
  Sub(BoxNode, BoxNode),
  Mul(BoxNode, BoxNode),
  Div(BoxNode, BoxNode),
  Mod(BoxNode, BoxNode),
}

impl BinaryFnNode {
  pub fn add(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Add(lhs.into(), rhs.into())
  }

  pub fn sub(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Sub(lhs.into(), rhs.into())
  }

  pub fn mul(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Mul(lhs.into(), rhs.into())
  }

  pub fn div(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Div(lhs.into(), rhs.into())
  }

  pub fn modulo(lhs: impl Into<BoxNode>, rhs: impl Into<BoxNode>) -> Self {
    Self::Mod(lhs.into(), rhs.into())
  }

  pub fn len(&self) -> usize {
    let inner = match self {
      Self::Add(lhs, rhs) => lhs.len() + rhs.len(),
      Self::Sub(lhs, rhs) => lhs.len() + rhs.len(),
      Self::Mul(lhs, rhs) => lhs.len() + rhs.len(),
      Self::Div(lhs, rhs) => lhs.len() + rhs.len(),
      Self::Mod(lhs, rhs) => lhs.len() + rhs.len(),
    };

    1 + inner
  }
}

impl Eval for BinaryFnNode {
  type Out = f32;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    match self {
      Self::Add(lhs, rhs) => {
        let lhs = lhs.eval(env)?;
        let rhs = rhs.eval(env)?;

        match lhs {
          NodeValue::Scalar(lhs) => match rhs {
            NodeValue::Scalar(rhs) => Ok(lhs + rhs),
            nv => Err(EvalError::message(format!("rhs of binary op must eval to scalar, got {nv:?}"))),
          },
          nv => Err(EvalError::message(format!("lhs of binary op must eval to scalar, got {nv:?}"))),
        }
      },
      Self::Sub(lhs, rhs) => {
        let lhs = lhs.eval(env)?;
        let rhs = rhs.eval(env)?;

        match lhs {
          NodeValue::Scalar(lhs) => match rhs {
            NodeValue::Scalar(rhs) => Ok(lhs - rhs),
            nv => Err(EvalError::message(format!("rhs of binary op must eval to scalar, got {nv:?}"))),
          },
          nv => Err(EvalError::message(format!("lhs of binary op must eval to scalar, got {nv:?}"))),
        }
      },
      Self::Mul(lhs, rhs) => {
        let lhs = lhs.eval(env)?;
        let rhs = rhs.eval(env)?;

        match lhs {
          NodeValue::Scalar(lhs) => match rhs {
            NodeValue::Scalar(rhs) => Ok(lhs * rhs),
            nv => Err(EvalError::message(format!("rhs of binary op must eval to scalar, got {nv:?}"))),
          },
          nv => Err(EvalError::message(format!("lhs of binary op must eval to scalar, got {nv:?}"))),
        }
      },
      Self::Div(lhs, rhs) => {
        let lhs = lhs.eval(env)?;
        let rhs = rhs.eval(env)?;

        match lhs {
          NodeValue::Scalar(lhs) => match rhs {
            NodeValue::Scalar(rhs) => Ok(lhs / rhs),
            nv => Err(EvalError::message(format!("rhs of binary op must eval to scalar, got {nv:?}"))),
          },
          nv => Err(EvalError::message(format!("lhs of binary op must eval to scalar, got {nv:?}"))),
        }
      },
      Self::Mod(lhs, rhs) => {
        let lhs = lhs.eval(env)?;
        let rhs = rhs.eval(env)?;

        match lhs {
          NodeValue::Scalar(lhs) => match rhs {
            NodeValue::Scalar(rhs) => Ok(lhs % rhs),
            nv => Err(EvalError::message(format!("rhs of binary op must eval to scalar, got {nv:?}"))),
          },
          nv => Err(EvalError::message(format!("lhs of binary op must eval to scalar, got {nv:?}"))),
        }
      },
    }
  }
}

impl Display for BinaryFnNode {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Add(lhs, rhs) => f.write_fmt(format_args!("{lhs} + {rhs}")),
      Self::Sub(lhs, rhs) => f.write_fmt(format_args!("{lhs} - {rhs}")),
      Self::Mul(lhs, rhs) => f.write_fmt(format_args!("{lhs} * {rhs}")),
      Self::Div(lhs, rhs) => f.write_fmt(format_args!("{lhs} / {rhs}")),
      Self::Mod(lhs, rhs) => f.write_fmt(format_args!("{lhs} % {rhs}")),
    }
  }
}
