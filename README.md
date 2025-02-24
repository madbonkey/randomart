# Random Art 

A reimplementation and extension of [tsoding's randomart project](https://github.com/tsoding/randomart), in Rust.

Original paper: [Hash Visualization: a New Technique
to improve Real World Security](http://users.ece.cmu.edu/~adrian/projects/validation/validation.pdf)

## Building

You can build this project with [Cargo](https://doc.rust-lang.org/cargo/):

```
cargo build --release
```

## Usage

```
$ randomart <COMMAND>

Commands:
  generate  Generate random art images and save to disk
  gui       Launch random art GUI to play around with
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Commands

#### `generate`

Generate random art images and save to disk

```
$ randomart generate [OPTIONS]

Options:
  -g, --grammar <GRAMMAR>        The .grammar file to use for generation
  -d, --depth <DEPTH>            Depth guide value of the function generation [default: 55]
  -s, --size <SIZE>              Size (width and height) of the generated image(s) [default: 1024]
  -n, --num-images <NUM_IMAGES>  Number of images to generate [default: 10]
      --seed <SEED>              Seed for the RNG [default: 1337]
  -h, --help                     Print help
```

#### `gui` 

Launch random art GUI to play around with

```
$ randomart gui [OPTIONS]

Options:
  -g, --grammar <GRAMMAR>  The .grammar file to use for generation
  -d, --depth <DEPTH>      Depth guide value of the function generation [default: 55]
      --seed <SEED>        Seed for the RNG
  -h, --help               Print help
```

### `.grammar` File Format

* Grammar files are lists of "rules"
* Rules start with an alphabetic rule ID, followed by `:=`, and one or more "branches", then terminate with `;`
* Branches start with a `|`, followed by
  * a color definition: `color(R, G, B)`, where `R/G/B` can be another expression
  * a rule reference: `<ID>`, where "ID" is a rule ID in the grammar
  * a variable:
    * `@x`: the X coordinate of the rendered pixel (in 0..1) 
    * `@y`: the Y coordinate of the rendered pixel (in 0..1)
    * `@t`: the time value of the rendering/recording (oscillates in 0..1)
  * a constant float: `const(1.0)`
  * a function call: see below for available functions 
* **IMPORTANT**: Grammars need to have exactly one rule which contains only `color(..)` branches. This rule will be used as the entrypoint for generating an AST. If there are more than one rule for which this applies, one is chosen randomly, but there can be errors if other rules with `color(..)` are reachable – this is an open to-do!
* For examples, see the [grammar/](./grammar) directory
 
#### Available functions
   
| name    | args | example                     |                                     |
|---------|:----:|-----------------------------|-------------------------------------|
| sin/cos |  1   | `sin(rand())`               | sine / cosine                       |
| sqrt    |  1   | `sqrt(@x)`                  | square root                         |
| add     |  2   | `add(@x, <C>)`              | addition                            |
| mul     |  2   | `mul(@y, const(0.5)`        | multiplication                      |
| mod     |  2   | `mod(@t, 0.1)`              | modulo                              |
| mix     |  3   | `mix(sqrt(@x), @x, rand())` | the usual mix function<sup>1)</sup> |

<small><sup>1)</sup> see [thebookofshaders.com](https://thebookofshaders.com/glossary/?search=mix)</small>

### GUI mode keybindings

| Key           | Description                            |
|---------------|----------------------------------------|
| `N`           | Next random art                        |
| `Left Arrow`  | History: back                          |
| `Right Arrow` | History: forward                       |
| `Space`       | Pause/unpause time                     |
| `+`           | Increase depth by 1 (10 with `Shift`)  |
| `-`           | Decrease depth by 1 (10 with `Shift`)  |
| `H`           | Toggle info HUD                        |
| `S`           | Re-seed and reset                      |
| `R`           | Start recording a ~6 second video clip |
| `D`           | Dump debug information to stdout       |
| `ESC`         | (while recording) Cancel recording     |
