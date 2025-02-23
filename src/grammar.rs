use std::fmt::{Display, Formatter, Write};

use rand::Rng;
use rand::seq::IteratorRandom;

use crate::ast::{Ast, ColorNode, FnNode, Node, ScalarNode, VarNode};

#[derive(Debug, thiserror::Error)]
pub enum GenError {
  #[error("gen: unknown error")]
  Unknown,
  #[error("gen: rule '{0}' not found")]
  RuleNotFound(RuleId),
  #[error("gen: variable '{0}' not found")]
  VarNotFound(VarId),
  #[error("gen: {0}")]
  Message(String),
}

#[derive(Debug)]
pub struct Grammar {
  rules: Vec<Rule>,
}

impl Grammar {
  pub fn new_sample() -> Self {
    Self {
      rules: vec![
        Rule {
          id: "E".to_string(),
          branches: vec![
            Branch {
              weight: 1,
              kind: BranchKind::Color(ColorBranch {
                r: Box::new(BranchKind::Rule("C".to_string())),
                g: Box::new(BranchKind::Rule("C".to_string())),
                b: Box::new(BranchKind::Rule("C".to_string())),
              }),
            }
          ],
        },
        Rule {
          id: "C".to_string(),
          branches: vec![
            Branch {
              weight: 2,
              kind: BranchKind::Rule("A".to_string()),
            },
            Branch {
              weight: 4,
              kind: BranchKind::Fn(FnBranch::Add(
                Box::new(BranchKind::Rule("C".to_string())),
                Box::new(BranchKind::Rule("C".to_string())),
              )),
            },
            Branch {
              weight: 4,
              kind: BranchKind::Fn(FnBranch::Mul(
                Box::new(BranchKind::Rule("C".to_string())),
                Box::new(BranchKind::Rule("C".to_string())),
              )),
            },
            Branch {
              weight: 3,
              kind: BranchKind::Fn(FnBranch::Sqrt(
                Box::new(BranchKind::Rule("C".to_string())),
              )),
            },
            Branch {
              weight: 3,
              kind: BranchKind::Fn(FnBranch::Sin(
                Box::new(BranchKind::Rule("C".to_string())),
              )),
            },
            Branch {
              weight: 1,
              kind: BranchKind::Fn(FnBranch::Mix(
                Box::new(BranchKind::Rule("C".to_string())),
                Box::new(BranchKind::Rule("C".to_string())),
                Box::new(BranchKind::Rule("A".to_string())),
              )),
            },
            Branch {
              weight: 1,
              kind: BranchKind::Fn(FnBranch::Mod(
                Box::new(BranchKind::Rule("C".to_string())),
                Box::new(BranchKind::Rule("A".to_string())),
              )),
            },
          ],
        },
        Rule {
          id: "A".to_string(),
          branches: vec![
            Branch {
              weight: 2,
              kind: BranchKind::Fn(FnBranch::Random),
            },
            Branch {
              weight: 3,
              kind: BranchKind::Var("x".to_string()),
            },
            Branch {
              weight: 3,
              kind: BranchKind::Var("y".to_string()),
            },
            Branch {
              weight: 1,
              kind: BranchKind::Var("t".to_string()),
            },
          ],
        },
      ]
    }
  }

  pub fn generate_ast(&self, rng: &mut impl Rng, depth: isize) -> Result<Ast, GenError> {
    let node = self.generate_node(rng, "E", depth)?;

    match node {
      Node::Color(color) => {
        Ok(Ast::new(color))
      }
      _ => {
        Err(GenError::Unknown)
      }
    }
  }

  fn generate_node<R>(&self, rng: &mut impl Rng, initial_rule: R, depth: isize) -> Result<Node, GenError> where R: AsRef<str> {
    // (1)
    let root = self.find_rule(initial_rule.as_ref()).ok_or(GenError::RuleNotFound(initial_rule.as_ref().to_string()))?;

    // (2)
    if depth <= 0 {
      let branch = root.select_first_branch();
      return branch.kind.generate_node(rng, self, depth);
    }

    // (3)
    let branch = root.select_random_branch(rng);

    // (4)
    if branch.is_terminal() {
      return branch.kind.generate_node(rng, self, depth);
    }

    // (5)
    let depth = {
      // The purpose of the 'while' statement in this step is to make sure that
      // the expressions do not grow too fast with respect to depth d.
      let mut d = depth;
      while d >= 0 && rng.random_bool(0.5) {
        d -= 1;
      }
      d
    };

    branch.kind.generate_node(rng, self, depth - 1)
  }

  fn find_rule<R>(&self, rule_id: R) -> Option<&Rule> where R: AsRef<str> {
    self.rules.iter().find(|&rule| rule.id == rule_id.as_ref())
  }
}

impl Display for Grammar {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    for rule in &self.rules {
      Display::fmt(rule, f)?;
      f.write_char('\n')?;
    }
    Ok(())
  }
}

pub type RuleId = String;
pub type VarId = String;

#[derive(Debug)]
pub struct Rule {
  id: RuleId,
  branches: Vec<Branch>,
}

impl Rule {
  fn select_first_branch(&self) -> &Branch {
    assert!(!self.branches.is_empty());
    &self.branches[0]
  }

  // TODO: find a way to pick a branch according to its weight/probability without
  //       allocating a Vec every time!
  fn select_random_branch(&self, rng: &mut impl Rng) -> &Branch {
    assert!(!self.branches.is_empty());

    if self.branches.len() == 1 {
      return self.select_first_branch();
    }

    let weight_sum = self.weight_sum();
    let mut indices = Vec::with_capacity(weight_sum);
    for (i, b) in self.branches.iter().enumerate() {
      for _ in 0..(b.weight) {
        indices.push(i);
      }
    }

    let mut selected_branch = None;

    while selected_branch.is_none() {
      if let Some(i) = indices.iter().choose(rng) {
        selected_branch.replace(&self.branches[*i]);
      }
    }

    // SAFETY: `selected_branch` must be Some() here, since the while loop
    //         would not have terminated otherwise
    unsafe { selected_branch.unwrap_unchecked() }
  }

  fn weight_sum(&self) -> usize {
    self.branches
        .iter()
        .map(|b| b.weight as usize)
        .sum()
  }
}

impl Display for Rule {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.id)?;
    f.write_str(" := \n")?;
    let num_branches = self.branches.len();

    for (i, branch) in self.branches.iter().enumerate() {
      f.write_str("  | ")?;
      Display::fmt(branch, f)?;
      if i + 1 == num_branches {
        f.write_char(';')?;
      }
      f.write_char('\n')?;
    }
    Ok(())
  }
}

#[derive(Debug)]
pub struct Branch {
  kind: BranchKind,
  weight: u8,
}

impl Display for Branch {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    if self.weight > 1 {
      f.write_fmt(format_args!("{}:", self.weight))?;
    }
    Display::fmt(&self.kind, f)?;
    Ok(())
  }
}

impl Branch {
  fn is_terminal(&self) -> bool {
    self.kind.is_terminal()
  }
}

#[derive(Debug)]
pub enum BranchKind {
  Rule(RuleId),
  Var(VarId),
  Fn(FnBranch),
  Color(ColorBranch),
}

impl Display for BranchKind {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Rule(rule) => {
        f.write_char('<')?;
        f.write_str(rule)?;
        f.write_char('>')?;
      }
      Self::Var(var) => {
        f.write_char('@')?;
        f.write_str(var)?;
      }
      Self::Fn(fb) => {
        Display::fmt(fb, f)?;
      }
      Self::Color(cb) => {
        Display::fmt(cb, f)?;
      }
    }
    Ok(())
  }
}

impl BranchKind {
  fn generate_node(
    &self,
    rng: &mut impl Rng,
    grammar: &Grammar,
    depth: isize,
  ) -> Result<Node, GenError> {
    match self {
      BranchKind::Rule(rid) => {
        grammar.generate_node(rng, rid, depth - 1)
      }
      BranchKind::Var(v) => {
        let v = match v.as_str() {
          "x" => VarNode::X,
          "y" => VarNode::Y,
          "t" => VarNode::T,
          v => return Err(GenError::VarNotFound(v.to_string())),
        };
        Ok(Node::from(v))
      }
      BranchKind::Fn(f) => {
        match f {
          FnBranch::Random => {
            let v = rng.random::<f32>();
            Ok(Node::from(ScalarNode::new(v)))
          }
          FnBranch::Sqrt(b) => {
            let b = b.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::sqrt(b)))
          }
          FnBranch::Sin(b) => {
            let b = b.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::sin(b)))
          }
          FnBranch::Cos(b) => {
            let b = b.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::cos(b)))
          }
          FnBranch::Add(l, r) => {
            let l = l.generate_node(rng, grammar, depth - 1)?;
            let r = r.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::add(l, r)))
          }
          FnBranch::Mul(l, r) => {
            let l = l.generate_node(rng, grammar, depth - 1)?;
            let r = r.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::mul(l, r)))
          }
          FnBranch::Mod(l, r) => {
            let l = l.generate_node(rng, grammar, depth - 1)?;
            let r = r.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::modulo(l, r)))
          }
          FnBranch::Mix(x, y, a) => {
            let x = x.generate_node(rng, grammar, depth - 1)?;
            let y = y.generate_node(rng, grammar, depth - 1)?;
            let a = a.generate_node(rng, grammar, depth - 1)?;
            Ok(Node::from(FnNode::mix(x, y, a)))
          }
        }
      }
      BranchKind::Color(c) => {
        let r = c.r.generate_node(rng, grammar, depth - 1)?;
        let g = c.r.generate_node(rng, grammar, depth - 1)?;
        let b = c.r.generate_node(rng, grammar, depth - 1)?;
        Ok(Node::from(ColorNode::new(r, g, b)))
      }
    }
  }

  fn is_terminal(&self) -> bool {
    matches!(self, BranchKind::Var(_))
  }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ColorBranch {
  r: Box<BranchKind>,
  g: Box<BranchKind>,
  b: Box<BranchKind>,
}

impl Display for ColorBranch {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    f.write_str("color(")?;
    Display::fmt(&self.r, f)?;
    f.write_str(", ")?;
    Display::fmt(&self.g, f)?;
    f.write_str(", ")?;
    Display::fmt(&self.b, f)?;
    f.write_char(')')?;
    Ok(())
  }
}

#[derive(Debug)]
pub enum FnBranch {
  Random,
  Sqrt(Box<BranchKind>),
  Sin(Box<BranchKind>),
  Cos(Box<BranchKind>),
  Add(Box<BranchKind>, Box<BranchKind>),
  Mul(Box<BranchKind>, Box<BranchKind>),
  Mod(Box<BranchKind>, Box<BranchKind>),
  Mix(Box<BranchKind>, Box<BranchKind>, Box<BranchKind>),
}

impl Display for FnBranch {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Random => f.write_str("rand()")?,
      Self::Sqrt(inner) => {
        f.write_str("sqrt(")?;
        Display::fmt(inner, f)?;
        f.write_char(')')?;
      }
      Self::Sin(inner) => {
        f.write_str("sin(")?;
        Display::fmt(inner, f)?;
        f.write_char(')')?;
      }
      Self::Cos(inner) => {
        f.write_str("cos(")?;
        Display::fmt(inner, f)?;
        f.write_char(')')?;
      }
      Self::Add(l, r) => {
        f.write_str("add(")?;
        Display::fmt(l, f)?;
        f.write_str(", ")?;
        Display::fmt(r, f)?;
        f.write_char(')')?;
      }
      Self::Mul(l, r) => {
        f.write_str("mul(")?;
        Display::fmt(l, f)?;
        f.write_str(", ")?;
        Display::fmt(r, f)?;
        f.write_char(')')?;
      }
      Self::Mod(l, r) => {
        f.write_str("mod(")?;
        Display::fmt(l, f)?;
        f.write_str(", ")?;
        Display::fmt(r, f)?;
        f.write_char(')')?;
      }
      Self::Mix(x, y, a) => {
        f.write_str("mix(")?;
        Display::fmt(x, f)?;
        f.write_str(", ")?;
        Display::fmt(y, f)?;
        f.write_str(", ")?;
        Display::fmt(a, f)?;
        f.write_char(')')?;
      }
    }
    Ok(())
  }
}

// E :=
//   | color(<C>, <C>, <C>);
//
// C :=
//   | 2:<A>
//   | 3:sin(<C>)
//   | 3:sqrt(<C>)
//   | 4:add(<C>, <C>)
//   | 4:mul(<C>, <C>)
//   | mod(<C>, <A>);
//   | mix(<C>, <C>, <A>)
//
// A :=
//   | 2:rand()
//   | 3:@x
//   | 3:@y
//   | @t;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
  #[error("parse: unknown error")]
  Unknown,
}

fn try_fn_call_from_str(input: &str) -> Result<FnBranch, ParseError> {
  let input = input.trim();

  if input == "rand()" {
    return Ok(FnBranch::Random);
  } else if input.starts_with("sin") {}

  todo!()
}

fn try_rule_id_from_str(input: &str) -> Result<RuleId, ParseError> {
  let input = input.trim();

  let mut cs = input.chars();

  let Some(c) = cs.next() else {
    return Err(ParseError::Unknown);
  };

  if c != '<' {
    return Err(ParseError::Unknown);
  }

  let mut rule = RuleId::with_capacity(12);

  for c in cs {
    if c.is_alphabetic() {
      rule.push(c);
    } else if c == '>' {
      break;
    } else {
      return Err(ParseError::Unknown);
    }
  }

  if rule.len() == 0 {
    return Err(ParseError::Unknown);
  }

  Ok(rule)
}

fn try_var_id_from_str(input: &str) -> Result<VarId, ParseError> {
  let input = input.trim();

  if input.len() != 2 {
    return Err(ParseError::Unknown);
  }

  let mut cs = input.chars();

  if cs.next().filter(|c| *c == '@').is_none() {
    return Err(ParseError::Unknown);
  }

  let var_id = match cs.next() {
    Some('x') => VarId::from('x'),
    Some('y') => VarId::from('y'),
    Some('t') => VarId::from('t'),
    _ => return Err(ParseError::Unknown),
  };

  Ok(var_id)
}

#[test]
fn test_parsing() {
  let rule = try_rule_id_from_str(" <entry>    ");
  println!("{rule:?}");

  let var = try_var_id_from_str(" @t    ");
  println!("{var:?}");
}
