use flate2::read::GzDecoder;
use tar::Archive;

pub fn tar_gz(bytes: &[u8]) -> Archive<GzDecoder<&[u8]>> {
  let tar = GzDecoder::new(bytes);
  Archive::new(tar)
}
