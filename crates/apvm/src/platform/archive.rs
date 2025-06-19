use flate2::read::GzDecoder;
use tar::Archive;
use xz::read::XzDecoder;

pub fn tar_gz(bytes: &[u8]) -> Archive<GzDecoder<&[u8]>> {
  let tar = GzDecoder::new(bytes);
  Archive::new(tar)
}

pub fn tar_xz(bytes: &[u8]) -> Archive<XzDecoder<&[u8]>> {
  let tar = XzDecoder::new(bytes);
  Archive::new(tar)
}
