use swc_core::common::{DUMMY_SP, SyntaxContext};
use swc_core::ecma::ast::*;

use crate::evaluate_expr::{ValueOrNumber, eval_value_expr};
use crate::shorthand;
use crate::utils::{normalize_at_query, to_kebab_case};

use crate::CompiledCssInJsCollector;
use crate::RuntimeStyleEntries;

type DynamicAndEntry = (String, Expr, String);
type DynamicCondEntry = (String, Expr, String, String);
type DynamicAndPair = (Vec<(String, String)>, Expr, String);
type DynamicCondPair = (Vec<(String, String)>, Expr, String, String);
type DynamicAndEntries = Vec<DynamicAndEntry>;
type DynamicCondEntries = Vec<DynamicCondEntry>;
type DynamicAndPairs = Vec<DynamicAndPair>;
type DynamicCondPairs = Vec<DynamicCondPair>;

struct ForwardRefDynamicData {
  dynamic_and_entries: DynamicAndEntries,
  dynamic_cond_entries: DynamicCondEntries,
  dynamic_and_pairs: DynamicAndPairs,
  dynamic_cond_pairs: DynamicCondPairs,
  runtime_style_entries: RuntimeStyleEntries,
}

impl CompiledCssInJsCollector {
  fn alloc_unique_ident_from(&mut self, candidates: &[&str]) -> Ident {
    for c in candidates {
      let name = (*c).to_string();
      if !self.used_idents.contains(&name) {
        self.used_idents.insert(name.clone());
        return Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty());
      }
    }
    // Fallback to numbered suffixes for the first candidate
    let base = candidates.first().copied().unwrap_or("__cmpl");
    let mut idx: usize = 2;
    loop {
      let name = format!("{}{}", base, idx);
      if !self.used_idents.contains(&name) {
        self.used_idents.insert(name.clone());
        return Ident::new(name.into(), DUMMY_SP, SyntaxContext::empty());
      }
      idx += 1;
    }
  }

  fn join_classes(classes: Vec<String>) -> String {
    if classes.len() <= 1 {
      classes.into_iter().next().unwrap_or_default()
    } else {
      let mut s = String::with_capacity(
        classes.iter().map(|c| c.len()).sum::<usize>() + classes.len().saturating_sub(1),
      );
      let mut first = true;
      for c in classes {
        if !first {
          s.push(' ');
        }
        s.push_str(&c);
        first = false;
      }
      s
    }
  }

  /// Replace all identifiers matching `from` inside `expr` with `to_ident`.
  /// This is a shallow/recursive replacer used to map arrow param name to `__cmplp`.
  fn replace_ident_in_expr(expr: &mut Expr, from: &str, to_ident: &Ident) {
    match expr {
      Expr::Ident(i) => {
        if &*i.sym == from {
          *i = to_ident.clone();
        }
      }
      Expr::Member(m) => {
        if let Expr::Ident(i) = &mut *m.obj {
          if &*i.sym == from {
            *i = to_ident.clone();
          }
        } else {
          Self::replace_ident_in_expr(&mut m.obj, from, to_ident);
        }
        if let MemberProp::Computed(c) = &mut m.prop {
          Self::replace_ident_in_expr(&mut c.expr, from, to_ident);
        }
      }
      Expr::Call(c) => {
        if let Callee::Expr(e) = &mut c.callee {
          Self::replace_ident_in_expr(e, from, to_ident)
        }
        for a in &mut c.args {
          Self::replace_ident_in_expr(&mut a.expr, from, to_ident);
        }
      }
      Expr::Bin(b) => {
        Self::replace_ident_in_expr(&mut b.left, from, to_ident);
        Self::replace_ident_in_expr(&mut b.right, from, to_ident);
      }
      Expr::Cond(c) => {
        Self::replace_ident_in_expr(&mut c.test, from, to_ident);
        Self::replace_ident_in_expr(&mut c.cons, from, to_ident);
        Self::replace_ident_in_expr(&mut c.alt, from, to_ident);
      }
      Expr::Paren(p) => Self::replace_ident_in_expr(&mut p.expr, from, to_ident),
      Expr::Seq(s) => {
        for e in &mut s.exprs {
          Self::replace_ident_in_expr(e, from, to_ident);
        }
      }
      Expr::New(n) => {
        if let Some(args) = &mut n.args {
          for a in args {
            Self::replace_ident_in_expr(&mut a.expr, from, to_ident);
          }
        }
      }
      Expr::Assign(a) => {
        // Skip replacing inside assignment LHS; update RHS only
        Self::replace_ident_in_expr(&mut a.right, from, to_ident);
      }
      _ => {}
    }
  }

  fn collect_binding_pairs_from_param(param: &Pat) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Pat::Object(obj) = param {
      for p in &obj.props {
        match p {
          ObjectPatProp::KeyValue(kv) => {
            let key_name = match &kv.key {
              PropName::Ident(i) => i.sym.to_string(),
              PropName::Str(s) => s.value.to_string(),
              PropName::Num(n) => n.value.to_string(),
              PropName::BigInt(b) => b.value.to_string(),
              PropName::Computed(_) => continue,
            };
            if let Pat::Ident(BindingIdent { id, .. }) = &*kv.value {
              let local = id.sym.to_string();
              pairs.push((local, key_name));
            }
          }
          ObjectPatProp::Assign(a) => {
            let local = a.key.sym.to_string();
            pairs.push((local.clone(), local));
          }
          ObjectPatProp::Rest(_) => {}
        }
      }
    }
    pairs
  }

  fn replace_idents_with_members(
    expr: &mut Expr,
    rest_props_ident: &Ident,
    pairs: &[(String, String)],
  ) {
    match expr {
      Expr::Ident(i) => {
        for (local, prop_name) in pairs {
          if &*i.sym == local {
            *expr = Expr::Member(MemberExpr {
              span: DUMMY_SP,
              obj: Box::new(Expr::Ident(rest_props_ident.clone())),
              prop: MemberProp::Ident(
                Ident::new(prop_name.clone().into(), DUMMY_SP, SyntaxContext::empty()).into(),
              ),
            });
            return;
          }
        }
      }
      Expr::Member(m) => {
        Self::replace_idents_with_members(&mut m.obj, rest_props_ident, pairs);
        if let MemberProp::Computed(c) = &mut m.prop {
          Self::replace_idents_with_members(&mut c.expr, rest_props_ident, pairs);
        }
      }
      Expr::Call(c) => {
        if let Callee::Expr(e) = &mut c.callee {
          Self::replace_idents_with_members(e, rest_props_ident, pairs);
        }
        for a in &mut c.args {
          Self::replace_idents_with_members(&mut a.expr, rest_props_ident, pairs);
        }
      }
      Expr::Bin(b) => {
        Self::replace_idents_with_members(&mut b.left, rest_props_ident, pairs);
        Self::replace_idents_with_members(&mut b.right, rest_props_ident, pairs);
      }
      Expr::Cond(c) => {
        Self::replace_idents_with_members(&mut c.test, rest_props_ident, pairs);
        Self::replace_idents_with_members(&mut c.cons, rest_props_ident, pairs);
        Self::replace_idents_with_members(&mut c.alt, rest_props_ident, pairs);
      }
      Expr::Paren(p) => Self::replace_idents_with_members(&mut p.expr, rest_props_ident, pairs),
      Expr::Seq(s) => {
        for e in &mut s.exprs {
          Self::replace_idents_with_members(e, rest_props_ident, pairs);
        }
      }
      Expr::New(n) => {
        if let Some(args) = &mut n.args {
          for a in args {
            Self::replace_idents_with_members(&mut a.expr, rest_props_ident, pairs);
          }
        }
      }
      Expr::Assign(a) => Self::replace_idents_with_members(&mut a.right, rest_props_ident, pairs),
      _ => {}
    }
  }

  fn build_forward_ref_component(
    &mut self,
    tag_default: &str,
    joined_classes: String,
    dynamics: ForwardRefDynamicData,
  ) -> Expr {
    let ForwardRefDynamicData {
      dynamic_and_entries,
      dynamic_cond_entries,
      dynamic_and_pairs,
      dynamic_cond_pairs,
      runtime_style_entries,
    } = dynamics;
    // Ensure we will import/resolve forwardRef
    if self.forward_ref_ident.is_none() {
      // Prefer the canonical name and mark that we need an import
      let fr = Ident::new("forwardRef".into(), DUMMY_SP, SyntaxContext::empty());
      self.forward_ref_ident = Some(fr);
      self.need_forward_ref = true;
    }
    let forward_ref_ident = self.forward_ref_ident.clone().unwrap();

    // Allocate local identifiers avoiding clashes
    let c_ident = self.alloc_unique_ident_from(&["C", "__cmplC"]);
    let style_ident = self.alloc_unique_ident_from(&["__cmpls"]);
    let rest_props_ident = self.alloc_unique_ident_from(&["__cmplp"]);
    let ref_ident = self.alloc_unique_ident_from(&["__cmplr"]);
    let props_ident = self.alloc_unique_ident_from(&["props", "__cmplprops"]);

    // const { as: C = "div", style: __cmpls, ...__cmplp } = __cmplprops;
    let obj_pat = ObjectPat {
      span: DUMMY_SP,
      optional: false,
      type_ann: None,
      props: vec![
        ObjectPatProp::KeyValue(KeyValuePatProp {
          key: PropName::Ident(Ident::new("as".into(), DUMMY_SP, SyntaxContext::empty()).into()),
          value: Box::new(Pat::Assign(AssignPat {
            span: DUMMY_SP,
            left: Box::new(Pat::Ident(BindingIdent {
              id: c_ident.clone(),
              type_ann: None,
            })),
            right: Box::new(Expr::Lit(Lit::Str(Str {
              span: DUMMY_SP,
              value: tag_default.into(),
              raw: None,
            }))),
          })),
        }),
        ObjectPatProp::KeyValue(KeyValuePatProp {
          key: PropName::Ident(Ident::new("style".into(), DUMMY_SP, SyntaxContext::empty()).into()),
          value: Box::new(Pat::Ident(BindingIdent {
            id: style_ident.clone(),
            type_ann: None,
          })),
        }),
        ObjectPatProp::Rest(RestPat {
          span: DUMMY_SP,
          dot3_token: DUMMY_SP,
          arg: Box::new(Pat::Ident(BindingIdent {
            id: rest_props_ident.clone(),
            type_ann: None,
          })),
          type_ann: None,
        }),
      ],
    };
    let destructure_decl = Stmt::Decl(Decl::Var(Box::new(VarDecl {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      kind: VarDeclKind::Const,
      declare: false,
      decls: vec![VarDeclarator {
        span: DUMMY_SP,
        name: Pat::Object(obj_pat),
        init: Some(Box::new(Expr::Ident(props_ident.clone()))),
        definite: false,
      }],
    })));

    // if (__cmplp.innerRef) { throw new Error("Please use 'ref' instead of 'innerRef'."); }
    let inner_ref_check = Stmt::If(IfStmt {
      span: DUMMY_SP,
      test: Box::new(Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(Expr::Ident(rest_props_ident.clone())),
        prop: MemberProp::Ident(
          Ident::new("innerRef".into(), DUMMY_SP, SyntaxContext::empty()).into(),
        ),
      })),
      cons: Box::new(Stmt::Throw(ThrowStmt {
        span: DUMMY_SP,
        arg: Box::new(Expr::New(NewExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Box::new(Expr::Ident(Ident::new(
            "Error".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          ))),
          args: Some(vec![ExprOrSpread {
            spread: None,
            expr: Box::new(Expr::Lit(Lit::Str(Str {
              span: DUMMY_SP,
              value: "Please use 'ref' instead of 'innerRef'.".into(),
              raw: None,
            }))),
          }]),
          type_args: None,
        })),
      })),
      alt: None,
    });

    // Use runtime ax to merge classes and incoming className
    let ax_ident = Ident::new("ax".into(), DUMMY_SP, SyntaxContext::empty());
    // mark we need runtime ax import; defer insertion to visit_mut_module
    self.need_runtime_ax = true;
    let joined_str = Expr::Lit(Lit::Str(Str {
      span: DUMMY_SP,
      value: joined_classes.into(),
      raw: None,
    }));
    let class_prop_access = Expr::Member(MemberExpr {
      span: DUMMY_SP,
      obj: Box::new(Expr::Ident(rest_props_ident.clone())),
      prop: MemberProp::Ident(
        Ident::new("className".into(), DUMMY_SP, SyntaxContext::empty()).into(),
      ),
    });
    // Build dynamic entries: map param name in condition to __cmplp, and combine with class string via &&
    let mut ax_array_elems: Vec<Option<ExprOrSpread>> = Vec::new();
    ax_array_elems.push(Some(ExprOrSpread {
      spread: None,
      expr: Box::new(joined_str),
    }));
    for (param_name, mut cond_expr, dyn_joined) in dynamic_and_entries.into_iter() {
      // Replace param identifier with rest props ident
      Self::replace_ident_in_expr(&mut cond_expr, &param_name, &rest_props_ident);
      let right = Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: dyn_joined.into(),
        raw: None,
      }));
      let and_expr = Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::LogicalAnd,
        left: Box::new(cond_expr),
        right: Box::new(right),
      });
      ax_array_elems.push(Some(ExprOrSpread {
        spread: None,
        expr: Box::new(and_expr),
      }));
    }
    for (pairs, mut cond_expr, dyn_joined) in dynamic_and_pairs.into_iter() {
      if !pairs.is_empty() {
        Self::replace_idents_with_members(&mut cond_expr, &rest_props_ident, &pairs);
      }
      let right = Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: dyn_joined.into(),
        raw: None,
      }));
      let and_expr = Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op: BinaryOp::LogicalAnd,
        left: Box::new(cond_expr),
        right: Box::new(right),
      });
      ax_array_elems.push(Some(ExprOrSpread {
        spread: None,
        expr: Box::new(and_expr),
      }));
    }
    for (param_name, mut test_expr, cons_joined, alt_joined) in dynamic_cond_entries.into_iter() {
      Self::replace_ident_in_expr(&mut test_expr, &param_name, &rest_props_ident);
      let cons_str = Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: cons_joined.into(),
        raw: None,
      }));
      let alt_str = Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: alt_joined.into(),
        raw: None,
      }));
      let cond = Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(test_expr),
        cons: Box::new(cons_str),
        alt: Box::new(alt_str),
      });
      ax_array_elems.push(Some(ExprOrSpread {
        spread: None,
        expr: Box::new(cond),
      }));
    }
    for (pairs, mut test_expr, cons_joined, alt_joined) in dynamic_cond_pairs.into_iter() {
      if !pairs.is_empty() {
        Self::replace_idents_with_members(&mut test_expr, &rest_props_ident, &pairs);
      }
      let cons_str = Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: cons_joined.into(),
        raw: None,
      }));
      let alt_str = Expr::Lit(Lit::Str(Str {
        span: DUMMY_SP,
        value: alt_joined.into(),
        raw: None,
      }));
      let cond = Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test: Box::new(test_expr),
        cons: Box::new(cons_str),
        alt: Box::new(alt_str),
      });
      ax_array_elems.push(Some(ExprOrSpread {
        spread: None,
        expr: Box::new(cond),
      }));
    }
    // Finally, include incoming className
    ax_array_elems.push(Some(ExprOrSpread {
      spread: None,
      expr: Box::new(class_prop_access),
    }));
    let class_array = Expr::Array(ArrayLit {
      span: DUMMY_SP,
      elems: ax_array_elems,
    });
    let cond_merge = Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(ax_ident))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(class_array),
      }],
      type_args: None,
    });

    // Build style attribute: either passthrough or object merging runtime custom props
    let style_attr_value: Expr = if runtime_style_entries.is_empty() {
      Expr::Ident(style_ident.clone())
    } else {
      // mark ix import
      self.need_runtime_ix = true;
      let mut obj_props: Vec<PropOrSpread> = Vec::with_capacity(runtime_style_entries.len() + 1);
      obj_props.push(PropOrSpread::Spread(SpreadElement {
        dot3_token: DUMMY_SP,
        expr: Box::new(Expr::Ident(style_ident.clone())),
      }));
      for (custom_name, ix_arg, wrap_iife) in runtime_style_entries {
        let key = PropName::Str(Str {
          span: DUMMY_SP,
          value: custom_name.into(),
          raw: None,
        });
        let ix_arg_expr: Expr = if wrap_iife {
          let arrow = ArrowExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            params: vec![],
            body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
              span: DUMMY_SP,
              ctxt: SyntaxContext::empty(),
              stmts: vec![Stmt::Return(ReturnStmt {
                span: DUMMY_SP,
                arg: Some(Box::new(ix_arg)),
              })],
            })),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
          };
          Expr::Call(CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
              span: DUMMY_SP,
              expr: Box::new(Expr::Arrow(arrow)),
            }))),
            args: vec![],
            type_args: None,
          })
        } else {
          ix_arg
        };
        let call = Expr::Call(CallExpr {
          span: DUMMY_SP,
          ctxt: SyntaxContext::empty(),
          callee: Callee::Expr(Box::new(Expr::Ident(Ident::new(
            "ix".into(),
            DUMMY_SP,
            SyntaxContext::empty(),
          )))),
          args: vec![ExprOrSpread {
            spread: None,
            expr: Box::new(ix_arg_expr),
          }],
          type_args: None,
        });
        obj_props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
          key,
          value: Box::new(call),
        }))));
      }
      Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props: obj_props,
      })
    };

    // <C {...__cmplp} style={...} ref={__cmplr} className={...} />
    let jsx = Expr::JSXElement(Box::new(JSXElement {
      span: DUMMY_SP,
      opening: JSXOpeningElement {
        span: DUMMY_SP,
        name: JSXElementName::Ident(c_ident.clone()),
        attrs: vec![
          JSXAttrOrSpread::SpreadElement(SpreadElement {
            dot3_token: DUMMY_SP,
            expr: Box::new(Expr::Ident(rest_props_ident.clone())),
          }),
          JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name: JSXAttrName::Ident(
              Ident::new("style".into(), DUMMY_SP, SyntaxContext::empty()).into(),
            ),
            value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
              span: DUMMY_SP,
              expr: JSXExpr::Expr(Box::new(style_attr_value)),
            })),
          }),
          JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name: JSXAttrName::Ident(
              Ident::new("ref".into(), DUMMY_SP, SyntaxContext::empty()).into(),
            ),
            value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
              span: DUMMY_SP,
              expr: JSXExpr::Expr(Box::new(Expr::Ident(ref_ident.clone()))),
            })),
          }),
          JSXAttrOrSpread::JSXAttr(JSXAttr {
            span: DUMMY_SP,
            name: JSXAttrName::Ident(
              Ident::new("className".into(), DUMMY_SP, SyntaxContext::empty()).into(),
            ),
            value: Some(JSXAttrValue::JSXExprContainer(JSXExprContainer {
              span: DUMMY_SP,
              expr: JSXExpr::Expr(Box::new(cond_merge)),
            })),
          }),
        ],
        self_closing: true,
        type_args: None,
      },
      children: vec![],
      closing: None,
    }));

    let ret_stmt = Stmt::Return(ReturnStmt {
      span: DUMMY_SP,
      arg: Some(Box::new(jsx)),
    });

    // (props, ref) => { const {...} = props; if (...) throw ...; return <C .../> }
    let arrow = ArrowExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      params: vec![
        Pat::Ident(BindingIdent {
          id: props_ident.clone(),
          type_ann: None,
        }),
        Pat::Ident(BindingIdent {
          id: ref_ident.clone(),
          type_ann: None,
        }),
      ],
      body: Box::new(BlockStmtOrExpr::BlockStmt(BlockStmt {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        stmts: vec![destructure_decl, inner_ref_check, ret_stmt],
      })),
      is_async: false,
      is_generator: false,
      type_params: None,
      return_type: None,
    };

    Expr::Call(CallExpr {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      callee: Callee::Expr(Box::new(Expr::Ident(forward_ref_ident))),
      args: vec![ExprOrSpread {
        spread: None,
        expr: Box::new(Expr::Arrow(arrow)),
      }],
      type_args: None,
    })
  }

  pub fn try_transform_styled_call(
    &mut self,
    member: &MemberExpr,
    call: &CallExpr,
  ) -> Option<Expr> {
    // styled.tag({ ... })
    if call.args.len() != 1 {
      return None;
    }
    let tag = match &member.prop {
      MemberProp::Ident(i) => i.sym.to_string(),
      _ => return None,
    };
    let obj = match &*call.args[0].expr {
      Expr::Object(o) => o,
      _ => return None,
    };
    // Collect classes for all non-function values; allow runtime fallback for non-const values
    self.begin_runtime_collection();
    let mut classes = self.collect_atomic_classes_from_object(obj);
    // Collect runtime custom props for function-valued declarations; dedupe by ix arg path
    let mut runtime_entries: Vec<(String, Expr, bool)> = Vec::new();
    use std::collections::HashMap as StdHashMap;
    let mut ix_key_to_var: StdHashMap<String, String> = StdHashMap::new();

    fn member_path_key(e: &Expr) -> Option<String> {
      fn rec(expr: &Expr, out: &mut Vec<String>) -> bool {
        match expr {
          Expr::Member(m) => {
            if !rec(&m.obj, out) {
              return false;
            }
            match &m.prop {
              MemberProp::Ident(i) => {
                out.push(i.sym.to_string());
                true
              }
              _ => false,
            }
          }
          Expr::Ident(i) => {
            out.push(i.sym.to_string());
            true
          }
          _ => false,
        }
      }
      let mut parts: Vec<String> = Vec::new();
      if rec(e, &mut parts) {
        Some(parts.join("."))
      } else {
        None
      }
    }
    for p in &obj.props {
      if let PropOrSpread::Prop(pp) = p
        && let Prop::KeyValue(kv) = &**pp
      {
        let original_prop_name = match &kv.key {
          PropName::Ident(i) => i.sym.to_string(),
          PropName::Str(s) => s.value.to_string(),
          PropName::Num(n) => n.value.to_string(),
          PropName::BigInt(b) => b.value.to_string(),
          PropName::Computed(_) => continue,
        };
        if let Expr::Arrow(a) = &*kv.value
          && a.params.len() == 1
        {
          let prop_kebab = to_kebab_case(&original_prop_name);
          // Attempt to derive ix argument from arrow
          let mut ix_arg_opt: Option<(Expr, bool)> = None;
          match &a.params[0] {
            Pat::Ident(BindingIdent { id, .. }) => {
              // Body can be ident (same as param.property) or member of param
              if let BlockStmtOrExpr::Expr(body_e) = &*a.body {
                match &**body_e {
                  Expr::Member(m) => {
                    // p => p.width OR p => (p.width)
                    let mut obj_expr = (*m.obj).clone();
                    // replace param ident with __cmplp
                    Self::replace_ident_in_expr(
                      &mut obj_expr,
                      id.sym.as_ref(),
                      &Ident::new("__cmplp".into(), DUMMY_SP, SyntaxContext::empty()),
                    );
                    ix_arg_opt = Some((
                      Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(obj_expr),
                        prop: m.prop.clone(),
                      }),
                      false,
                    ));
                  }
                  Expr::Ident(local) => {
                    // p => prop (unsupported unless prop == param)
                    // Treat as __cmplp[local]
                    ix_arg_opt = Some((
                      Expr::Member(MemberExpr {
                        span: DUMMY_SP,
                        obj: Box::new(Expr::Ident(Ident::new(
                          "__cmplp".into(),
                          DUMMY_SP,
                          SyntaxContext::empty(),
                        ))),
                        prop: MemberProp::Ident(local.clone().into()),
                      }),
                      false,
                    ));
                  }
                  Expr::Bin(bin_expr) => {
                    // Support p => p.dim.width + 10 + "px"
                    let mut cloned = Expr::Bin(bin_expr.clone());
                    Self::replace_ident_in_expr(
                      &mut cloned,
                      id.sym.as_ref(),
                      &Ident::new("__cmplp".into(), DUMMY_SP, SyntaxContext::empty()),
                    );
                    ix_arg_opt = Some((cloned, true));
                  }
                  _ => {}
                }
              }
            }
            Pat::Object(obj_pat) => {
              // ({ width }) => width ; ({ width: w }) => w
              if obj_pat.props.len() == 1 {
                match &obj_pat.props[0] {
                  ObjectPatProp::KeyValue(kvp) => {
                    let key_name = match &kvp.key {
                      PropName::Ident(i) => i.sym.to_string(),
                      PropName::Str(s) => s.value.to_string(),
                      _ => String::new(),
                    };
                    if key_name.is_empty() {
                      continue;
                    }
                    let local_ident = match &*kvp.value {
                      Pat::Ident(BindingIdent { id, .. }) => id.clone(),
                      _ => continue,
                    };
                    if let BlockStmtOrExpr::Expr(body_e) = &*a.body {
                      match &**body_e {
                        Expr::Ident(local) if local.sym == local_ident.sym => {
                          ix_arg_opt = Some((
                            Expr::Member(MemberExpr {
                              span: DUMMY_SP,
                              obj: Box::new(Expr::Ident(Ident::new(
                                "__cmplp".into(),
                                DUMMY_SP,
                                SyntaxContext::empty(),
                              ))),
                              prop: MemberProp::Ident(
                                Ident::new(key_name.into(), DUMMY_SP, SyntaxContext::empty())
                                  .into(),
                              ),
                            }),
                            false,
                          ));
                        }
                        _ => {}
                      }
                    }
                  }
                  ObjectPatProp::Assign(ap) => {
                    let key_name = ap.key.sym.to_string();
                    if let BlockStmtOrExpr::Expr(body_e) = &*a.body
                      && let Expr::Ident(local) = &**body_e
                      && local.sym == ap.key.sym
                    {
                      ix_arg_opt = Some((
                        Expr::Member(MemberExpr {
                          span: DUMMY_SP,
                          obj: Box::new(Expr::Ident(Ident::new(
                            "__cmplp".into(),
                            DUMMY_SP,
                            SyntaxContext::empty(),
                          ))),
                          prop: MemberProp::Ident(
                            Ident::new(key_name.into(), DUMMY_SP, SyntaxContext::empty()).into(),
                          ),
                        }),
                        false,
                      ));
                    }
                  }
                  _ => {}
                }
              }
            }
            _ => {}
          }
          if let Some((ix_arg, need_iife)) = ix_arg_opt {
            // Emit atomic rule using var(--token) where token stable per ix arg path
            let ix_key = member_path_key(&ix_arg).unwrap_or_else(|| prop_kebab.clone());
            let var_name = ix_key_to_var
              .entry(ix_key.clone())
              .or_insert_with(|| format!("--{}", self.hash_ix(&ix_key)))
              .clone();
            let value_text = format!("var({})", var_name);
            let mut tmp: Vec<String> = Vec::new();
            self.emit_rule_input(
              crate::emit::RuleInput {
                prop_kebab: &prop_kebab,
                value_text: std::borrow::Cow::Borrowed(value_text.as_str()),
                suffix: "",
                wrappers: &[],
              },
              &mut tmp,
            );
            classes.extend(tmp);
            if !runtime_entries.iter().any(|(k, _, _)| k == &var_name) {
              runtime_entries.push((var_name, ix_arg, need_iife));
            }
          }
        }
      }
    }
    // Merge runtime entries captured during object collection for non-function values
    let captured = self.take_runtime_entries();
    if !captured.is_empty() {
      for (v, e, w) in captured {
        if !runtime_entries.iter().any(|(k, _, _)| k == &v) {
          runtime_entries.push((v, e, w));
        }
      }
    }
    let joined = Self::join_classes(classes);
    Some(self.build_forward_ref_component(
      &tag,
      joined,
      ForwardRefDynamicData {
        dynamic_and_entries: DynamicAndEntries::new(),
        dynamic_cond_entries: DynamicCondEntries::new(),
        dynamic_and_pairs: DynamicAndPairs::new(),
        dynamic_cond_pairs: DynamicCondPairs::new(),
        runtime_style_entries: runtime_entries,
      },
    ))
  }

  pub fn try_transform_styled_tagged_tpl(
    &mut self,
    member: &MemberExpr,
    tt: &TaggedTpl,
  ) -> Option<Expr> {
    // styled.tag`...`
    let tag = match &member.prop {
      MemberProp::Ident(i) => i.sym.to_string(),
      _ => return None,
    };
    // Build base css text while capturing function interpolations for dynamic classes
    let tpl = &tt.tpl;
    // Approximate capacity based on raw quasis lengths
    let approx_cap: usize = tpl
      .quasis
      .iter()
      .map(|q| q.raw.as_ref().len())
      .sum::<usize>()
      + tpl.exprs.len() * 8;
    let mut css_buf = String::with_capacity(approx_cap);
    let mut dynamics: DynamicAndEntries = DynamicAndEntries::new();
    let mut dynamics_cond: DynamicCondEntries = DynamicCondEntries::new();
    let mut dynamics_pairs: DynamicAndPairs = DynamicAndPairs::new();
    let mut dynamics_cond_pairs: DynamicCondPairs = DynamicCondPairs::new();
    let mut runtime_style_entries: RuntimeStyleEntries = RuntimeStyleEntries::new();
    // Enable runtime var capture for non-const template interpolations
    self.begin_runtime_collection();

    fn backscan_property_name(s: &str) -> Option<(usize, String)> {
      let bytes = s.as_bytes();
      let mut i: isize = (bytes.len() as isize) - 1;
      // skip whitespace
      while i >= 0 && bytes[i as usize].is_ascii_whitespace() {
        i -= 1;
      }
      // find colon
      while i >= 0 && bytes[i as usize] != b':' {
        i -= 1;
      }
      if i < 0 {
        return None;
      }
      i -= 1; // move before ':'
      // skip spaces
      while i >= 0 && bytes[i as usize].is_ascii_whitespace() {
        i -= 1;
      }
      let end = i as usize + 1;
      // scan backward to previous delimiter ; { } or newline
      while i >= 0 {
        let b = bytes[i as usize];
        if b == b'\n' || b == b';' || b == b'{' || b == b'}' {
          break;
        }
        i -= 1;
      }
      let start = i as usize + 1;
      let prop_slice = &s[start..end];
      // Remove block comments from the slice
      let mut cleaned = String::with_capacity(prop_slice.len());
      let mut j = 0usize;
      let n = prop_slice.len();
      while j < n {
        if j + 1 < n && prop_slice.as_bytes()[j] == b'/' && prop_slice.as_bytes()[j + 1] == b'*' {
          // skip until closing */
          j += 2;
          while j + 1 < n {
            if prop_slice.as_bytes()[j] == b'*' && prop_slice.as_bytes()[j + 1] == b'/' {
              j += 2;
              break;
            }
            j += 1;
          }
          continue;
        }
        cleaned.push(prop_slice.as_bytes()[j] as char);
        j += 1;
      }
      let cleaned = cleaned.trim();
      // Take the last token (after spaces), as comments may leave leading spaces
      let name = cleaned.split_whitespace().last().unwrap_or("");
      if name.is_empty() {
        None
      } else {
        Some((start, name.to_string()))
      }
    }

    fn value_to_raw_text(v: &Expr) -> Option<String> {
      match v {
        Expr::Lit(Lit::Str(s)) => Some(s.value.to_string()),
        Expr::Lit(Lit::Num(n)) => {
          let txt = if (n.value - (n.value as i64 as f64)).abs() < f64::EPSILON {
            (n.value as i64).to_string()
          } else {
            n.value.to_string()
          };
          Some(txt)
        }
        _ => None,
      }
    }

    fn emit_value_classes_for_prop(
      this: &mut CompiledCssInJsCollector,
      prop_name_raw: &str,
      value_expr: &Expr,
      suffix: &str,
      wrappers: &[String],
    ) -> Vec<String> {
      let mut out_classes: Vec<String> = Vec::new();
      let prop_kebab = to_kebab_case(prop_name_raw);
      let mut value_text: Option<String> = None;
      if let Some(v) = eval_value_expr(value_expr, &this.const_env) {
        match v {
          ValueOrNumber::Str(s) => value_text = Some(s),
          ValueOrNumber::Num(n) => {
            let mut s = if (n - (n as i64 as f64)).abs() < f64::EPSILON {
              (n as i64).to_string()
            } else {
              n.to_string()
            };
            if !CompiledCssInJsCollector::is_unitless_property_generic(prop_name_raw) {
              s.push_str("px");
            }
            value_text = Some(s);
          }
        }
      } else if let Some(raw) = value_to_raw_text(value_expr) {
        // literal fallback
        let mut s = raw;
        if let Ok(n) = s.parse::<f64>()
          && !CompiledCssInJsCollector::is_unitless_property_generic(prop_name_raw)
        {
          s = if (n - (n as i64 as f64)).abs() < f64::EPSILON {
            (n as i64).to_string()
          } else {
            n.to_string()
          };
          s.push_str("px");
        }
        value_text = Some(s);
      }
      if let Some(vtxt) = value_text {
        if shorthand::is_shorthand_prop(&prop_kebab) {
          let val_for_sh = if vtxt.chars().any(|c| c.is_alphabetic()) {
            ValueOrNumber::Str(vtxt.clone())
          } else if let Ok(num) = vtxt.parse::<f64>() {
            ValueOrNumber::Num(num)
          } else {
            ValueOrNumber::Str(vtxt.clone())
          };
          if let Some(expanded) = shorthand::try_expand_shorthand(&prop_kebab, &val_for_sh) {
            for (sub_prop, sub_val) in expanded {
              this.emit_rule_input(
                crate::emit::RuleInput {
                  prop_kebab: &sub_prop,
                  value_text: std::borrow::Cow::Borrowed(sub_val.as_str()),
                  suffix,
                  wrappers,
                },
                &mut out_classes,
              );
            }
          } else {
            this.emit_rule_input(
              crate::emit::RuleInput {
                prop_kebab: &prop_kebab,
                value_text: std::borrow::Cow::Borrowed(vtxt.as_str()),
                suffix,
                wrappers,
              },
              &mut out_classes,
            );
          }
        } else {
          this.emit_rule_input(
            crate::emit::RuleInput {
              prop_kebab: &prop_kebab,
              value_text: std::borrow::Cow::Borrowed(vtxt.as_str()),
              suffix,
              wrappers,
            },
            &mut out_classes,
          );
        }
      }
      out_classes
    }

    // Parse wrappers in the current chunk and update a stack of (wrapper, depth)
    fn update_wrappers_from_chunk(chunk: &str, stack: &mut Vec<(String, usize)>) {
      let b = chunk.as_bytes();
      let n = b.len();
      let mut i = 0usize;
      let mut in_str: u8 = 0;
      while i < n {
        let c = b[i];
        if in_str != 0 {
          if c == in_str {
            in_str = 0;
          } else if c == b'\\' && i + 1 < n {
            i += 1;
          }
          i += 1;
          continue;
        }
        if c == b'\'' || c == b'"' {
          in_str = c;
          i += 1;
          continue;
        }
        if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
          // comment
          i += 2;
          while i + 1 < n {
            if b[i] == b'*' && b[i + 1] == b'/' {
              i += 2;
              break;
            }
            i += 1;
          }
          continue;
        }
        if c == b'@' {
          let start = i;
          i += 1;
          while i < n && b[i] != b'{' && b[i] != b'}' {
            i += 1;
          }
          let header = &chunk[start..i].trim();
          let wrapper = if let Some(stripped) = header.strip_prefix("@media") {
            let normalized = normalize_at_query(stripped.trim());
            if normalized.starts_with('(') {
              format!("@media {}", normalized)
            } else if normalized.is_empty() {
              "@media".to_string()
            } else {
              format!("@media ({})", normalized)
            }
          } else if let Some(stripped) = header.strip_prefix("@supports") {
            let normalized = normalize_at_query(stripped.trim());
            if normalized.starts_with('(') {
              format!("@supports {}", normalized)
            } else if normalized.is_empty() {
              "@supports".to_string()
            } else {
              format!("@supports ({})", normalized)
            }
          } else {
            String::new()
          };
          if i < n && b[i] == b'{' {
            if !wrapper.is_empty() {
              stack.push((wrapper, 1));
            } else if let Some(top) = stack.last_mut() {
              top.1 += 1;
            }
            i += 1;
            continue;
          }
        }
        if c == b'{' {
          if let Some(top) = stack.last_mut() {
            top.1 += 1;
          }
          i += 1;
          continue;
        }
        if c == b'}' {
          if let Some(mut top) = stack.pop()
            && top.1 > 1
          {
            top.1 -= 1;
            stack.push(top);
          }
          i += 1;
          continue;
        }
        i += 1;
      }
    }

    // Track nested selector suffixes like &:hover, & .child:hover, etc.
    fn update_selector_suffix_from_chunk(chunk: &str, stack: &mut Vec<(String, usize)>) {
      let b = chunk.as_bytes();
      let n = b.len();
      let mut i = 0usize;
      let mut in_str: u8 = 0;
      while i < n {
        let c = b[i];
        if in_str != 0 {
          if c == in_str {
            in_str = 0;
          } else if c == b'\\' && i + 1 < n {
            i += 1;
          }
          i += 1;
          continue;
        }
        if c == b'\'' || c == b'"' {
          in_str = c;
          i += 1;
          continue;
        }
        if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
          i += 2;
          while i + 1 < n {
            if b[i] == b'*' && b[i + 1] == b'/' {
              i += 2;
              break;
            }
            i += 1;
          }
          continue;
        }
        if c == b'{' {
          // scan back to previous delimiter to find header
          let mut j: isize = (i as isize) - 1;
          while j >= 0 && b[j as usize].is_ascii_whitespace() {
            j -= 1;
          }
          let end = (j as usize) + 1;
          while j >= 0 {
            let ch = b[j as usize];
            if ch == b'}' || ch == b';' || ch == b'{' || ch == b'\n' {
              break;
            }
            j -= 1;
          }
          let start = (j as usize) + 1;
          if end > start {
            let header = &chunk[start..end].trim();
            if !header.is_empty() && !header.starts_with('@') {
              // Normalize to suffix (strip leading '&')
              let mut suf = header.to_string();
              if let Some(stripped) = suf.strip_prefix('&') {
                suf = stripped.to_string();
              }
              // Preserve as-is; emit_rule_input will append after ".class"
              stack.push((suf, 1));
              i += 1;
              continue;
            }
          }
          if let Some(top) = stack.last_mut() {
            top.1 += 1;
          }
          i += 1;
          continue;
        }
        if c == b'}' {
          if let Some(mut top) = stack.pop()
            && top.1 > 1
          {
            top.1 -= 1;
            stack.push(top);
          }
          i += 1;
          continue;
        }
        i += 1;
      }
    }

    let mut wrapper_stack: Vec<(String, usize)> = Vec::new();
    let mut selector_suffix_stack: Vec<(String, usize)> = Vec::new();
    for (idx, quasi) in tpl.quasis.iter().enumerate() {
      css_buf.push_str(quasi.raw.as_ref());
      update_wrappers_from_chunk(quasi.raw.as_ref(), &mut wrapper_stack);
      update_selector_suffix_from_chunk(quasi.raw.as_ref(), &mut selector_suffix_stack);
      if let Some(expr) = tpl.exprs.get(idx) {
        // First, try to fold to constant string (e.g., keyframes names)
        if let Some(s) = self.eval_expr_to_string(expr) {
          css_buf.push_str(&s);
          continue;
        }
        // Detect arrow function param => param.cond && ({...})
        if let Expr::Arrow(arrow) = &**expr
          && arrow.params.len() == 1
        {
          let param_pat = &arrow.params[0];
          let binding_pairs: Vec<(String, String)> =
            Self::collect_binding_pairs_from_param(param_pat);
          if let Pat::Ident(BindingIdent {
            id: param_ident, ..
          }) = param_pat
          {
            // Only support concise body expressions for now
            if let BlockStmtOrExpr::Expr(body_expr) = &*arrow.body {
              // Unwrap parens
              let mut e: &Expr = body_expr;
              if let Expr::Paren(p) = e {
                e = &*p.expr;
              }
              if let Expr::Bin(bin) = e {
                if matches!(bin.op, BinaryOp::LogicalAnd) {
                  // Right side should be object (possibly wrapped in parens)
                  let mut right_expr: &Expr = &bin.right;
                  if let Expr::Paren(p) = right_expr {
                    right_expr = &*p.expr;
                  }
                  if let Expr::Object(style_obj) = right_expr {
                    let classes = self.collect_atomic_classes_from_object(style_obj);
                    let joined_dyn = Self::join_classes(classes);
                    let cond_clone = (*bin.left).clone();
                    if !binding_pairs.is_empty() { /* mapping handled at build time */ }
                    dynamics.push((param_ident.sym.to_string(), cond_clone, joined_dyn));
                    // Do not append anything to css for this expr
                    continue;
                  }
                }
                // Ternary dynamic property values: test ? cons : alt
              } else if let Expr::Cond(cond) = e {
                if let Some((prop_start, prop_name)) = backscan_property_name(&css_buf) {
                  // remove the pending "prop: " from base css buffer
                  if prop_start <= css_buf.len() {
                    css_buf.truncate(prop_start);
                  }
                  let wrappers_now: Vec<String> =
                    wrapper_stack.iter().map(|(w, _)| w.clone()).collect();
                  let suffix_now: String = selector_suffix_stack
                    .last()
                    .map(|(s, _)| s.clone())
                    .unwrap_or_default();
                  let classes_cons = emit_value_classes_for_prop(
                    self,
                    &prop_name,
                    &cond.cons,
                    &suffix_now,
                    &wrappers_now,
                  );
                  let classes_alt = emit_value_classes_for_prop(
                    self,
                    &prop_name,
                    &cond.alt,
                    &suffix_now,
                    &wrappers_now,
                  );
                  let joined_cons = Self::join_classes(classes_cons);
                  let joined_alt = Self::join_classes(classes_alt);
                  let test_clone = (*cond.test).clone();
                  if binding_pairs.is_empty() {
                    dynamics_cond.push((
                      param_ident.sym.to_string(),
                      test_clone,
                      joined_cons,
                      joined_alt,
                    ));
                  } else {
                    dynamics_cond_pairs.push((
                      binding_pairs.clone(),
                      test_clone,
                      joined_cons,
                      joined_alt,
                    ));
                  }
                  continue;
                } else {
                  // Non-property ternary: expect branches to evaluate to class strings (e.g., css vars)
                  if let (Some(cons_str), Some(alt_str)) = (
                    self.eval_expr_to_string(&cond.cons),
                    self.eval_expr_to_string(&cond.alt),
                  ) {
                    let test_clone = (*cond.test).clone();
                    if binding_pairs.is_empty() {
                      dynamics_cond.push((
                        param_ident.sym.to_string(),
                        test_clone,
                        cons_str,
                        alt_str,
                      ));
                    } else {
                      dynamics_cond_pairs.push((
                        binding_pairs.clone(),
                        test_clone,
                        cons_str,
                        alt_str,
                      ));
                    }
                    continue;
                  }
                }
              }
            }
          } else {
            // Destructured param case (no param ident); handle ternary
            if let BlockStmtOrExpr::Expr(body_expr) = &*arrow.body {
              let mut e: &Expr = body_expr;
              if let Expr::Paren(p) = e {
                e = &*p.expr;
              }
              if let Expr::Cond(cond) = e {
                if let Some((prop_start, prop_name)) = backscan_property_name(&css_buf) {
                  if prop_start <= css_buf.len() {
                    css_buf.truncate(prop_start);
                  }
                  let wrappers_now: Vec<String> =
                    wrapper_stack.iter().map(|(w, _)| w.clone()).collect();
                  let suffix_now: String = selector_suffix_stack
                    .last()
                    .map(|(s, _)| s.clone())
                    .unwrap_or_default();
                  let classes_cons = emit_value_classes_for_prop(
                    self,
                    &prop_name,
                    &cond.cons,
                    &suffix_now,
                    &wrappers_now,
                  );
                  let classes_alt = emit_value_classes_for_prop(
                    self,
                    &prop_name,
                    &cond.alt,
                    &suffix_now,
                    &wrappers_now,
                  );
                  let joined_cons = Self::join_classes(classes_cons);
                  let joined_alt = Self::join_classes(classes_alt);
                  let test_expr = (*cond.test).clone();
                  dynamics_cond_pairs.push((
                    binding_pairs.clone(),
                    test_expr,
                    joined_cons,
                    joined_alt,
                  ));
                  continue;
                } else if let (Some(cons_str), Some(alt_str)) = (
                  self.eval_expr_to_string(&cond.cons),
                  self.eval_expr_to_string(&cond.alt),
                ) {
                  let test_expr = (*cond.test).clone();
                  dynamics_cond_pairs.push((binding_pairs.clone(), test_expr, cons_str, alt_str));
                  continue;
                }
              } else if let Expr::Bin(bin) = e
                && matches!(bin.op, BinaryOp::LogicalAnd)
              {
                let mut right_expr: &Expr = &bin.right;
                if let Expr::Paren(p) = right_expr {
                  right_expr = &*p.expr;
                }
                if let Expr::Object(style_obj) = right_expr {
                  let classes = self.collect_atomic_classes_from_object(style_obj);
                  let joined_dyn = Self::join_classes(classes);
                  let cond_clone = (*bin.left).clone();
                  dynamics_pairs.push((binding_pairs.clone(), cond_clone, joined_dyn));
                  continue;
                }
              }
            }
          }
        }
        // Otherwise, emit runtime var in CSS context
        let var_name = self.create_runtime_var_for_expr(expr, false);
        css_buf.push_str("var(");
        css_buf.push_str(&var_name);
        css_buf.push(')');
      }
    }
    // Merge any captured runtime entries from template
    let captured = self.take_runtime_entries();
    if !captured.is_empty() {
      for (v, e, w) in captured {
        runtime_style_entries.push((v, e, w));
      }
    }
    // Also merge keyframes runtime entries based on animation-name in built css
    {
      let s = css_buf.as_str();
      let bytes = s.as_bytes();
      let mut i: usize = 0;
      while i < bytes.len() {
        // naive scan for "animation-name"
        if i + 14 <= bytes.len() {
          let slice = &s[i..i + 14];
          if slice.eq_ignore_ascii_case("animation-name") {
            // advance to ':'
            while i < bytes.len() && bytes[i] != b':' {
              i += 1;
            }
            if i < bytes.len() {
              i += 1;
            }
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
              i += 1;
            }
            let start = i;
            while i < bytes.len() && bytes[i] != b';' && bytes[i] != b'}' {
              i += 1;
            }
            let value = s[start..i].trim();
            for part in value.split(',') {
              let name = part.trim().trim_matches('"').trim_matches('\'');
              if name.is_empty() {
                continue;
              }
              if let Some(entries) = self.keyframes_name_to_runtime.get(name) {
                for (v, e, w) in entries.clone() {
                  runtime_style_entries.push((v, e, w));
                }
              }
            }
            continue;
          }
        }
        i += 1;
      }
    }
    let classes = self.collect_atomic_classes_from_css_text(&css_buf);
    let joined = Self::join_classes(classes);
    Some(self.build_forward_ref_component(
      &tag,
      joined,
      ForwardRefDynamicData {
        dynamic_and_entries: dynamics,
        dynamic_cond_entries: dynamics_cond,
        dynamic_and_pairs: dynamics_pairs,
        dynamic_cond_pairs: dynamics_cond_pairs,
        runtime_style_entries,
      },
    ))
  }
}
