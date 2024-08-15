use std::borrow::Cow;
use std::sync::Arc;

use atlaspack_dev_dep_resolver::build_esm_graph;
use atlaspack_resolver::Cache;
use atlaspack_resolver::CacheCow;
use atlaspack_resolver::OsFileSystem;
use atlaspack_resolver::Resolution;
use atlaspack_resolver::Resolver;
use atlaspack_resolver::SpecifierType;

fn main() {
  let contents = std::fs::read_to_string("package.json").unwrap();
  let pkg: serde_json::Value = serde_json::from_str(&contents).unwrap();
  let deps = pkg.get("dependencies").unwrap().as_object().unwrap();
  let cwd = std::env::current_dir().unwrap();

  let cache = Cache::new(Arc::new(OsFileSystem));
  let cjs_resolver = Resolver::node(Cow::Borrowed(&cwd), CacheCow::Borrowed(&cache));
  let esm_graph_cache = atlaspack_dev_dep_resolver::Cache::default();

  deps.keys().for_each(|dep| {
    #[cfg(debug_assertions)]
    println!("------------ {} -----------", dep);
    let resolved = match cjs_resolver.resolve(dep, &cwd, SpecifierType::Esm).result {
      Ok(res) => res.0,
      Err(e) => {
        #[cfg(debug_assertions)]
        println!("FAILED TO RESOLVE {} {:?}", dep, e);
        return;
      }
    };

    if let Resolution::Path(p) = resolved {
      match build_esm_graph(&p, &cwd, &cache, &esm_graph_cache) {
        Ok(_res) => {
          // #[cfg(debug_assertions)]
          // println!("{:?}", res)
        }
        Err(err) => {
          #[cfg(debug_assertions)]
          println!("FAIL: {:?}", err)
        }
      }
    }

    #[cfg(debug_assertions)]
    println!();
  });
}
