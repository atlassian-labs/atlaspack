use std::io::ErrorKind;

use mozjpeg::*;
use napi::bindgen_prelude::*;
use napi::Env;
use napi::Error;
use napi::JsBuffer;
use napi::Result;
use napi_derive::napi;
use oxipng::optimize_from_memory;
use oxipng::Options;
use oxipng::StripChunks;

#[napi]
pub fn optimize_image(kind: String, buf: Buffer, env: Env) -> Result<JsBuffer> {
  let slice = buf.as_ref();

  match kind.as_ref() {
    "png" => {
      let options = Options {
        strip: StripChunks::Safe,
        ..Default::default()
      };
      match optimize_from_memory(slice, &options) {
        Ok(res) => Ok(env.create_buffer_with_data(res)?.into_raw()),
        Err(err) => Err(Error::from_reason(format!("{}", err))),
      }
    }
    "jpg" | "jpeg" => match optimize_jpeg(slice) {
      Ok(res) => Ok(env.create_buffer_with_data(res)?.into_raw()),
      Err(err) => Err(Error::from_reason(err.to_string())),
    },
    _ => Err(Error::from_reason(format!("Unknown image type {}", kind))),
  }
}

fn optimize_jpeg(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
  let jpeg = std::panic::catch_unwind(|| -> std::io::Result<Vec<u8>> {
    let src = Decompress::new_mem(bytes)?;
    let mut dst = Compress::new(src.color_space());

    let width = src.width();
    let height = src.height();

    dst.set_size(width, height);

    let mut compress = dst.start_compress(Vec::new())?;
    let pixels = vec![0u8; width * height * 3];

    compress.write_scanlines(&pixels[..])?;

    let writer = compress.finish()?;

    Ok(writer)
  });

  jpeg.map_err(|err| {
    let str_error = err.downcast::<String>();

    match str_error {
      Ok(err) => std::io::Error::new(ErrorKind::Other, *err),
      Err(_) => std::io::Error::new(ErrorKind::Other, "Unknown jpeg optimisation error"),
    }
  })?
}
