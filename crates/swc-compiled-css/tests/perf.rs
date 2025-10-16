use std::time::Instant;

use swc_common::{FileName, SourceMap, sync::Lrc};
use swc_ecma_ast::Program;
use swc_ecma_codegen::{Emitter, text_writer::JsWriter};
use swc_ecma_parser::{Parser, StringInput, Syntax};

fn transform_source(src: String) -> String {
  let cm: Lrc<SourceMap> = Default::default();
  let fm = cm.new_source_file(FileName::Custom("input.js".into()).into(), src);
  let mut parser = Parser::new(
    Syntax::Es(Default::default()),
    StringInput::from(&*fm),
    None,
  );
  let module = parser.parse_module().expect("parse module");
  let program = Program::Module(module);

  let out_program = atlassian_swc_compiled_css::process_transform(program);

  let mut buf = Vec::new();
  {
    let mut emitter = Emitter {
      cfg: Default::default(),
      cm: cm.clone(),
      comments: None,
      wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
    };
    emitter.emit_program(&out_program).expect("emit program");
  }
  String::from_utf8(buf).expect("utf8")
}

fn bench_ops_per_sec(source: &str, iterations: usize) -> f64 {
  // Warmup
  let _ = transform_source(source.to_string());
  let start = Instant::now();
  for _ in 0..iterations {
    let _ = transform_source(source.to_string());
  }
  let elapsed = start.elapsed();
  // ops/sec = iterations / seconds
  iterations as f64 / elapsed.as_secs_f64()
}

#[test]
#[ignore]
fn perf_object_vs_tagged_template_within_15pct() {
  // Representative object syntax
  let object_src = r#"
        import { css } from '@compiled/react';
        const styles = css({
          border: 'none',
          padding: '8px 10px',
          backgroundColor: '#6554C0',
          color: '#fff',
          fontWeight: 400,
          '&:hover': { backgroundColor: '#8777D9' },
          '@media (min-width: 64rem)': {
            gridArea: 'main',
          },
        });
    "#;

  // Representative tagged template syntax
  let tagged_src = r#"
        import { css } from '@compiled/react';
        const styles = css`
          border: none;
          padding: 8px 10px;
          background-color: #6554C0;
          color: #fff;
          font-weight: 400;
          &:hover { background-color: #8777D9; }
          @media (min-width: 64rem) {
            grid-area: main;
          }
        `;
    "#;

  // Representative styled.div object syntax (similar complexity)
  let styled_object_src = r#"
        import { styled } from '@compiled/react';
        const Button = styled.div({
          border: 'none',
          padding: '8px 10px',
          backgroundColor: '#6554C0',
          color: '#fff',
          fontWeight: 400,
          '&:hover': { backgroundColor: '#8777D9' },
          '@media (min-width: 64rem)': {
            gridArea: 'main',
          },
        });
    "#;

  // Keep iterations moderate for CI stability
  let iterations = 50000;
  let ops_object = bench_ops_per_sec(object_src, iterations);
  let ops_tagged = bench_ops_per_sec(tagged_src, iterations);
  let ops_styled = bench_ops_per_sec(styled_object_src, iterations);

  // Allow within 15% difference either way
  let faster = ops_object.max(ops_tagged);
  let slower = ops_object.min(ops_tagged);
  let within = slower / faster;
  eprintln!(
    "ops/s object={:.0}, tagged={:.0}, styled={:.0}, ratio(object/tagged)={:.3}",
    ops_object, ops_tagged, ops_styled, within
  );
  assert!(
    within >= 0.85,
    "Performance drift exceeds 15% (object={:.0} ops/s, tagged={:.0} ops/s)",
    ops_object,
    ops_tagged
  );
}
