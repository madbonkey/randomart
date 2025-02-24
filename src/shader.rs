use std::fmt::Write;

use rand::Rng;

use macroquad::material::{load_material, Material, MaterialParams};
use macroquad::miniquad::{ShaderSource, UniformDesc, UniformType};

use crate::ast::{Ast, FnNode, Node, VarNode};
use crate::grammar::Grammar;

const DEFAULT_VERTEX_SHADER: &str = /*language=glsl*/
  "#version 100
  precision mediump float;
  attribute vec3 position;
  attribute vec2 texcoord;

  uniform mat4 Model;
  uniform mat4 Projection;

  varying lowp vec2 uv;

  void main() {
      gl_Position = Projection * Model * vec4(position, 1);
      uv = texcoord;
  }
";

const FRAGMENT_SHADER_TEMPLATE: &str = /*language=glsl*/
  "#version 100
  precision mediump float;

  uniform float t;

  varying vec2 uv;

  void main() {
    float x = uv.x;
    float y = uv.y;
    // ---
    // ___INSERT_MARKER___
    // ---
    gl_FragColor.a = 1.0;
  }
";

const FRAGMENT_SHADER_INSERT_MARKER: &str = "// ___INSERT_MARKER___";
const FRAGMENT_SHADER_TIME_VAR: &str = "t";

pub enum TimeWave {
  Sine,
  Triangle,
}

impl TimeWave {
  const PERIOD_SINE: f32 = 6.283_185_5;
  const PERIOD_TRIANGLE: f32 = 4.0;

  #[inline]
  pub fn calc(&self, t: f32) -> f32 {
    match self {
      Self::Sine => Self::norm(t.sin()),
      Self::Triangle => {
        let p = Self::PERIOD_TRIANGLE;
        let x = (t + 1.0) % p;
        let r = if x < p / 2.0 {
          (4.0 / p) * x - 1.0
        } else {
          3.0 - (4.0 / p) * x
        };

        Self::norm(r)
      }
    }
  }

  #[inline(always)]
  pub fn oscillation_period(&self) -> f32 {
    match self {
      Self::Sine => Self::PERIOD_SINE,
      Self::Triangle => Self::PERIOD_TRIANGLE,
    }
  }

  fn norm(t: f32) -> f32 {
    (t + 1.0) / 2.0
  }
}

pub struct ShaderMaterial {
  time_wave: TimeWave,
  current: usize,
  total: usize,
  history: Vec<(Ast, Material)>,
}

impl ShaderMaterial {
  const MAX_HISTORY: usize = 10;
  const HISTORY_SWAP_TAKEOVER: usize = 5;

  pub fn new(grammar: &Grammar, rng: &mut impl Rng, depth: isize) -> Self {
    let top = load_shader_material(grammar, rng, depth);
    let mut history = Vec::with_capacity(Self::MAX_HISTORY);
    history.push(top);

    Self {
      time_wave: TimeWave::Triangle,
      current: 0,
      total: 1,
      history,
    }
  }

  pub fn current_material(&self) -> &Material {
    &self.history[self.current].1
  }

  #[inline]
  pub fn set_elapsed_time(&self, t: f32) {
    self
      .current_material()
      .set_uniform(FRAGMENT_SHADER_TIME_VAR, self.time_wave.calc(t));
  }

  pub fn time_wave(&self) -> &TimeWave {
    &self.time_wave
  }

  pub fn next(&mut self, grammar: &Grammar, rng: &mut impl Rng, depth: isize) {
    if self.history.len() == Self::MAX_HISTORY {
      let prev = std::mem::replace(&mut self.history, Vec::with_capacity(Self::MAX_HISTORY));
      for prev in prev.into_iter().take(Self::HISTORY_SWAP_TAKEOVER) {
        self.history.push(prev);
      }
      self.current = Self::HISTORY_SWAP_TAKEOVER - 1;
    }

    let top = load_shader_material(grammar, rng, depth);
    self.history.push(top);
    self.current = self.history.len() - 1;
    self.total += 1;
  }

  pub fn reset(&mut self, grammar: &Grammar, rng: &mut impl Rng, depth: isize) {
    self.current = 0;
    self.total = 0;
    self.history.clear();
    self.next(grammar, rng, depth);
  }

  pub fn history_forward(&mut self) {
    if self.current == self.history.len() - 1 {
      return;
    }
    self.current += 1;
  }

  pub fn history_backward(&mut self) {
    if self.current == 0 {
      return;
    }
    self.current -= 1;
  }

  pub fn history_info(&self) -> (usize, usize, usize) {
    (self.current + 1, self.history.len(), self.total)
  }

  pub fn debug_dump(&self) {
    // - compute ast hash
    // - use Display impl of ast as content for "<hash>.ast"
    // - use fragment shader as content for "<hash>.frag"
    // - implement some serialization/display for grammar and output
    tracing::warn!("debug_dump() is not implemented yet");
  }
}

fn load_shader_material(grammar: &Grammar, rng: &mut impl Rng, depth: isize) -> (Ast, Material) {
  let mut ast = grammar.generate_ast(rng, depth).expect("ast generation");
  ast.optimize();

  tracing::debug!("Generated AST: {ast}");

  let vertex_shader = DEFAULT_VERTEX_SHADER.to_string();
  let fragment_shader = compile_fragment_shader(&ast);

  tracing::debug!("Generated fragment shader: {fragment_shader}");

  let material = load_material(
    ShaderSource::Glsl {
      vertex: &vertex_shader,
      fragment: &fragment_shader,
    },
    MaterialParams {
      uniforms: vec![UniformDesc::new(
        FRAGMENT_SHADER_TIME_VAR,
        UniformType::Float1,
      )],
      ..Default::default()
    },
  );

  match material {
    Ok(material) => (ast, material),
    Err(e) => {
      tracing::error!("{e}\n{fragment_shader}");
      panic!("Shader compilation failed");
    }
  }
}

fn compile_fragment_shader(ast: &Ast) -> String {
  let (fs_start, fs_end) = FRAGMENT_SHADER_TEMPLATE
    .split_once(FRAGMENT_SHADER_INSERT_MARKER)
    .expect("could not split on fragment shader insert marker");

  let mut s = String::from(fs_start);

  let root = ast.root();

  let (r, g, b) = root.nodes();

  let r = compile_fragment_shader_node_temp(r);
  let g = compile_fragment_shader_node_temp(g);
  let b = compile_fragment_shader_node_temp(b);

  writeln!(&mut s, "gl_FragColor.r = {r};").expect("write red channel expression");
  writeln!(&mut s, "    gl_FragColor.g = {g};").expect("write green channel expression");
  write!(&mut s, "    gl_FragColor.b = {b};").expect("write blue channel expression");

  s.push_str(fs_end);

  s
}

fn compile_fragment_shader_node_temp(node: &Node) -> String {
  let mut temp = String::new();
  compile_fragment_shader_node(node, &mut temp);
  temp
}

fn compile_fragment_shader_node(node: &Node, out: &mut String) {
  match node {
    Node::Scalar(inner) => {
      write!(out, "{:.8}", inner.value()).expect("write Node::Scalar");
    }
    Node::Var(inner) => {
      let c = match inner {
        VarNode::X => 'x',
        VarNode::Y => 'y',
        VarNode::T => 't',
      };
      out.push(c);
    }
    Node::Color(inner) => {
      let (r, g, b) = inner.nodes();

      out.push_str("vec3(");
      out.push_str("abs(");
      compile_fragment_shader_node(r, out);
      out.push(')');
      out.push(',');
      out.push_str("abs(");
      compile_fragment_shader_node(g, out);
      out.push(')');
      out.push(',');
      out.push_str("abs(");
      compile_fragment_shader_node(b, out);
      out.push(')');
      out.push(')');
    }
    Node::Fn(inner) => {
      compile_fragment_shader_node_fn(inner, out);
    }
  }
}

fn compile_fragment_shader_node_fn(node: &FnNode, out: &mut String) {
  match node {
    FnNode::Sqrt(inner) => {
      out.push_str("sqrt(");
      compile_fragment_shader_node(inner, out);
      out.push(')');
    }
    FnNode::Sin(inner) => {
      out.push_str("sin(");
      compile_fragment_shader_node(inner, out);
      out.push(')');
    }
    FnNode::Cos(inner) => {
      out.push_str("cos(");
      compile_fragment_shader_node(inner, out);
      out.push(')');
    }
    FnNode::Add(l, r) => {
      compile_fragment_shader_node(l, out);
      out.push('+');
      compile_fragment_shader_node(r, out);
    }
    FnNode::Mul(l, r) => {
      compile_fragment_shader_node(l, out);
      out.push('*');
      compile_fragment_shader_node(r, out);
    }
    FnNode::Mod(l, r) => {
      out.push_str("mod(");
      compile_fragment_shader_node(l, out);
      out.push(',');
      compile_fragment_shader_node(r, out);
      out.push(')');
    }
    FnNode::Mix(x, y, a) => {
      out.push_str("mix(");
      compile_fragment_shader_node(x, out);
      out.push(',');
      compile_fragment_shader_node(y, out);
      out.push_str(",clamp(");
      compile_fragment_shader_node(a, out);
      out.push_str(", 0.0, 1.0))");
    }
  }
}
