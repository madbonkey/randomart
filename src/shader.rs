use std::fmt::Write;

use macroquad::material::{load_material, Material, MaterialParams};
use macroquad::miniquad::{ShaderSource, UniformDesc, UniformType};
use rand::Rng;

use crate::ast::{Ast, FnNode, Node, VarNode};
use crate::generate::generate_ast_grammar;

const DEFAULT_VERTEX_SHADER: &str = /*language=glsl*/ "#version 100
  precision mediump float;
  attribute vec3 position;
  attribute vec2 texcoord;

  varying lowp vec2 uv;

  uniform mat4 Model;
  uniform mat4 Projection;

  void main() {
      gl_Position = Projection * Model * vec4(position, 1);
      uv = texcoord;
  }
";

const FRAGMENT_SHADER_START: &str = "#version 100
precision mediump float;
uniform float elapsed;
varying vec2 uv;
void main() {
    float x = uv.x;
    float y = uv.y;
    float t = (sin(elapsed) + 1.0) / 2.0;

    vec3 ast_color = ";

const FRAGMENT_SHADER_END: &str = "
    gl_FragColor = vec4(ast_color, 1);
}
";

pub fn load_shader_material(rng: &mut impl Rng, depth: isize) -> (Ast, Material) {
  let ast = generate_ast_grammar(rng, depth).expect("ast generation");

  info!("generated AST:\n{ast}");

  let vertex_shader = DEFAULT_VERTEX_SHADER.to_string();
  let fragment_shader = compile_fragment_shader(&ast);

  let material = load_material(
    ShaderSource::Glsl {
      vertex: &vertex_shader,
      fragment: &fragment_shader,
    },
    MaterialParams {
      uniforms: vec![
        UniformDesc::new("elapsed", UniformType::Float1),
      ],
      ..Default::default()
    },
  ).expect("load material");

  (ast, material)
}

fn compile_fragment_shader(ast: &Ast) -> String {
  let mut s = String::from(FRAGMENT_SHADER_START);
  let root = ast.root();

  let (r, g, b) = root.nodes();

  let r = compile_fragment_shader_node_temp(r);
  let g = compile_fragment_shader_node_temp(g);
  let b = compile_fragment_shader_node_temp(b);

  write!(&mut s, "vec3({r}, {g}, {b});").expect("write ColorNode");

  s.push_str(FRAGMENT_SHADER_END);

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
      write!(out, "{}", inner.value()).expect("write Node::Scalar");
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
  }
}

