use rustc_hash::FxHashMap;
use swc_core::{atoms::Atom, ecma::ast::*};

#[derive(Clone)]
pub enum ConstValue {
  Str(String),
  Num(f64),
  Obj(FxHashMap<String, ConstValue>),
  Func(FunctionDef),
}

#[derive(Clone)]
pub struct FunctionDef {
  pub is_arrow: bool,
  pub arrow: Option<Box<ArrowExpr>>, // if is_arrow
  pub func: Option<Box<Function>>,   // if !is_arrow
}

pub enum ValueOrNumber {
  Str(String),
  Num(f64),
}

// Runtime fallback signaling removed; we only expose const evaluators here

// Evaluate a const initializer into a ConstValue (supports str/num/obj/func and nested)
pub fn eval_const_expr(expr: &Expr) -> Option<ConstValue> {
  match expr {
    Expr::Paren(p) => eval_const_expr(&p.expr),
    Expr::Lit(Lit::Str(s)) => Some(ConstValue::Str(s.value.to_string())),
    Expr::Lit(Lit::Num(n)) => Some(ConstValue::Num(n.value)),
    Expr::Object(obj) => eval_object_literal_to_const(obj),
    Expr::Arrow(a) => Some(ConstValue::Func(FunctionDef {
      is_arrow: true,
      arrow: Some(Box::new(a.clone())),
      func: None,
    })),
    Expr::Fn(f) => Some(ConstValue::Func(FunctionDef {
      is_arrow: false,
      arrow: None,
      func: Some(f.function.clone()),
    })),
    Expr::Tpl(t) => {
      // allow const s = `${a}${b}` with a/b being literals inlined later by eval_value_expr through env
      // Here we only evaluate templates with no expressions
      if t.exprs.is_empty() {
        let mut s = String::new();
        for q in &t.quasis {
          s.push_str(q.raw.as_ref());
        }
        Some(ConstValue::Str(s))
      } else {
        None
      }
    }
    _ => None,
  }
}

fn eval_object_literal_to_const(obj: &ObjectLit) -> Option<ConstValue> {
  let mut map: FxHashMap<String, ConstValue> = FxHashMap::default();
  for p in &obj.props {
    match p {
      PropOrSpread::Prop(pp) => {
        if let Prop::KeyValue(kv) = &**pp {
          let key = prop_name_to_string(&kv.key)?;
          let val = match &*kv.value {
            Expr::Lit(Lit::Str(s)) => ConstValue::Str(s.value.to_string()),
            Expr::Lit(Lit::Num(n)) => ConstValue::Num(n.value),
            Expr::Object(o) => match eval_object_literal_to_const(o) {
              Some(ConstValue::Obj(m)) => ConstValue::Obj(m),
              _ => continue,
            },
            Expr::Arrow(a) => ConstValue::Func(FunctionDef {
              is_arrow: true,
              arrow: Some(Box::new(a.clone())),
              func: None,
            }),
            Expr::Fn(f) => ConstValue::Func(FunctionDef {
              is_arrow: false,
              arrow: None,
              func: Some(f.function.clone()),
            }),
            _ => continue,
          };
          map.insert(key, val);
        }
      }
      PropOrSpread::Spread(_) => {
        // skip spreads inside const object initializers for now (rare), keep simple
      }
    }
  }
  Some(ConstValue::Obj(map))
}

fn eval_object_literal_to_const_with_env(
  obj: &ObjectLit,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<ConstValue> {
  let mut map: FxHashMap<String, ConstValue> = FxHashMap::default();
  for p in &obj.props {
    match p {
      PropOrSpread::Prop(pp) => {
        match &**pp {
          Prop::KeyValue(kv) => {
            let key = prop_name_to_string(&kv.key)?;
            // Try full const-eval first to allow nested objects
            if let Some(cv) = eval_expr_to_const(&kv.value, env) {
              map.insert(key, cv);
              continue;
            }
            // Fallback to simple value evaluation (str/num)
            if let Some(v) = eval_value_expr(&kv.value, env) {
              match v {
                ValueOrNumber::Str(s) => {
                  map.insert(key, ConstValue::Str(s));
                }
                ValueOrNumber::Num(n) => {
                  map.insert(key, ConstValue::Num(n));
                }
              }
              continue;
            }
          }
          Prop::Shorthand(ident) => {
            // Support object shorthand like { fontSize } by resolving from env
            let key = ident.sym.to_string();
            if let Some(val) = env.get(&ident.sym) {
              match val {
                ConstValue::Str(s) => {
                  map.insert(key, ConstValue::Str(s.clone()));
                }
                ConstValue::Num(n) => {
                  map.insert(key, ConstValue::Num(*n));
                }
                ConstValue::Obj(m) => {
                  map.insert(key, ConstValue::Obj(m.clone()));
                }
                ConstValue::Func(f) => {
                  map.insert(key, ConstValue::Func(f.clone()));
                }
              }
            }
          }
          _ => {}
        }
      }
      PropOrSpread::Spread(sp) => {
        // Support spreading const objects inside object literal
        if let Some(inner) = eval_object_expr(&sp.expr, env) {
          for (k, v) in inner {
            map.insert(k, v);
          }
        }
      }
    }
  }
  Some(ConstValue::Obj(map))
}

fn prop_name_to_string(n: &PropName) -> Option<String> {
  Some(match n {
    PropName::Ident(i) => i.sym.to_string(),
    PropName::Str(s) => s.value.to_string(),
    PropName::Num(n) => n.value.to_string(),
    PropName::BigInt(b) => b.value.to_string(),
    PropName::Computed(_) => return None,
  })
}

// Evaluate expression into Str/Num if possible (ident/member/bin/tpl/call)
pub fn eval_value_expr(expr: &Expr, env: &FxHashMap<Atom, ConstValue>) -> Option<ValueOrNumber> {
  match expr {
    Expr::Paren(p) => eval_value_expr(&p.expr, env),
    Expr::Lit(Lit::Str(s)) => Some(ValueOrNumber::Str(s.value.to_string())),
    Expr::Lit(Lit::Num(n)) => Some(ValueOrNumber::Num(n.value)),
    Expr::Ident(i) => {
      if &*i.sym == "runtime" {
        return None;
      }
      match env.get(&i.sym)? {
        ConstValue::Str(s) => Some(ValueOrNumber::Str(s.clone())),
        ConstValue::Num(n) => Some(ValueOrNumber::Num(*n)),
        _ => None,
      }
    }
    Expr::Member(m) => eval_member_to_value(m, env),
    Expr::Bin(b) if b.op == BinaryOp::Add => {
      let l = eval_value_expr(&b.left, env)?;
      let r = eval_value_expr(&b.right, env)?;
      match (l, r) {
        (ValueOrNumber::Num(a), ValueOrNumber::Num(b)) => Some(ValueOrNumber::Num(a + b)),
        (ValueOrNumber::Str(a), ValueOrNumber::Str(b)) => {
          Some(ValueOrNumber::Str(format!("{}{}", a, b)))
        }
        (ValueOrNumber::Str(a), ValueOrNumber::Num(b)) => {
          Some(ValueOrNumber::Str(format!("{}{}", a, num_to_string(b))))
        }
        (ValueOrNumber::Num(a), ValueOrNumber::Str(b)) => {
          Some(ValueOrNumber::Str(format!("{}{}", num_to_string(a), b)))
        }
      }
    }
    Expr::Tpl(t) => {
      let mut s = String::new();
      for (i, q) in t.quasis.iter().enumerate() {
        s.push_str(q.raw.as_ref());
        if let Some(e) = t.exprs.get(i) {
          let v = eval_value_expr(e, env)?;
          match v {
            ValueOrNumber::Str(st) => s.push_str(&st),
            ValueOrNumber::Num(n) => s.push_str(&num_to_string(n)),
          }
        }
      }
      Some(ValueOrNumber::Str(s))
    }
    Expr::Call(c) => eval_call_to_value(c, env),
    _ => None,
  }
}

fn eval_member_to_value(
  member: &MemberExpr,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<ValueOrNumber> {
  // Supports obj.prop where obj is const object
  let (obj_val, key) = eval_member_target(member, env)?;
  match obj_val {
    ConstValue::Obj(map) => match map.get(&key)? {
      ConstValue::Str(s) => Some(ValueOrNumber::Str(s.clone())),
      ConstValue::Num(n) => Some(ValueOrNumber::Num(*n)),
      _ => None,
    },
    _ => None,
  }
}

fn eval_member_target(
  member: &MemberExpr,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<(ConstValue, String)> {
  // Evaluate object part
  let obj_val = match &*member.obj {
    Expr::Ident(i) => {
      if &*i.sym == "runtime" {
        return None;
      }
      env.get(&i.sym)?.clone()
    }
    Expr::Member(m2) => {
      let (owner, key2) = eval_member_target(m2, env)?;
      match owner {
        ConstValue::Obj(m) => m.get(&key2)?.clone(),
        _ => return None,
      }
    }
    _ => return None,
  };
  // Evaluate property key
  let key = match &member.prop {
    MemberProp::Ident(i) => i.sym.to_string(),
    MemberProp::Computed(c) => match &*c.expr {
      Expr::Lit(Lit::Str(s)) => s.value.to_string(),
      Expr::Lit(Lit::Num(n)) => num_to_string(n.value),
      Expr::Ident(i) => match env.get(&i.sym)? {
        ConstValue::Str(s) => s.clone(),
        ConstValue::Num(n) => num_to_string(*n),
        _ => return None,
      },
      _ => return None,
    },
    MemberProp::PrivateName(_) => return None,
  };
  Some((obj_val, key))
}

fn eval_call_to_value(call: &CallExpr, env: &FxHashMap<Atom, ConstValue>) -> Option<ValueOrNumber> {
  if !call.args.is_empty() {
    return None;
  } // only no-arg functions supported
  let func = resolve_callee_to_func(&call.callee, env)?;
  let ret = eval_function_to_const_value(&func, env)?;
  match ret {
    ConstValue::Str(s) => Some(ValueOrNumber::Str(s)),
    ConstValue::Num(n) => Some(ValueOrNumber::Num(n)),
    _ => None,
  }
}

fn resolve_callee_to_func(
  callee: &Callee,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<FunctionDef> {
  match callee {
    Callee::Expr(e) => match &**e {
      Expr::Ident(i) => match env.get(&i.sym)? {
        ConstValue::Func(f) => Some(f.clone()),
        _ => None,
      },
      Expr::Member(m) => {
        let (obj_val, key) = eval_member_target(m, env)?;
        match obj_val {
          ConstValue::Obj(map) => match map.get(&key)? {
            ConstValue::Func(f) => Some(f.clone()),
            _ => None,
          },
          _ => None,
        }
      }
      _ => None,
    },
    _ => None,
  }
}

fn eval_function_to_const_value(
  func: &FunctionDef,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<ConstValue> {
  if func.is_arrow {
    let a = func.arrow.as_ref()?;
    if !a.params.is_empty() {
      return None;
    }
    match &*a.body {
      BlockStmtOrExpr::Expr(e) => eval_expr_to_const(e, env),
      BlockStmtOrExpr::BlockStmt(b) => eval_block_return_expr_to_const(b, env),
    }
  } else {
    let f = func.func.as_ref()?;
    if !f.params.is_empty() {
      return None;
    }
    let body = f.body.as_ref()?;
    eval_block_return_expr_to_const(body, env)
  }
}

fn eval_block_return_expr_to_const(
  block: &BlockStmt,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<ConstValue> {
  for stmt in &block.stmts {
    if let Stmt::Return(ReturnStmt {
      arg: Some(expr), ..
    }) = stmt
    {
      return eval_expr_to_const(expr, env);
    }
  }
  None
}

pub fn eval_expr_to_const(expr: &Expr, env: &FxHashMap<Atom, ConstValue>) -> Option<ConstValue> {
  match expr {
    Expr::Paren(p) => eval_expr_to_const(&p.expr, env),
    Expr::Lit(Lit::Str(s)) => Some(ConstValue::Str(s.value.to_string())),
    Expr::Lit(Lit::Num(n)) => Some(ConstValue::Num(n.value)),
    Expr::Tpl(t) => {
      let mut s = String::new();
      for (i, q) in t.quasis.iter().enumerate() {
        s.push_str(q.raw.as_ref());
        if let Some(e) = t.exprs.get(i) {
          match eval_value_expr(e, env)? {
            ValueOrNumber::Str(st) => s.push_str(&st),
            ValueOrNumber::Num(n) => s.push_str(&num_to_string(n)),
          }
        }
      }
      Some(ConstValue::Str(s))
    }
    Expr::Object(o) => eval_object_literal_to_const_with_env(o, env),
    Expr::Ident(i) => {
      if &*i.sym == "runtime" {
        return None;
      }
      env.get(&i.sym).cloned()
    }
    Expr::Member(m) => {
      let (owner, key) = eval_member_target(m, env)?;
      match owner {
        ConstValue::Obj(map) => map.get(&key).cloned(),
        _ => None,
      }
    }
    Expr::Bin(b) if b.op == BinaryOp::Add => {
      let l = eval_value_expr(&b.left, env)?;
      let r = eval_value_expr(&b.right, env)?;
      match (l, r) {
        (ValueOrNumber::Num(a), ValueOrNumber::Num(b)) => Some(ConstValue::Num(a + b)),
        (ValueOrNumber::Str(a), ValueOrNumber::Str(b)) => {
          Some(ConstValue::Str(format!("{}{}", a, b)))
        }
        (ValueOrNumber::Str(a), ValueOrNumber::Num(b)) => {
          Some(ConstValue::Str(format!("{}{}", a, num_to_string(b))))
        }
        (ValueOrNumber::Num(a), ValueOrNumber::Str(b)) => {
          Some(ConstValue::Str(format!("{}{}", num_to_string(a), b)))
        }
      }
    }
    Expr::Call(c) => {
      let v = eval_call_to_value(c, env)?;
      match v {
        ValueOrNumber::Str(s) => Some(ConstValue::Str(s)),
        ValueOrNumber::Num(n) => Some(ConstValue::Num(n)),
      }
    }
    _ => None,
  }
}

pub fn eval_object_expr(
  expr: &Expr,
  env: &FxHashMap<Atom, ConstValue>,
) -> Option<FxHashMap<String, ConstValue>> {
  match expr {
    Expr::Object(o) => match eval_object_literal_to_const(o)? {
      ConstValue::Obj(m) => Some(m),
      _ => None,
    },
    Expr::Ident(i) => match env.get(&i.sym)?.clone() {
      ConstValue::Obj(m) => Some(m),
      _ => None,
    },
    Expr::Member(m) => {
      let (owner, key) = eval_member_target(m, env)?;
      match owner {
        ConstValue::Obj(map) => match map.get(&key)? {
          ConstValue::Obj(m) => Some(m.clone()),
          _ => None,
        },
        _ => None,
      }
    }
    Expr::Call(c) => {
      if !c.args.is_empty() {
        return None;
      }
      let func = resolve_callee_to_func(&c.callee, env)?;
      match eval_function_to_const_value(&func, env)? {
        ConstValue::Obj(m) => Some(m),
        _ => None,
      }
    }
    _ => None,
  }
}

fn num_to_string(n: f64) -> String {
  if (n - (n as i64 as f64)).abs() < f64::EPSILON {
    (n as i64).to_string()
  } else {
    n.to_string()
  }
}

// Note: runtime handling happens within collectors; not needed here.
