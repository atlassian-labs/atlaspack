#[macro_export]
macro_rules! into_variant_impl {
  ($enum_name:ident, $method:ident, $variant:ident, $output:ty) => {
    impl $enum_name {
      pub fn $method(self) -> Option<$output> {
        match self {
          $enum_name::$variant(output) => Some(output),
          _ => return None,
        }
      }
    }
  };
}

#[macro_export]
macro_rules! as_variant_impl {
  ($enum_name:ident, $method:ident, $variant:ident, $output:ty) => {
    impl $enum_name {
      pub fn $method(&self) -> Option<&$output> {
        match self {
          $enum_name::$variant(output) => Some(output),
          _ => return None,
        }
      }
    }
  };
}
