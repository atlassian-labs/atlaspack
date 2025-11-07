use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Error;
use async_trait::async_trait;
use html5ever::serialize::SerializeOpts;
use html5ever::tendril::TendrilSink;
use html5ever::{ParseOpts, serialize};
use markup5ever_rcdom::{RcDom, SerializableHandle};
use regex::Regex;

use atlaspack_core::plugin::{PluginContext, TransformContext, TransformResult, TransformerPlugin};
use atlaspack_core::types::{
  Asset, AssetId, AssetWithDependencies, BundleBehavior, Code, Dependency, Environment,
};

use crate::dom_visitor::walk;
use crate::svg_dependencies_visitor::SvgDependenciesVisitor;

#[derive(Debug)]
pub struct AtlaspackSvgTransformerPlugin {
  project_root: PathBuf,
}

impl AtlaspackSvgTransformerPlugin {
  pub fn new(ctx: &PluginContext) -> Self {
    AtlaspackSvgTransformerPlugin {
      project_root: ctx.options.project_root.clone(),
    }
  }
}

#[async_trait]
impl TransformerPlugin for AtlaspackSvgTransformerPlugin {
  async fn transform(
    &self,
    context: TransformContext,
    input: Asset,
  ) -> Result<TransformResult, Error> {
    let bytes: &[u8] = input.code.bytes();

    // Pre-process XML processing instructions before parsing with html5ever
    let (processed_content, xml_dependencies, processing_instructions) =
      process_xml_processing_instructions(bytes)?;

    let mut dom = parse_svg(&processed_content)?;
    let context = SVGTransformationContext {
      env: context.env().clone(),
      project_root: self.project_root.clone(),
      side_effects: input.side_effects,
      source_asset_id: input.id.clone(),
      source_path: Some(input.file_path.clone()),
    };

    let SvgTransformation {
      mut dependencies,
      discovered_assets,
      ..
    } = run_svg_transformations(context.clone(), &mut dom)?;

    // Add XML processing instruction dependencies
    for xml_dep in xml_dependencies {
      let dependency = atlaspack_core::types::DependencyBuilder::default()
        .env(context.env.clone())
        .priority(atlaspack_core::types::Priority::Parallel)
        .source_asset_id(context.source_asset_id.clone())
        .source_asset_type(atlaspack_core::types::FileType::Other("svg".to_string()))
        .source_path_option(context.source_path.clone())
        .specifier(xml_dep)
        .specifier_type(atlaspack_core::types::SpecifierType::Url)
        .build();
      dependencies.push(dependency);
    }

    let mut asset = input;
    asset.bundle_behavior = Some(BundleBehavior::Isolated);

    // Serialize SVG and prepend processing instructions
    let serialized_svg = serialize_svg(dom)?;
    let final_output = if processing_instructions.is_empty() {
      serialized_svg
    } else {
      let mut output = processing_instructions.into_bytes();
      output.extend(serialized_svg);
      output
    };

    asset.code = Code::new(final_output);

    Ok(TransformResult {
      asset,
      dependencies,
      discovered_assets,
      ..Default::default()
    })
  }
}

fn process_xml_processing_instructions(
  bytes: &[u8],
) -> Result<(Vec<u8>, Vec<String>, String), Error> {
  let content = std::str::from_utf8(bytes)?;
  let mut xml_dependencies = Vec::new();

  // Handle <?xml-stylesheet?> processing instructions
  // Process each processing instruction individually to handle malformed cases
  let xml_pi_regex = Regex::new(r#"(?s)<\?xml-stylesheet\s[^?]*?\?>"#)?;
  let href_regex = Regex::new(r#"href\s*=\s*["']([^"']+)["']"#)?;

  // Extract dependencies from XML processing instructions
  for pi_match in xml_pi_regex.find_iter(content) {
    let pi_content = pi_match.as_str();

    // Only process xml-stylesheet instructions (not xml-not-a-stylesheet, etc.)
    if pi_content.starts_with("<?xml-stylesheet") && pi_content.ends_with("?>") {
      // Look for href within this specific processing instruction
      if let Some(caps) = href_regex.captures(pi_content) {
        if let Some(href_match) = caps.get(1) {
          let href = href_match.as_str();
          if !href.is_empty() {
            xml_dependencies.push(href.to_string());
          }
        }
      }
    }
  }

  // Extract all XML processing instructions to preserve them
  let all_pi_regex = Regex::new(r#"(?s)<\?[^?]*?\?>"#)?;
  let mut processing_instructions = String::new();
  for pi_match in all_pi_regex.find_iter(content) {
    processing_instructions.push_str(pi_match.as_str());
    processing_instructions.push('\n');
  }

  // Remove XML processing instructions from content for html5ever parsing
  let svg_content_only = all_pi_regex.replace_all(content, "");

  Ok((
    svg_content_only.as_bytes().to_vec(),
    xml_dependencies,
    processing_instructions,
  ))
}

fn serialize_svg(dom: RcDom) -> Result<Vec<u8>, Error> {
  let document: SerializableHandle = dom.document.clone().into();
  let mut output_bytes = vec![];
  let options = SerializeOpts::default();
  serialize(&mut output_bytes, &document, options)?;
  Ok(output_bytes)
}

fn parse_svg(bytes: &[u8]) -> Result<RcDom, Error> {
  let mut bytes = BufReader::new(bytes);
  let options = ParseOpts::default();
  let dom = RcDom::default();
  // Parse as HTML since html5ever doesn't have a specific SVG parser
  // but SVG elements are part of the HTML5 spec
  let dom = html5ever::parse_document(dom, options)
    .from_utf8()
    .read_from(&mut bytes)?;
  Ok(dom)
}

#[derive(Clone, Default)]
pub struct SVGTransformationContext {
  pub env: Arc<Environment>,
  pub project_root: PathBuf,
  pub side_effects: bool,
  pub source_asset_id: AssetId,
  pub source_path: Option<PathBuf>,
}

#[derive(Debug, PartialEq)]
struct SvgTransformation {
  dependencies: Vec<Dependency>,
  discovered_assets: Vec<AssetWithDependencies>,
}

/// Entry-point for all SVG transformations
fn run_svg_transformations(
  context: SVGTransformationContext,
  dom: &mut RcDom,
) -> Result<SvgTransformation, Error> {
  let node = dom.document.clone();
  let context = Rc::new(context);

  let mut dependencies_visitor = SvgDependenciesVisitor::new(context.clone());
  walk(node.clone(), &mut dependencies_visitor);

  // Check for errors and return them
  if !dependencies_visitor.errors.is_empty() {
    return Err(anyhow::anyhow!(
      "SVG processing errors: {}",
      dependencies_visitor.errors.join(", ")
    ));
  }

  Ok(SvgTransformation {
    dependencies: dependencies_visitor.dependencies,
    discovered_assets: dependencies_visitor.discovered_assets,
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use atlaspack_core::types::Environment;
  use std::path::PathBuf;

  fn create_test_context() -> SVGTransformationContext {
    SVGTransformationContext {
      env: Arc::new(Environment::default()),
      project_root: PathBuf::from("/test"),
      side_effects: false,
      source_asset_id: "test-asset".to_string(),
      source_path: Some(PathBuf::from("/test/input.svg")),
    }
  }

  #[test]
  fn test_xml_processing_instructions() {
    let svg_with_xml = r#"<?xml-stylesheet href="style.css" type="text/css"?>
<?xml-stylesheet href="theme.css" type="text/css"?>
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40"/>
</svg>"#;

    let (processed_content, xml_deps, processing_instructions) =
      process_xml_processing_instructions(svg_with_xml.as_bytes()).unwrap();

    // Should extract both CSS dependencies
    assert_eq!(xml_deps.len(), 2);
    assert!(xml_deps.contains(&"style.css".to_string()));
    assert!(xml_deps.contains(&"theme.css".to_string()));

    // Processing instructions should be extracted separately
    assert!(processing_instructions.contains("<?xml-stylesheet"));

    // Content should have processing instructions removed for parsing
    assert!(
      !String::from_utf8(processed_content)
        .unwrap()
        .contains("<?xml-stylesheet")
    );
  }

  #[test]
  fn test_empty_href_error() {
    let svg_with_empty_href = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <use href=""/>
</svg>"#;

    let mut dom = parse_svg(svg_with_empty_href.as_bytes()).unwrap();
    let context = create_test_context();

    let result = run_svg_transformations(context, &mut dom);
    assert!(result.is_err());
    assert!(
      result
        .unwrap_err()
        .to_string()
        .contains("should not be empty")
    );
  }

  #[test]
  fn test_functional_iri_dependencies() {
    let svg_with_func_iri = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <rect fill="url('gradient.svg#grad')" stroke="url(pattern.svg#pat)" />
  <circle clip-path="url(clip.svg#clipPath)" />
</svg>"#;

    let mut dom = parse_svg(svg_with_func_iri.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should create dependencies for all functional IRI references
    assert_eq!(transformation.dependencies.len(), 3);
    let specifiers: Vec<String> = transformation
      .dependencies
      .iter()
      .map(|d| d.specifier.clone())
      .collect();
    assert!(specifiers.contains(&"gradient.svg#grad".to_string()));
    assert!(specifiers.contains(&"pattern.svg#pat".to_string()));
    assert!(specifiers.contains(&"clip.svg#clipPath".to_string()));
  }

  #[test]
  fn test_external_scripts() {
    let svg_with_external_scripts = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <script href="script.js" type="text/javascript"/>
  <script src="module.js" type="module"/>
</svg>"#;

    let mut dom = parse_svg(svg_with_external_scripts.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should create dependencies for both scripts
    assert_eq!(transformation.dependencies.len(), 2);
    let specifiers: Vec<String> = transformation
      .dependencies
      .iter()
      .map(|d| d.specifier.clone())
      .collect();
    assert!(specifiers.contains(&"script.js".to_string()));
    assert!(specifiers.contains(&"module.js".to_string()));
  }

  #[test]
  fn test_inline_scripts_and_styles() {
    let svg_with_inline = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <style type="text/css">.red { fill: red; }</style>
  <script type="text/javascript">console.log('test');</script>
  <style type="text/scss">$color: blue; .blue { fill: $color; }</style>
</svg>"#;

    let mut dom = parse_svg(svg_with_inline.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should create 3 inline assets (2 styles + 1 script) and 3 dependencies
    assert_eq!(transformation.discovered_assets.len(), 3);
    assert_eq!(transformation.dependencies.len(), 3);

    // Check file types
    let css_assets = transformation
      .discovered_assets
      .iter()
      .filter(|a| a.asset.file_type == atlaspack_core::types::FileType::Css)
      .count();
    let scss_assets = transformation
      .discovered_assets
      .iter()
      .filter(|a| a.asset.file_type == atlaspack_core::types::FileType::Other("scss".to_string()))
      .count();
    let js_assets = transformation
      .discovered_assets
      .iter()
      .filter(|a| a.asset.file_type == atlaspack_core::types::FileType::Js)
      .count();

    assert_eq!(css_assets, 1);
    assert_eq!(scss_assets, 1);
    assert_eq!(js_assets, 1);
  }

  #[test]
  fn test_custom_data_parcel_key() {
    let svg_with_custom_key = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <style data-parcel-key="custom-style-key">.custom { fill: red; }</style>
  <script data-parcel-key="custom-script-key">console.log('custom');</script>
</svg>"#;

    let mut dom = parse_svg(svg_with_custom_key.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should have 2 inline assets with custom keys
    assert_eq!(transformation.discovered_assets.len(), 2);

    let style_asset = transformation
      .discovered_assets
      .iter()
      .find(|a| a.asset.file_type == atlaspack_core::types::FileType::Css)
      .unwrap();
    let script_asset = transformation
      .discovered_assets
      .iter()
      .find(|a| a.asset.file_type == atlaspack_core::types::FileType::Js)
      .unwrap();

    assert_eq!(
      style_asset.asset.unique_key,
      Some("custom-style-key".to_string())
    );
    assert_eq!(
      script_asset.asset.unique_key,
      Some("custom-script-key".to_string())
    );
  }

  #[test]
  fn test_xlink_href_precedence() {
    let svg_with_mixed_href = r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">
  <use href="modern.svg#symbol" xlink:href="legacy.svg#symbol"/>
  <use xlink:href="legacy-only.svg#symbol"/>
</svg>"#;

    let mut dom = parse_svg(svg_with_mixed_href.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should have 2 dependencies: one for href (takes precedence), one for xlink:href only
    assert_eq!(transformation.dependencies.len(), 2);
    let specifiers: Vec<String> = transformation
      .dependencies
      .iter()
      .map(|d| d.specifier.clone())
      .collect();
    assert!(specifiers.contains(&"modern.svg#symbol".to_string())); // href takes precedence
    assert!(specifiers.contains(&"legacy-only.svg#symbol".to_string())); // xlink:href only
    // Should NOT contain legacy.svg#symbol since href takes precedence
    assert!(!specifiers.contains(&"legacy.svg#symbol".to_string()));
  }

  #[test]
  fn test_script_type_detection() {
    let svg_with_script_types = r#"<svg xmlns="http://www.w3.org/2000/svg">
  <script type="application/javascript">console.log('app-js');</script>
  <script type="module">console.log('module');</script>
  <script type="text/typescript">console.log('ts');</script>
  <script>console.log('default');</script>
</svg>"#;

    let mut dom = parse_svg(svg_with_script_types.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should have 4 script assets
    assert_eq!(transformation.discovered_assets.len(), 4);

    // Check that module script has correct source type
    let module_script = transformation
      .discovered_assets
      .iter()
      .find(|a| {
        std::str::from_utf8(a.asset.code.bytes())
          .unwrap()
          .contains("module")
      })
      .unwrap();
    assert_eq!(
      module_script.asset.env.source_type,
      atlaspack_core::types::SourceType::Module
    );

    // Check that other scripts have Script source type
    let app_js_script = transformation
      .discovered_assets
      .iter()
      .find(|a| {
        std::str::from_utf8(a.asset.code.bytes())
          .unwrap()
          .contains("app-js")
      })
      .unwrap();
    assert_eq!(
      app_js_script.asset.env.source_type,
      atlaspack_core::types::SourceType::Script
    );
  }

  #[test]
  fn test_basic_svg_parsing() {
    let svg_content = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <circle cx="50" cy="50" r="40" fill="red"/>
</svg>"#;

    let dom = parse_svg(svg_content.as_bytes()).unwrap();
    assert!(!dom.document.children.borrow().is_empty());
  }

  #[test]
  fn test_svg_with_inline_style() {
    let svg_content = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <style>.red { fill: red; }</style>
  <circle cx="50" cy="50" r="40" class="red"/>
</svg>"#;

    let mut dom = parse_svg(svg_content.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should have discovered one inline CSS asset
    assert_eq!(transformation.discovered_assets.len(), 1);
    assert_eq!(transformation.dependencies.len(), 1);
  }

  #[test]
  fn test_svg_with_href_dependencies() {
    let svg_content = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <use href="symbols.svg#icon"/>
  <image href="image.png" x="0" y="0"/>
</svg>"#;

    let mut dom = parse_svg(svg_content.as_bytes()).unwrap();
    let context = create_test_context();

    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // Should have dependencies for both href attributes
    assert_eq!(transformation.dependencies.len(), 2);
  }

  #[test]
  fn test_xml_complex_integration_fixture() {
    // This test matches the exact fixture from the complex XML test
    let svg_content = r#"<?xml-stylesheet href="style1.css" type="text/css"?>
<?xml-stylesheet
  href="style2.css"
  type="text/css"
  media="screen"?>
<?xml-stylesheet href='style3.css' type='text/css'?>
<?xml-not-stylesheet href="should-not-process.css"?>
<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
  <text>Styled text</text>
</svg>"#;

    let (processed_content, xml_deps, processing_instructions) =
      process_xml_processing_instructions(svg_content.as_bytes()).unwrap();

    println!("Complex XML test - found {} dependencies:", xml_deps.len());
    for dep in &xml_deps {
      println!("  - {}", dep);
    }

    // JS version expects only 2 bundles, so only 2 stylesheets should be processed
    // Let's see what we're actually finding
    assert_eq!(
      xml_deps.len(),
      2,
      "Expected 2 XML stylesheet dependencies to match JS behavior"
    );
  }

  #[test]
  fn test_xml_stylesheet_integration_fixture() {
    // This test matches the exact fixture from svg-xml-stylesheet/img.svg
    let svg_content = r#"<?xml-stylesheet href="style1.css"?>
<?xml-stylesheet href="style2.css?>
  <?xml-stylesheet
    href
    =
    "style3.css"type="text/css"
      ?>
<?xml-not-a-stylesheet href="style4.css"?>
<svg viewBox="0 0 240 80" xmlns="http://www.w3.org/2000/svg">
  <text>Should be red and monospace</text>
</svg>"#;

    // Test XML processing instruction extraction
    let (processed_content, xml_deps, processing_instructions) =
      process_xml_processing_instructions(svg_content.as_bytes()).unwrap();

    // Should extract exactly 2 dependencies: style1.css and style3.css
    assert_eq!(
      xml_deps.len(),
      2,
      "Should find exactly 2 XML stylesheet dependencies"
    );
    assert!(
      xml_deps.contains(&"style1.css".to_string()),
      "Should find style1.css"
    );
    assert!(
      xml_deps.contains(&"style3.css".to_string()),
      "Should find style3.css"
    );

    // Should NOT extract malformed or wrong instruction types
    assert!(
      !xml_deps.contains(&"style2.css".to_string()),
      "Should NOT extract style2.css (malformed)"
    );
    assert!(
      !xml_deps.contains(&"style4.css".to_string()),
      "Should NOT extract style4.css from xml-not-a-stylesheet"
    );

    // Parse the SVG and run transformations
    let mut dom = parse_svg(&processed_content).unwrap();
    let context = create_test_context();
    let transformation = run_svg_transformations(context, &mut dom).unwrap();

    // No DOM dependencies expected in this SVG (just text content)
    assert_eq!(
      transformation.dependencies.len(),
      0,
      "No DOM dependencies expected"
    );
    assert_eq!(
      transformation.discovered_assets.len(),
      0,
      "No inline assets expected"
    );

    // Processing instructions should be extracted separately
    assert!(
      processing_instructions.contains("<?xml-stylesheet"),
      "XML processing instructions should be extracted"
    );
    assert!(
      processing_instructions.contains("<?xml-not-a-stylesheet"),
      "Non-stylesheet processing instructions should be extracted"
    );

    // Content should have processing instructions removed for parsing
    let content_str = String::from_utf8(processed_content).unwrap();
    assert!(
      !content_str.contains("<?xml-stylesheet"),
      "XML processing instructions should be removed from content for parsing"
    );
  }
}
