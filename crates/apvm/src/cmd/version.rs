use crate::context::Context;
use crate::platform::colors::*;

// TODO: get version from package.json and fallback to cargo version
// const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Print the apvm logo
#[rustfmt::skip]
pub fn main(_ctx: Context) -> anyhow::Result<()> {
  println!("{color_red}");
  println!(r#"      __         _______  ___      ___  ___      ___ "#);
  println!(r#"     /""\       |   __ "\|"  \    /"  ||"  \    /"  |"#);
  println!(r#"    /    \      (. |__) :)\   \  //  /  \   \  //   |"#);
  println!(r#"   /' /\  \     |:  ____/  \\  \/. ./   /\\  \/.    |"#);
  println!(r#"  //  __'  \    (|  /       \.    //   |: \.        |"#);
  println!(r#" /   /  \\  \  /|__/ \       \\   /    |.  \    /:  |"#);
  println!(r#"(___/    \___)(_______)       \__/     |___|\__/|___|"#);
  println!(r#"                                                     "#);
  print!("{color_reset}");
  print!("{style_bold}");
  println!(r#"             Atlaspack Version Manager               "#);
  println!("{style_reset}");
  Ok(())
}
