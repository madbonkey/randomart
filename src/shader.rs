use std::fmt::Write;
use macroquad::material::{load_material, Material, MaterialParams};
use macroquad::miniquad::{ShaderSource, UniformDesc, UniformType};
use rand::Rng;

use crate::ast::{Ast, BinaryFnNode, BoxNode, Node, UnaryFnNode, VarNode};
use crate::generate::generate_ast;

const DEFAULT_VERTEX_SHADER: &'static str = /*language=glsl*/ "#version 100
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

const FRAGMENT_SHADER_START: &'static str = "#version 100
precision mediump float;
uniform float elapsed;
varying vec2 uv;
void main() {
    float x = uv.x;
    float y = uv.y;
    float t = (sin(elapsed) + 1.0) / 2.0;

    vec3 ast_color = ";

const FRAGMENT_SHADER_END: &'static str = "
    gl_FragColor = vec4(ast_color, 1);
}
";

pub fn load_shader_material(rng: &mut impl Rng, depth: isize) -> (Ast, Material) {
  let ast = generate_ast(rng, depth).expect("ast generation");

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
    }
  ).expect("load material");

  (ast, material)
}

fn compile_fragment_shader(ast: &Ast) -> String {
  let mut s = String::from(FRAGMENT_SHADER_START);
  let root = ast.root();

  compile_fragment_shader_node(root, &mut s);

  s.push(';');
  s.push_str(FRAGMENT_SHADER_END);

  s
}

fn compile_fragment_shader_node(node: &Node, out: &mut String) {
  match node {
    Node::Val(inner) => {
      write!(out, "{}", inner.value()).expect("write Node::Val");
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
      let (r, g, b) = inner.value();

      out.push_str("vec3(");
      compile_fragment_shader_node(r, out);
      out.push_str(",");
      compile_fragment_shader_node(g, out);
      out.push_str(",");
      compile_fragment_shader_node(b, out);
      out.push(')');
    }
    Node::UnaryFn(inner) => {
      match inner {
        UnaryFnNode::Sqrt(inner) => {
          out.push_str("sqrt(");
          compile_fragment_shader_node(inner, out);
          out.push(')');
        }
        UnaryFnNode::Sin(inner) => {
          out.push_str("sin(");
          compile_fragment_shader_node(inner, out);
          out.push(')');
        }
        UnaryFnNode::Cos(inner) => {
          out.push_str("cos(");
          compile_fragment_shader_node(inner, out);
          out.push(')');
        }
      }
    }
    Node::BinaryFn(inner) => {
      match inner {
        BinaryFnNode::Add(lhs, rhs) => {
          compile_fragment_shader_node_binary_fn(lhs, rhs, "+", out);
        }
        BinaryFnNode::Sub(lhs, rhs) => {
          compile_fragment_shader_node_binary_fn(lhs, rhs, "-", out);
        }
        BinaryFnNode::Mul(lhs, rhs) => {
          compile_fragment_shader_node_binary_fn(lhs, rhs, "*", out);
        }
        BinaryFnNode::Div(lhs, rhs) => {
          compile_fragment_shader_node_binary_fn(lhs, rhs, "/", out);
        }
        BinaryFnNode::Mod(lhs, rhs) => {
          out.push_str("mod(");
          compile_fragment_shader_node(lhs, out);
          out.push(',');
          compile_fragment_shader_node(rhs, out);
          out.push(')');
        }
      }
    }
    Node::NullaryFn(_) => panic!("nullary fns are not part of the shader compilation process"),
  }
}

fn compile_fragment_shader_node_binary_fn(lhs: &BoxNode, rhs: &BoxNode, op: &str, out: &mut String) {
  // out.push('(');
  // out.push(')');

  compile_fragment_shader_node(lhs, out);
  out.push(' ');
  out.push_str(op);
  out.push(' ');
  compile_fragment_shader_node(rhs, out);

  // out.push('(');
  // out.push(')');
}
