/// Generates a method `into_<...>` that consumes the enum and returns `Option<Inner>`
/// for a given enum variant `Variant(<Inner>)`.
///
/// Example:
///
/// ```
/// enum MyEnum {
///   Variant(String),
///   Other(usize),
/// }
///
/// into_variant_impl!(MyEnum, into_variant, Variant, String);
///
/// let my_enum = MyEnum::Variant("Hello, world!".to_string());
/// let string = my_enum.into_variant();
/// assert_eq!(string, Some("Hello, world!"));
/// ```
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

/// Generates a method `as_<...>` that returns `Option<&Inner>`
/// for a given enum variant `Variant(<Inner>)`.
///
/// Example:
///
/// ```
/// enum MyEnum {
///   Variant(String),
///   Other(usize),
/// }
///
/// as_variant_impl!(MyEnum, as_variant, Variant, String);
///
/// let my_enum = MyEnum::Variant("Hello, world!".to_string());
/// let string = my_enum.as_variant();
/// assert_eq!(string, Some("Hello, world!"));
/// ```
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
