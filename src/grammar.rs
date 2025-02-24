use std::convert::TryFrom;
use std::fmt::{Display, Formatter, Write};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::Path;
use std::str::FromStr;

use rand::Rng;
use rand::seq::IteratorRandom;

use crate::ast::{Ast, ColorNode, FnNode, Node, ScalarNode, VarNode};
use crate::grammar::parse::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum GenError {
  #[error("gen: unknown error")]
  Unknown,
  #[error("gen: rule '{0}' not found")]
  RuleNotFound(RuleId),
  #[error("gen: entry rule not found")]
  EntryRuleNotFound,
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
  pub fn read_from_path<P>(path: &P) -> Result<Self, parse::ParseError> where P: AsRef<Path> {
    let file = OpenOptions::new().read(true).open(path)?;
    Grammar::try_from(file)
  }

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
    let entry_rule = self.find_entry_rule().ok_or_else(|| GenError::EntryRuleNotFound)?;
    tracing::debug!("using rule {:?} as entry rule", &entry_rule.id);
    let node = self.generate_node(rng, &entry_rule.id, depth)?;

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

  fn find_entry_rule(&self) -> Option<&Rule> {
    self.rules.iter().find(|rule| rule.is_entry_rule())
  }
}

impl TryFrom<File> for Grammar {
  type Error = parse::ParseError;

  fn try_from(value: File) -> Result<Self, Self::Error> {
    let mut reader = BufReader::new(value);
    let mut input = String::with_capacity(1024);

    let _ = reader.read_to_string(&mut input)?;

    input.shrink_to_fit();

    input.parse()
  }
}

impl FromStr for Grammar {
  type Err = parse::ParseError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    use nom::Parser;

    match parse::parse_grammar().parse_complete(s) {
      Ok((_, grammar)) => Ok(grammar),
      Err(e) => Err(ParseError::Custom(e.to_string())),
    }
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

  /// Checks that the rule has exactly one branch, which is a `ColorBranch`
  fn is_entry_rule(&self) -> bool {
    if self.branches.len() != 1 {
      false
    } else if let Some(first) = self.branches.first() {
      first.kind.is_color()
    } else {
      false
    }
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
  Const(f32),
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
      Self::Const(c) => {
        f.write_fmt(format_args!("const({c:.})"))?;
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
      BranchKind::Const(c) => {
        Ok(Node::from(ScalarNode::new(*c)))
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
        let g = c.g.generate_node(rng, grammar, depth - 1)?;
        let b = c.b.generate_node(rng, grammar, depth - 1)?;
        Ok(Node::from(ColorNode::new(r, g, b)))
      }
    }
  }

  fn is_terminal(&self) -> bool {
    matches!(self, BranchKind::Var(_))
  }

  fn is_color(&self) -> bool {
    matches!(self, BranchKind::Color(_))
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

pub mod parse {
  use nom::character::{digit1, multispace0};
  use nom::character::complete::alpha1;
  use nom::error::{ErrorKind, FromExternalError};
  use nom::multi::{many1, separated_list0};
  use nom::number::float;
  use nom::Parser;
  use nom::sequence::{delimited, preceded, separated_pair, terminated};

  use crate::grammar::{Branch, BranchKind, ColorBranch, FnBranch, Grammar, Rule, RuleId, VarId};

  #[derive(Debug, thiserror::Error)]
  pub enum ParseError {
    #[error("parse: IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: unknown error")]
    Unknown,
    #[error("parse (custom): {0}")]
    Custom(String),
    #[error("parse: missing entry rule (exactly one color branch) for grammar")]
    MissingEntryRule,
    #[error("parse: expected {0} arguments, got {1}")]
    ArgLen(usize, usize),
    #[error("parse: invalid function {0:?}")]
    UnknownFn(String),
    #[error("parse (nom): {kind:?} in input {input:?}")]
    Nom {
      input: String,
      kind: ErrorKind,
      inner: Option<Box<Self>>,
    },
  }

  impl nom::error::ParseError<ParseIn<'_>> for ParseError {
    fn from_error_kind(input: ParseIn<'_>, kind: ErrorKind) -> Self {
      Self::Nom { input: input.to_string(), kind, inner: None }
    }

    fn append(input: ParseIn<'_>, kind: ErrorKind, other: Self) -> Self {
      Self::Nom { input: input.to_string(), kind, inner: Some(Box::new(other)) }
    }
  }

  impl FromExternalError<ParseIn<'_>, ParseError> for ParseError {
    fn from_external_error(input: ParseIn<'_>, kind: ErrorKind, e: ParseError) -> Self {
      Self::Nom { input: input.to_string(), kind, inner: Some(Box::new(e)) }
    }
  }

  pub type ParseIn<'i> = &'i str;

  pub type ParseRes<'i, O> = nom::IResult<ParseIn<'i>, O, ParseError>;

  macro_rules! ws_char {
  ($chr:expr) => {
    delimited(
      multispace0(),
      nom::character::char($chr),
      multispace0(),
    )
  };
}

  macro_rules! ws_tag {
  ($tag:expr) => {
    delimited(
      multispace0(),
      nom::bytes::tag($tag),
      multispace0(),
    )
  };
}

  fn parse_rule_id<'i>() -> impl Parser<ParseIn<'i>, Output=RuleId, Error=ParseError> {
    delimited(multispace0(), alpha1, multispace0())
      .map(|rule_id: ParseIn<'i>| rule_id.to_string())
  }

  fn parse_rule_id_ref<'i>() -> impl Parser<ParseIn<'i>, Output=RuleId, Error=ParseError> {
    delimited(
      ws_char!('<'),
      parse_rule_id(),
      ws_char!('>'),
    )
  }

  fn parse_bk_rule_id_ref<'i>() -> impl Parser<ParseIn<'i>, Output=BranchKind, Error=ParseError> {
    parse_rule_id_ref().map(|rule_id| BranchKind::Rule(rule_id))
  }

  fn parse_var_id<'i>() -> impl Parser<ParseIn<'i>, Output=VarId, Error=ParseError> {
    preceded(
      ws_char!('@'),
      alpha1.map(|var_id: ParseIn<'i>| var_id.to_string()),
    )
  }

  fn parse_bk_var_id<'i>() -> impl Parser<ParseIn<'i>, Output=BranchKind, Error=ParseError> {
    parse_var_id().map(|var_id| BranchKind::Var(var_id))
  }

  fn ensure_args_len(args: &Vec<BranchKind>, n: usize) -> Result<(), ParseError> {
    if args.len() != n {
      return Err(ParseError::ArgLen(n, args.len()));
    }

    Ok(())
  }

  fn parse_bk_arglist<'i>() -> impl Parser<ParseIn<'i>, Output=Vec<BranchKind>, Error=ParseError> {
    delimited(
      ws_char!('('),
      separated_list0(ws_char!(','), parse_branch_kind),
      ws_char!(')'),
    )
  }

  fn parse_color<'i>() -> impl Parser<ParseIn<'i>, Output=ColorBranch, Error=ParseError> {
    preceded(
      ws_tag!("color"),
      parse_bk_arglist(),
    ).map_res(|args| {
      if args.len() != 3 {
        return Err(ParseError::Custom(format!("expected 3 arguments for color(), got {}", args.len())));
      }

      let mut args = args.into_iter();

      let r = args.next().unwrap();
      let g = args.next().unwrap();
      let b = args.next().unwrap();

      Ok(ColorBranch {
        r: Box::new(r),
        g: Box::new(g),
        b: Box::new(b),
      })
    })
  }

  fn parse_bk_const<'i>() -> impl Parser<ParseIn<'i>, Output=BranchKind, Error=ParseError> {
    preceded(
      ws_tag!("const"),
      delimited(
        ws_char!('('),
        float().map_res(|c| {
          if c < -1.0 || c > 1.0 {
            return Err(ParseError::Custom(format!("constants must be in the range -1..1 (got {c})")));
          }
          Ok(BranchKind::Const(c))
        }),
        ws_char!(')'),
      )
    )
  }

  fn parse_bk_color<'i>() -> impl Parser<ParseIn<'i>, Output=BranchKind, Error=ParseError> {
    parse_color().map(|c| BranchKind::Color(c))
  }

  // TODO: better error reporting on invalid fn names (currently only reports the branch in which the failure occured)
  fn parse_fn_branch<'i>() -> impl Parser<ParseIn<'i>, Output=FnBranch, Error=ParseError> {
    (alpha1, parse_bk_arglist()).map_res(|(f, args)| {
      match f {
        "rand" => {
          Ok::<FnBranch, ParseError>(FnBranch::Random)
        }
        "sin" => {
          ensure_args_len(&args, 1)?;
          let arg = args.into_iter().next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Sin(Box::new(arg)))
        }
        "cos" => {
          ensure_args_len(&args, 1)?;
          let arg = args.into_iter().next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Cos(Box::new(arg)))
        }
        "sqrt" => {
          ensure_args_len(&args, 1)?;
          let arg = args.into_iter().next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Sqrt(Box::new(arg)))
        }
        "add" => {
          ensure_args_len(&args, 2)?;
          let mut args = args.into_iter();
          let l = args.next().unwrap();
          let r = args.next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Add(Box::new(l), Box::new(r)))
        }
        "mul" => {
          ensure_args_len(&args, 2)?;
          let mut args = args.into_iter();
          let l = args.next().unwrap();
          let r = args.next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Mul(Box::new(l), Box::new(r)))
        }
        "mod" => {
          ensure_args_len(&args, 2)?;
          let mut args = args.into_iter();
          let l = args.next().unwrap();
          let r = args.next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Mod(Box::new(l), Box::new(r)))
        }
        "mix" => {
          ensure_args_len(&args, 3)?;
          let mut args = args.into_iter();
          let x = args.next().unwrap();
          let y = args.next().unwrap();
          let a = args.next().unwrap();
          Ok::<FnBranch, ParseError>(FnBranch::Mix(Box::new(x), Box::new(y), Box::new(a)))
        }
        f => {
          Err(ParseError::UnknownFn(f.to_string()))
        }
      }
    })
  }

  fn parse_bk_fn<'i>() -> impl Parser<ParseIn<'i>, Output=BranchKind, Error=ParseError> {
    parse_fn_branch().map(|f| BranchKind::Fn(f))
  }

  fn parse_branch_kind(input: ParseIn) -> ParseRes<BranchKind> {
    nom::branch::alt((
      parse_bk_rule_id_ref(),
      parse_bk_var_id(),
      parse_bk_const(),
      parse_bk_color(),
      parse_bk_fn(),
    )).parse_complete(input)
  }

  fn parse_branch_weight<'i>() -> impl Parser<ParseIn<'i>, Output=u8, Error=ParseError> {
    digit1().map_res(|r: ParseIn<'i>| {
      let weight = u8::from_str_radix(r, 10)
        .map_err(|e| ParseError::Custom(format!("{e}")))?;

      if weight == 0 {
        return Err(ParseError::Custom(format!("Weight cannot be zero")));
      }

      Ok(weight)
    })
  }

  fn parse_weighted_branch_kind<'i>() -> impl Parser<ParseIn<'i>, Output=(u8, BranchKind), Error=ParseError> {
    separated_pair(parse_branch_weight(), ws_char!(':'), parse_branch_kind)
  }

  fn parse_branch<'i>() -> impl Parser<ParseIn<'i>, Output=Branch, Error=ParseError> {
    preceded(
      ws_char!('|'),
      nom::branch::alt((
        parse_branch_kind.map(|kind| Branch { weight: 1, kind }),
        parse_weighted_branch_kind().map(|(weight, kind)| Branch { weight, kind }),
      )),
    )
  }

  fn parse_rule<'i>() -> impl Parser<ParseIn<'i>, Output=Rule, Error=ParseError> {
    separated_pair(
      parse_rule_id(),
      ws_tag!(":="),
      terminated(many1(parse_branch()), ws_char!(';')),
    ).map(|(id, branches)| Rule { id, branches })
  }

  pub(super) fn parse_grammar<'i>() -> impl Parser<ParseIn<'i>, Output=Grammar, Error=ParseError> {
    terminated(many1(parse_rule()), nom::combinator::eof)
      .map_res(|rules| {
        if let None = rules.iter().find(|rule| rule.is_entry_rule()) {
          return Err(ParseError::MissingEntryRule);
        }

        Ok(Grammar { rules })
      })
  }

  #[test]
  #[allow(dead_code)]
  fn test_parsing() {
    const DEV_GRAMMAR: &str = "
A :=
  | color(<B>, <B>, <B>);

B :=
  | rand()
  | 3:@x
  | @y
  | @t;
";

    const SAMPLE_GRAMMAR: &str = "
E :=
  | color(<C>, <C>, <C>);

C :=
  | 2:<A>
  | 3:sin(<C>)
  | 3:sqrt(<C>)
  | 4:add(<C>, <C>)
  | 4:mul(<C>, <C>)
  | mod(<C>, <A>)
  | mix(<C>, <C>, <A>);

A :=
  | 2:rand()
  | 3:@x
  | 3:@y
  | @t;
";

    fn report_parse_error(e: nom::Err<ParseError, ParseError>) {
      match e {
        nom::Err::Incomplete(e) => eprintln!("parse: incomplete ({e:?})"),
        nom::Err::Error(e) | nom::Err::Failure(e) => match &e {
          ParseError::Nom { inner, .. } => match inner {
            None => eprintln!("{e}"),
            Some(inner) => eprintln!("{inner}"),
          },
          e => eprintln!("{e}"),
        },
      }
    }

    match parse_grammar().parse_complete(SAMPLE_GRAMMAR) {
      Ok((_, grammar)) => println!("SAMPLE_GRAMMAR:\n{grammar}"),
      Err(e) => report_parse_error(e),
    }

    match parse_grammar().parse_complete(DEV_GRAMMAR) {
      Ok((_, grammar)) => println!("DEV_GRAMMAR:\n{grammar}"),
      Err(e) => report_parse_error(e),
    }
  }
}

