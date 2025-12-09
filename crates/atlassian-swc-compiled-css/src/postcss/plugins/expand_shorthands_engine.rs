#[cfg(feature = "postcss_engine")]
pub fn plugin() -> postcss::BuiltPlugin {
  use crate::postcss::plugins::expand_shorthands::index::expand_shorthand_pairs;
  use postcss::ast::nodes::as_declaration;

  postcss::plugin("expand-shorthands")
    .rule(|rule, _| {
      // Collect target declaration nodes first to avoid iterator invalidation
      let mut targets: Vec<postcss::ast::NodeRef> = Vec::new();
      for child in rule.nodes() {
        if let Some(decl) = as_declaration(&child) {
          let prop = decl.prop();
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[expand-shorthands:engine] rule visit prop='{}' value='{}'",
              prop,
              decl.value()
            );
          }
          // Only attempt expansion for known shorthands
          if expand_shorthand_pairs(&prop.to_lowercase(), &decl.value()).is_some() {
            // Skip if expansion yields empty set (remove)
            targets.push(child.clone());
          } else if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[expand-shorthands:engine] rule skip prop='{}' (no pairs)",
              prop
            );
          }
        }
      }

      for child in targets {
        if let Some(decl) = as_declaration(&child) {
          let prop = decl.prop();
          let value = decl.value();
          let important = decl.important();
          if let Some(pairs) = expand_shorthand_pairs(&prop.to_lowercase(), &value) {
            if std::env::var("COMPILED_CLI_TRACE").is_ok() {
              eprintln!(
                "[expand-shorthands:engine] rule pairs prop='{}' count={} -> {}",
                prop,
                pairs.len(),
                pairs
                  .iter()
                  .map(|(n, v)| format!("{}:{}", n, v))
                  .collect::<Vec<_>>()
                  .join(",")
              );
            }
            // Determine original index, then remove shorthand and insert longhands at same spot
            let idx_opt = rule.child_index(&child);
            rule.remove_child(child.clone());
            if let Some(idx) = idx_opt {
              let mut insert_at = idx;
              for (name, val) in pairs.into_iter() {
                let mut raws = postcss::ast::RawData::default();
                raws.set_text("between", ":");
                let new_decl =
                  postcss::ast::nodes::declaration_with_raws(name, val, important, raws);
                // Use low-level Node::insert on the underlying node
                postcss::ast::Node::insert(&rule.to_node(), insert_at, new_decl);
                insert_at += 1;
              }
            } else {
              for (name, val) in pairs.into_iter() {
                let mut raws = postcss::ast::RawData::default();
                raws.set_text("between", ":");
                let new_decl =
                  postcss::ast::nodes::declaration_with_raws(name, val, important, raws);
                rule.append(new_decl);
              }
            }
          } else if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[expand-shorthands:engine] rule skip prop='{}' (pairs none at apply)",
              prop
            );
          }
        }
      }

      Ok(())
    })
    .decl(|decl, _| {
      // Handle declarations that appear directly under Root or AtRule trees.
      // If the declaration lives under a normal Rule we let the rule hook handle it to avoid duplicate work.
      let parent = decl.to_node().borrow().parent();
      if let Some(p) = &parent {
        if postcss::ast::nodes::as_rule(p).is_some() {
          return Ok(());
        }
      }
      let prop = decl.prop();
      let value = decl.value();
      let important = decl.important();
      if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[expand-shorthands:engine] decl visit prop='{}' value='{}'",
          prop, value
        );
      }
      if let Some(pairs) = expand_shorthand_pairs(&prop.to_lowercase(), &value) {
        if std::env::var("COMPILED_CLI_TRACE").is_ok() {
          eprintln!(
            "[expand-shorthands:engine] decl pairs prop='{}' count={} -> {}",
            prop,
            pairs.len(),
            pairs
              .iter()
              .map(|(n, v)| format!("{}:{}", n, v))
              .collect::<Vec<_>>()
              .join(",")
          );
        }
        if let Some(container) = parent {
          // Find index of this declaration in the container
          let idx = {
            let borrowed = container.borrow();
            borrowed
              .nodes
              .iter()
              .position(|n| std::ptr::eq(n, &decl.to_node()))
          };
          if idx.is_some() {
            // Mutate the current declaration into the first longhand,
            // then clone_after() for remaining longhands to ensure
            // downstream plugins see updated nodes immediately.
            if let Some((first_name, first_val)) = pairs.get(0).cloned() {
              decl.set_prop(first_name);
              decl.set_value(first_val);
              decl.set_important(important);
              for (name, val) in pairs.into_iter().skip(1) {
                if let Some(new_decl) = decl.clone_after() {
                  new_decl.set_prop(name);
                  new_decl.set_value(val);
                  new_decl.set_important(important);
                }
              }
            }
          }
        }
      } else if std::env::var("COMPILED_CLI_TRACE").is_ok() {
        eprintln!(
          "[expand-shorthands:engine] decl skip prop='{}' (no pairs)",
          prop
        );
      }
      Ok(())
    })
    .at_rule(|at, _| {
      // Only process at-rules that contain a block with declarations (e.g., @media)
      let children = at.nodes();
      if children.is_empty() {
        return Ok(());
      }
      let mut targets: Vec<postcss::ast::NodeRef> = Vec::new();
      for child in children.iter() {
        if let Some(decl) = postcss::ast::nodes::as_declaration(child) {
          let prop = decl.prop();
          if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[expand-shorthands:engine] at-rule visit prop='{}' value='{}'",
              prop,
              decl.value()
            );
          }
          if expand_shorthand_pairs(&prop.to_lowercase(), &decl.value()).is_some() {
            targets.push(child.clone());
          } else if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[expand-shorthands:engine] at-rule skip prop='{}' (no pairs)",
              prop
            );
          }
        }
      }
      for child in targets {
        if let Some(decl) = postcss::ast::nodes::as_declaration(&child) {
          let prop = decl.prop();
          let value = decl.value();
          let important = decl.important();
          if let Some(pairs) = expand_shorthand_pairs(&prop.to_lowercase(), &value) {
            if std::env::var("COMPILED_CLI_TRACE").is_ok() {
              eprintln!(
                "[expand-shorthands:engine] at-rule pairs prop='{}' count={} -> {}",
                prop,
                pairs.len(),
                pairs
                  .iter()
                  .map(|(n, v)| format!("{}:{}", n, v))
                  .collect::<Vec<_>>()
                  .join(",")
              );
            }
            let idx_opt = at.child_index(&child);
            at.remove_child(child.clone());
            if let Some(idx) = idx_opt {
              let mut insert_at = idx;
              for (name, val) in pairs.into_iter() {
                let mut raws = postcss::ast::RawData::default();
                raws.set_text("between", ":");
                let new_decl =
                  postcss::ast::nodes::declaration_with_raws(name, val, important, raws);
                postcss::ast::Node::insert(&at.to_node(), insert_at, new_decl);
                insert_at += 1;
              }
            } else {
              for (name, val) in pairs.into_iter() {
                let mut raws = postcss::ast::RawData::default();
                raws.set_text("between", ":");
                let new_decl =
                  postcss::ast::nodes::declaration_with_raws(name, val, important, raws);
                at.append(new_decl);
              }
            }
          } else if std::env::var("COMPILED_CLI_TRACE").is_ok() {
            eprintln!(
              "[expand-shorthands:engine] at-rule skip prop='{}' (pairs none at apply)",
              prop
            );
          }
        }
      }
      Ok(())
    })
    .build()
}
