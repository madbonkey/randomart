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
  -d, --depth <DEPTH>  Depth guide value of the function generation [default: 55]
      --seed <SEED>    Seed for the RNG [default: 1337]
  -h, --help           Print help
```

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
