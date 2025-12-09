//! Constants mirrored from `packages/babel-plugin/src/constants.ts`.

/// Identifier used to store DOM props on compiled components.
pub const DOM_PROPS_IDENTIFIER_NAME: &str = "__cmpldp";

/// Identifier used to reference the props object in compiled component factories.
pub const PROPS_IDENTIFIER_NAME: &str = "__cmplp";

/// Identifier used to track the React ref passed to compiled components.
pub const REF_IDENTIFIER_NAME: &str = "__cmplr";

/// Identifier used to collect style declarations for compiled components.
pub const STYLE_IDENTIFIER_NAME: &str = "__cmpls";

/// Default module imported when enabling compiled transforms.
pub const COMPILED_IMPORT: &str = "@compiled/react";

/// Default module origins that trigger compiled transforms.
pub const DEFAULT_IMPORT_SOURCES: &[&str] = &[COMPILED_IMPORT, "@atlaskit/css"];

/// Default Babel parser plugins applied when loading dependent modules.
pub const DEFAULT_PARSER_BABEL_PLUGINS: &[&str] = &["typescript", "jsx"];

/// Comment directive that disables the current line of compiled transforms.
pub const COMPILED_DIRECTIVE_DISABLE_LINE: &str = "@compiled-disable-line";

/// Comment directive that disables the next line of compiled transforms.
pub const COMPILED_DIRECTIVE_DISABLE_NEXT_LINE: &str = "@compiled-disable-next-line";

/// Comment directive key used to control the CSS prop transform.
pub const COMPILED_DIRECTIVE_TRANSFORM_CSS_PROP: &str = "transform-css-prop";

/// Default file extensions treated as code by the resolver.
pub const DEFAULT_CODE_EXTENSIONS: &[&str] = &[".js", ".jsx", ".ts", ".tsx"];
