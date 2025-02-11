use std::collections::BTreeMap;
use std::fmt::{Debug, Display};

use rand::{rng, Rng};

pub trait Eval {
  type Out: Debug;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError>;
}

#[derive(Debug, Default)]
pub struct Env {
  vars: BTreeMap<String, f32>,
}

impl Env {
  pub fn set_var(&mut self, name: impl Into<String>, value: f32) {
    self.vars.insert(name.into(), value);
  }

  pub fn with_var(mut self, name: impl Into<String>, value: f32) -> Self {
    self.set_var(name, value);
    self
  }
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
pub struct Ast<'a> {
  root: Node<'a>,
}

impl<'a> Ast<'a> {
  pub fn new(root: Node<'a>) -> Self {
    Self { root }
  }
}

impl<'a> Eval for Ast<'a> {
  type Out = NodeValue;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    self.root.eval(env)
  }
}

#[derive(Debug)]
pub enum Node<'n> {
  Val(ValNode),
  Var(VarNode<'n>),
  Color(ColorNode<'n>),
  NullaryFn(NullaryFnNode),
  UnaryFn(UnaryFnNode<'n>),
  BinaryFn(BinaryFnNode<'n>),
}

impl<'n> Eval for Node<'n> {
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

impl<'n> Node<'n> {
  pub fn var(name: &'n str) -> Self {
    Self::Var(VarNode { name })
  }

  pub fn val(value: f32) -> Self {
    Self::Val(ValNode { value })
  }

  pub fn color(inner: ColorNode<'n>) -> Self {
    Self::Color(inner)
  }

  pub fn nullary_fn(inner: NullaryFnNode) -> Self {
    Self::NullaryFn(inner)
  }

  pub fn unary_fn(inner: UnaryFnNode<'n>) -> Self {
    Self::UnaryFn(inner)
  }

  pub fn binary_fn(inner: BinaryFnNode<'n>) -> Self {
    Self::BinaryFn(inner)
  }
}

#[derive(Debug)]
pub enum NodeValue {
  Scalar(f32),
  Triple(f32, f32, f32),
}

pub type BoxNode<'n> = Box<Node<'n>>;

#[derive(Debug)]
pub struct VarNode<'n> {
  name: &'n str,
}

impl<'n> Eval for VarNode<'n> {
  type Out = f32;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    if let Some(val) = env.vars.get(self.name) {
      Ok(*val)
    } else {
      Err(EvalError::message(format!("var {:?} not present in env", self.name)))
    }
  }
}

#[derive(Debug)]
pub struct ValNode {
  value: f32,
}

impl Eval for ValNode {
  type Out = f32;

  fn eval(&self, _env: &Env) -> Result<Self::Out, EvalError> {
    Ok(self.value)
  }
}

#[derive(Debug)]
pub struct ColorNode<'n> {
  r: BoxNode<'n>,
  g: BoxNode<'n>,
  b: BoxNode<'n>,
}

impl<'n> ColorNode<'n> {
  pub fn new(r: impl Into<BoxNode<'n>>, g: impl Into<BoxNode<'n>>, b: impl Into<BoxNode<'n>>) -> Self {
    Self {
      r: r.into(),
      g: g.into(),
      b: b.into(),
    }
  }
}

impl<'n> Eval for ColorNode<'n> {
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

#[derive(Debug)]
pub enum NullaryFnNode {
  Random,
}

impl NullaryFnNode {
  pub fn random() -> Self { Self::Random }
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

#[derive(Debug)]
pub enum UnaryFnNode<'n> {
  Sqrt(BoxNode<'n>),
}

impl<'n> UnaryFnNode<'n> {
  pub fn sqrt(arg: impl Into<BoxNode<'n>>) -> Self {
    Self::Sqrt(arg.into())
  }
}

impl<'n> Eval for UnaryFnNode<'n> {
  type Out = f32;

  fn eval(&self, env: &Env) -> Result<Self::Out, EvalError> {
    match self {
      Self::Sqrt(node) => {
        let out = node.eval(env)?;

        match out {
          NodeValue::Scalar(val) => Ok(val.sqrt()),
          _ => Err(EvalError::Message(format!("cannot apply sqrt() to node value {out:?}"))),
        }
      }
    }
  }
}

#[derive(Debug)]
pub enum BinaryFnNode<'n> {
  Add(BoxNode<'n>, BoxNode<'n>),
  Sub(BoxNode<'n>, BoxNode<'n>),
  Mul(BoxNode<'n>, BoxNode<'n>),
  Div(BoxNode<'n>, BoxNode<'n>),
  Mod(BoxNode<'n>, BoxNode<'n>),
}

impl<'n> BinaryFnNode<'n> {
  pub fn add(lhs: impl Into<BoxNode<'n>>, rhs: impl Into<BoxNode<'n>>) -> Self {
    Self::Add(lhs.into(), rhs.into())
  }
}

impl<'n> Eval for BinaryFnNode<'n> {
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
      }
      _ => Err(EvalError::message(format!("binary fn {:?} is not implemented", self))),
    }
  }
}
