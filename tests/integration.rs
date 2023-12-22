use std::convert::Infallible;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use embedded_io;
use embedded_io::SeekFrom;
use mp3stitch::merge_mp3s;

const OUT_MP3_1: &str = "output1.mp3";
const OUT_MP3_2: &str = "output2.mp3";
const OUT_COMBINED: &str = "output_combined.mp3";

#[test]
fn test_mp3_synth_1() {
    gen_pitch(440, OUT_MP3_1)
        .expect("Failed to execute command");
}

#[test]
fn test_mp3_synth_2() {
    gen_pitch(880, OUT_MP3_2)
        .expect("Failed to execute command");
}

fn gen_pitch(freq: u32, out_path: &str) -> io::Result<()> {
    let synth = format!("aevalsrc=sin(2*PI*{}*t):d=1", freq);
    std::process::Command::new("ffmpeg")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg(synth.as_str())
        .arg("-c:a")
        .arg("libmp3lame")
        .arg(out_path)
        .output()?;
    Ok(())
}

#[test]
fn test_mp3_merge() {
    let mut input1 = Vec::new();
    let mut input2 = Vec::new();
    {
        File::open(OUT_MP3_1).unwrap()
            .read_to_end(&mut input1).unwrap();
    }
    {
        File::open(OUT_MP3_2).unwrap()
            .read_to_end(&mut input2).unwrap();
    }

    let cursor1 = Cursor::new(input1.as_slice());
    let cursor2 = Cursor::new(input2.as_slice());

    let mut output = Vec::new();

    let mut inputs = vec![cursor1, cursor2];
    merge_mp3s(inputs.iter_mut(), &mut output, |_| {}).unwrap();

    File::create(OUT_COMBINED).unwrap()
        .write_all(output.as_slice()).unwrap()
}

// HOWTF is this working!?
// #[test]
// fn test_mp3_tagged_merge() {
//     let mut input1 = File::open(OUT_MP3_TAGGED_1)
//         .expect("Failed to open file 1");
//     let mut input2 = File::open(OUT_MP3_TAGGED_2)
//         .expect("Failed to open file 2");
//     let mut output = File::create(OUT_TAGGED_COMBINED)
//         .expect("Failed to create combined file");
//
//     let mut inputs = vec![&mut input1, &mut input2];
//     merge_mp3s(inputs.iter_mut(), &mut output)
//         .expect("Failed to merge mp3s");
// }

pub struct Cursor<'a> {
    inner: &'a[u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(inner: &'a[u8]) -> Self {
        Self {
            inner,
            pos: 0,
        }
    }
}

impl embedded_io::ErrorType for Cursor<'_> {
    type Error = Infallible;
}

impl embedded_io::Read for Cursor<'_> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let len = self.inner.len();
        if self.pos >= len {
            return Ok(0);
        }
        let mut read_len = buf.len();
        if self.pos + read_len > len {
            read_len = len - self.pos;
        }
        buf[..read_len].copy_from_slice(&self.inner[self.pos..self.pos + read_len]);
        self.pos += read_len;
        Ok(read_len)
    }
}

impl embedded_io::Seek for Cursor<'_> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64, Self::Error> {
        match pos {
            SeekFrom::Start(pos) => {
                self.pos = pos as usize;
            }
            SeekFrom::End(pos) => {
                self.pos = self.inner.len() - pos as usize;
            }
            SeekFrom::Current(pos) => {
                self.pos += pos as usize;
            }
        }
        Ok(self.pos as u64)
    }
}