use napi_derive::napi;

#[napi(object)]
pub struct Replacement {
  pub from: String,
  pub to: String,
}

#[napi]
pub fn perform_string_replacements(input: String, replacements: Vec<Replacement>) -> String {
  todo!("Write the function")
}
