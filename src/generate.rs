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
  pub fn generate(&self, rng: &mut impl Rng, initial_rule: RuleId, depth: isize) -> Result<Node, GenError> {
    // (1)
    let root = self.find_rule(initial_rule).ok_or(GenError::RuleNotFound(initial_rule))?;

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

  fn find_rule(&self, rule_id: RuleId) -> Option<&Rule> {
    self.rules.iter().find(|&rule| rule.id == rule_id)
  }
}

pub type RuleId = char;
pub type VarId = char;

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

#[derive(Debug)]
pub struct Branch {
  kind: BranchKind,
  weight: u8,
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

impl BranchKind {
  fn generate_node(&self, rng: &mut impl Rng, grammar: &Grammar, depth: isize) -> Result<Node, GenError> {
    match self {
      BranchKind::Rule(rid) => {
        grammar.generate(rng, *rid, depth - 1)
      }
      BranchKind::Var(v) => {
        let v = match v {
          'x' => VarNode::X,
          'y' => VarNode::Y,
          't' => VarNode::T,
          v => return Err(GenError::VarNotFound(*v)),
        };
        Ok(Node::from(v))
      }
      BranchKind::Fn(f) => {
        match f {
          FnBranch::Random => {
            let v = rng.random::<f32>();
            Ok(Node::from(ScalarNode::new(v)))
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

#[derive(Debug)]
pub enum FnBranch {
  Random,
  Add(Box<BranchKind>, Box<BranchKind>),
  Mul(Box<BranchKind>, Box<BranchKind>),
  Sqrt(Box<BranchKind>),
  Sin(Box<BranchKind>),
  Cos(Box<BranchKind>),
}

pub fn generate_ast_grammar(rng: &mut impl Rng, d: isize) -> Result<Ast, GenError> {
  let grammar = Grammar {
    rules: vec![
      Rule {
        id: 'E',
        branches: vec![
          Branch {
            weight: 1,
            kind: BranchKind::Color(ColorBranch {
              r: Box::new(BranchKind::Rule('C')),
              g: Box::new(BranchKind::Rule('C')),
              b: Box::new(BranchKind::Rule('C')),
            }),
          }
        ],
      },
      Rule {
        id: 'C',
        branches: vec![
          Branch {
            weight: 2,
            kind: BranchKind::Rule('A'),
          },
          Branch {
            weight: 3,
            kind: BranchKind::Fn(FnBranch::Add(
              Box::new(BranchKind::Rule('C')),
              Box::new(BranchKind::Rule('C')),
            )),
          },
          Branch {
            weight: 3,
            kind: BranchKind::Fn(FnBranch::Mul(
              Box::new(BranchKind::Rule('C')),
              Box::new(BranchKind::Rule('C')),
            )),
          },
          Branch {
            weight: 3,
            kind: BranchKind::Fn(FnBranch::Sqrt(
              Box::new(BranchKind::Rule('C')),
            )),
          },
          Branch {
            weight: 3,
            kind: BranchKind::Fn(FnBranch::Sin(
              Box::new(BranchKind::Rule('C')),
            )),
          },
        ],
      },
      Rule {
        id: 'A',
        branches: vec![
          Branch {
            weight: 1,
            kind: BranchKind::Fn(FnBranch::Random),
          },
          Branch {
            weight: 1,
            kind: BranchKind::Var('x'),
          },
          Branch {
            weight: 1,
            kind: BranchKind::Var('y'),
          },
          Branch {
            weight: 1,
            kind: BranchKind::Var('t'),
          },
        ],
      },
    ]
  };
  let node = grammar.generate(rng, 'E', d)?;

  match node {
    Node::Color(color) => {
      Ok(Ast::new(color))
    }
    _ => {
      Err(GenError::Unknown)
    }
  }
}

// ---

// pub fn generate_ast_hardcoded(rng: &mut impl Rng, d: isize) -> Result<Ast, GenError> {
//   let root = generate_node_e(rng, d)?;
//   let ast = Ast::new(root);
//   Ok(ast)
// }
//
// fn generate_node_e(rng: &mut impl Rng, d: isize) -> Result<ColorNode, GenError> {
//   Ok(ColorNode::new(
//     generate_node_c(rng, d - 1)?,
//     generate_node_c(rng, d - 1)?,
//     generate_node_c(rng, d - 1)?,
//   ))
// }
//
// fn generate_node_a(rng: &mut impl Rng, _d: isize) -> Result<Node, GenError> {
//   let r: f32 = rng.random();
//
//   let node = if r > 0.75 {
//     Node::from(ScalarNode::new(rng.random::<f32>() * 2. - 1.))
//   } else if r > 0.5 {
//     Node::from(VarNode::X)
//   } else if r > 0.25 {
//     Node::from(VarNode::Y)
//   } else {
//     Node::from(VarNode::T)
//   };
//
//   Ok(node)
// }
//
// fn generate_node_c(rng: &mut impl Rng, d: isize) -> Result<Node, GenError> {
//   let r: f32 = rng.random();
//
//   let node = if d <= 0 || r > 0.75 {
//     generate_node_a(rng, d - 1)?
//   } else {
//     let r: f32 = rng.random();
//
//     if r > 0.9 {
//       Node::from(FnNode::sqrt(generate_node_c(rng, d - 1)?))
//     } else if r > 0.8 {
//       Node::from(FnNode::sin(generate_node_c(rng, d - 1)?))
//     } else if r > 0.6 {
//       Node::from(FnNode::add(
//         generate_node_c(rng, d - 1)?,
//         generate_node_c(rng, d - 1)?,
//       ))
//     } else if r > 0.4 {
//       Node::from(FnNode::mul(
//         generate_node_c(rng, d - 1)?,
//         generate_node_c(rng, d - 1)?,
//       ))
//     } else if r > 0.3 {
//       Node::from(FnNode::modulo(
//         generate_node_c(rng, d - 1)?,
//         generate_node_c(rng, d - 1)?,
//       ))
//     } else {
//       generate_node_a(rng, d - 1)?
//     }
//
//     // if r > 0.75 {
//     //   Node::unary_fn(UnaryFnNode::sqrt(generate_node_c(rng, d - 1)?))
//     // } else if r > 0.5 {
//     //   Node::binary_fn(BinaryFnNode::add(
//     //     generate_node_c(rng, d - 1)?,
//     //     generate_node_c(rng, d - 1)?,
//     //   ))
//     // } else if r > 0.25 {
//     //   Node::binary_fn(BinaryFnNode::modulo(
//     //     generate_node_c(rng, d - 1)?,
//     //     generate_node_c(rng, d - 1)?,
//     //   ))
//     // } else {
//     //   Node::binary_fn(BinaryFnNode::mul(
//     //     generate_node_c(rng, d - 1)?,
//     //     generate_node_c(rng, d - 1)?,
//     //   ))
//     // }
//   };
//
//   Ok(node)
// }

// ---
