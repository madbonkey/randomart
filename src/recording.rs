use std::io::Write;
use std::process::{Child, Stdio};

// pub struct Ffmpeg {
//   proc: std::process::Child,
// }

#[derive(Default)]
#[allow(dead_code)]
pub enum Ffmpeg {
  #[default]
  Idle,
  Recording(Child),
  Error(std::io::Error),
}

impl Ffmpeg {
  pub fn start_recording(&mut self, seed: u64, depth: isize, hn: usize) -> Result<(), std::io::Error> {
    if !matches!(self, Self::Idle) {
      return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
    }

    let output_path = format!("output/rec_{seed}_{depth}_{hn}.mp4");

    let cmd = std::process::Command::new("ffmpeg")
      // see https://github.com/tsoding/randomart/blob/main/src/ffmpeg_linux.c#L47
      .args([
        // global options
        "-loglevel", "verbose",
        "-y",

        // input #1: piped raw video frames
        "-f", "rawvideo",
        "-pix_fmt", "rgba",
        "-s", "1440x1440",
        "-r", "60",
        "-i", "-",

        // output
        "-vf", "vflip",
        "-c:v", "libx264",
        "-vb", "4500k",
        "-pix_fmt", "yuv420p",
      ])
      .arg(output_path)
      .stdin(Stdio::piped())
      .stderr(Stdio::inherit())
      .stdout(Stdio::inherit())
      .spawn();

    let proc = cmd?;

    if proc.stdin.is_some() {
      let pid = proc.id();
      tracing::error!(pid = pid, "spawned recording child process");
      let _ = std::mem::replace(self, Self::Recording(proc));
      Ok(())
    } else {
      let _ = std::mem::replace(self, Self::Error(std::io::Error::from(std::io::ErrorKind::BrokenPipe)));
      Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
    }
  }

  pub fn record_frame(&mut self, image: macroquad::texture::Image) {
    match self {
      Self::Recording(proc) => {
        // SAFETY: existence of proc.stdin verified on self variant creation
        let pipe = unsafe { proc.stdin.as_mut().unwrap_unchecked() };

        if let Err(e) = pipe.write_all(image.bytes.as_slice()) {
          tracing::error!("error writing frame: {e}");
        }
      },
      _ => {
        tracing::error!("not in recording state");
      }
    }
  }

  pub fn finish_recording(&mut self) {
    if !self.is_recording() { return; }

    match std::mem::replace(self, Self::Idle) {
      Self::Recording(proc) => {
        match proc.wait_with_output() {
          Ok(output) => {
            tracing::info!("recording finished with exit status {:?}", output.status);
          }
          Err(e) => {
            tracing::error!("error finishing recording: {e}");
          }
        }

      },
      _ => unreachable!(),
    }
  }

  pub fn cancel_recording(&mut self) {
    if !self.is_recording() { return; }

    match std::mem::replace(self, Self::Idle) {
      Self::Recording(mut proc) => {
        let pid = proc.id();

        if let Err(e) = interrupt_child(pid) {
          tracing::error!("failed sending SIGINT to recording child process: {e}");
        }

        if let Err(e) = proc.kill() {
          tracing::error!("error killing recording child process: {e}");
          return;
        }

        tracing::info!(pid = pid, "killed recording child process");
      },
      _ => unreachable!(),
    }

  }

  #[inline]
  pub fn is_recording(&self) -> bool {
    matches!(self, Self::Recording(_))
  }
}

fn interrupt_child(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
  let mut kill = std::process::Command::new("kill")
    .args(["-s", "INT"])
    .arg(pid.to_string())
    .spawn()?;

  kill.wait()?;

  Ok(())
}
