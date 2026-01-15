#![allow(clippy::result_large_err, clippy::large_enum_variant)]

mod tokenizer;

pub use tokenizer::{Token, TokenKind, Tokenizer};

use crate::ast::nodes::{self, Root};
use crate::ast::{self, Node, NodeData, NodeRef};
use crate::css_syntax_error::CssSyntaxError;
use crate::input::{Input, InputOptions, InputRef, Position};
use crate::source_map::{MapSetting, PreviousMapError};

#[derive(Clone, Debug, Default)]
pub struct ParseOptions {
  pub from: Option<String>,
  pub map: MapSetting,
  pub ignore_errors: bool,
}

#[derive(Debug)]
pub enum ParseError {
  PreviousMap(PreviousMapError),
  Css(CssSyntaxError),
}

impl std::fmt::Display for ParseError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ParseError::PreviousMap(err) => write!(f, "{}", err),
      ParseError::Css(err) => write!(f, "{}", err),
    }
  }
}

impl std::error::Error for ParseError {}

impl From<PreviousMapError> for ParseError {
  fn from(err: PreviousMapError) -> Self {
    ParseError::PreviousMap(err)
  }
}

impl From<CssSyntaxError> for ParseError {
  fn from(err: CssSyntaxError) -> Self {
    ParseError::Css(err)
  }
}

fn token_start(token: &Token) -> Option<usize> {
  token.start
}

fn token_end(token: &Token) -> Option<usize> {
  token.end.or_else(|| {
    token
      .start
      .map(|start| start + token.value.len().saturating_sub(1))
  })
}

fn token_after(token: &Token) -> Option<usize> {
  token_end(token).map(|end| end + 1)
}

fn find_last_with_position(tokens: &[Token]) -> Option<usize> {
  tokens
    .iter()
    .rev()
    .find_map(|token| token_end(token).or_else(|| token_start(token)))
}

struct Parser {
  input: InputRef,
  tokenizer: Tokenizer,
  root: Root,
  current: NodeRef,
  spaces: String,
  semicolon: bool,
}

impl Parser {
  fn new(input: InputRef, ignore_errors: bool) -> Self {
    let root = Root::new();
    {
      let mut borrowed = root.raw().borrow_mut();
      borrowed.source.input = Some(input.clone());
      borrowed.source.start = Some(Position::new(1, 1, 0));
    }
    let tokenizer = Tokenizer::new(input.clone(), ignore_errors);
    let current = root.raw().clone();
    Self {
      input,
      tokenizer,
      root,
      current,
      spaces: String::new(),
      semicolon: false,
    }
  }

  #[allow(clippy::result_large_err)]
  fn parse(mut self) -> Result<Root, CssSyntaxError> {
    while !self.tokenizer.end_of_file() {
      let token = match self.tokenizer.next_token(false)? {
        Some(token) => token,
        None => break,
      };

      match token.kind {
        TokenKind::Space => self.spaces.push_str(&token.value),
        TokenKind::Semicolon => self.free_semicolon(&token),
        TokenKind::CloseCurly => self.end(token)?,
        TokenKind::Comment => self.comment(token)?,
        TokenKind::AtWord => self.atrule(token)?,
        TokenKind::OpenCurly => self.empty_rule(token)?,
        _ => self.other(token)?,
      }
    }

    self.end_file()?;
    Ok(self.root)
  }

  fn get_position(&self, offset: usize) -> Position {
    self.input.from_offset(offset)
  }

  fn set_end_position(&self, node: &NodeRef, offset: usize) {
    let position = self.input.from_offset(offset);
    ast::set_end_position(node, position);
  }

  fn init(&mut self, node: &NodeRef, offset: Option<usize>) {
    Node::append(&self.current, node.clone());
    {
      let mut borrowed = node.borrow_mut();
      borrowed.source.input = Some(self.input.clone());
      let start_offset = offset.unwrap_or(0);
      borrowed.source.start = Some(self.get_position(start_offset));
      borrowed.raws.set_text("before", self.spaces.clone());
    }
    self.spaces.clear();
    if !matches!(node.borrow().data, NodeData::Comment(_)) {
      self.semicolon = false;
    }
  }

  fn free_semicolon(&mut self, token: &Token) {
    self.spaces.push_str(&token.value);
    let previous = {
      let borrowed = self.current.borrow();
      borrowed.nodes.last().cloned()
    };
    if let Some(prev) = previous {
      if matches!(prev.borrow().data, NodeData::Rule(_)) {
        let mut prev_borrow = prev.borrow_mut();
        let own_semicolon = prev_borrow.raws.get_text("ownSemicolon").unwrap_or("");
        if own_semicolon.is_empty() {
          prev_borrow
            .raws
            .set_text("ownSemicolon", self.spaces.clone());
          self.spaces.clear();
        }
      }
    }
  }

  fn spaces_and_comments_from_end(&self, tokens: &mut Vec<Token>) -> String {
    let mut spaces = String::new();
    while let Some(token) = tokens.last() {
      match token.kind {
        TokenKind::Space | TokenKind::Comment => {
          let token = tokens.pop().unwrap();
          spaces = format!("{}{}", token.value, spaces);
        }
        _ => break,
      }
    }
    spaces
  }

  fn spaces_and_comments_from_start(&self, tokens: &mut Vec<Token>) -> String {
    let mut spaces = String::new();
    while let Some(token) = tokens.first() {
      match token.kind {
        TokenKind::Space | TokenKind::Comment => {
          let token = tokens.remove(0);
          spaces.push_str(&token.value);
        }
        _ => break,
      }
    }
    spaces
  }

  fn spaces_from_end(&self, tokens: &mut Vec<Token>) -> String {
    let mut spaces = String::new();
    while let Some(token) = tokens.last() {
      if matches!(token.kind, TokenKind::Space) {
        let token = tokens.pop().unwrap();
        spaces = format!("{}{}", token.value, spaces);
      } else {
        break;
      }
    }
    spaces
  }

  fn set_property(node: &NodeRef, prop: &str, value: String) {
    let mut borrowed = node.borrow_mut();
    match &mut borrowed.data {
      NodeData::AtRule(data) if prop == "params" => data.params = value,
      NodeData::Rule(data) if prop == "selector" => data.selector = value,
      NodeData::Declaration(data) if prop == "value" => data.value = value,
      _ => {}
    }
  }

  fn raw(&self, node: &NodeRef, prop: &str, tokens: Vec<Token>, custom_property: bool) {
    let length = tokens.len();
    let mut value = String::new();
    let mut clean = true;

    for (i, token) in tokens.iter().enumerate() {
      match token.kind {
        TokenKind::Space if i == length - 1 && !custom_property => {
          clean = false;
        }
        TokenKind::Comment => {
          let prev_kind = if i == 0 {
            None
          } else {
            Some(tokens[i - 1].kind.clone())
          };
          let next_kind = tokens.get(i + 1).map(|t| t.kind.clone());
          let prev_safe = prev_kind.is_none() || matches!(prev_kind, Some(TokenKind::Space));
          let next_safe = next_kind.is_none() || matches!(next_kind, Some(TokenKind::Space));
          if !prev_safe && !next_safe {
            if value.ends_with(',') {
              clean = false;
            } else {
              value.push_str(&token.value);
            }
          } else {
            clean = false;
          }
        }
        _ => value.push_str(&token.value),
      }
    }

    if clean {
      node.borrow_mut().raws.remove(prop);
    } else {
      let raw: String = tokens.iter().map(|t| t.value.as_str()).collect();
      node
        .borrow_mut()
        .raws
        .set_value_pair(prop, value.clone(), raw);
    }

    Self::set_property(node, prop, value);
  }

  fn colon(&self, tokens: &[Token]) -> Result<Option<usize>, CssSyntaxError> {
    let mut brackets = 0isize;
    let mut prev: Option<&Token> = None;

    for (index, token) in tokens.iter().enumerate() {
      match token.kind {
        TokenKind::OpenParenthesis => brackets += 1,
        TokenKind::CloseParenthesis => brackets -= 1,
        TokenKind::Colon if brackets == 0 => {
          if prev.is_none() {
            self.double_colon(token)?;
          } else if prev
            .map(|t| t.kind == TokenKind::Word && t.value.eq_ignore_ascii_case("progid"))
            .unwrap_or(false)
          {
            // continue scanning
          } else {
            return Ok(Some(index));
          }
        }
        _ => {}
      }
      prev = Some(token);
    }

    Ok(None)
  }

  fn check_missed_semicolon(&self, tokens: &[Token]) -> Result<(), CssSyntaxError> {
    let Some(colon_index) = self.colon(tokens)? else {
      return Ok(());
    };

    let mut target = &tokens[colon_index];
    let mut found = 0usize;
    for token in tokens[..colon_index].iter().rev() {
      if !matches!(token.kind, TokenKind::Space) {
        found += 1;
        target = token;
        if found == 2 {
          break;
        }
      }
    }

    let offset = if matches!(target.kind, TokenKind::Word) {
      token_after(target).unwrap_or_else(|| token_start(target).unwrap_or(0))
    } else {
      token_start(target).unwrap_or(0)
    };

    Err(
      self
        .input
        .error("Missed semicolon", self.get_position(offset), None),
    )
  }

  fn comment(&mut self, token: Token) -> Result<(), CssSyntaxError> {
    let node = Node::new(NodeData::Comment(nodes::CommentData::default()));
    self.init(&node, token_start(&token));

    if let Some(end) = token_after(&token) {
      self.set_end_position(&node, end);
    }

    let raw_text = if token.value.len() >= 4 {
      &token.value[2..token.value.len() - 2]
    } else {
      ""
    };

    {
      let mut borrowed = node.borrow_mut();
      if let Some(data) = borrowed.as_comment_mut() {
        if raw_text.trim().is_empty() {
          data.text.clear();
          borrowed.raws.set_text("left", raw_text.to_string());
          borrowed.raws.set_text("right", String::new());
        } else {
          let left_len = raw_text
            .chars()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum::<usize>();
          let right_len = raw_text
            .chars()
            .rev()
            .take_while(|c| c.is_whitespace())
            .map(|c| c.len_utf8())
            .sum::<usize>();
          let middle_end = raw_text.len().saturating_sub(right_len);
          let middle = &raw_text[left_len..middle_end];
          data.text = middle.to_string();
          borrowed
            .raws
            .set_text("left", raw_text[..left_len].to_string());
          borrowed
            .raws
            .set_text("right", raw_text[middle_end..].to_string());
        }
      }
    }

    Ok(())
  }

  fn double_colon(&self, token: &Token) -> Result<(), CssSyntaxError> {
    let start = token_start(token).unwrap_or(0);
    let end = token_after(token).unwrap_or(start);
    Err(self.input.error(
      "Double colon",
      self.get_position(start),
      Some(self.get_position(end)),
    ))
  }

  fn empty_rule(&mut self, token: Token) -> Result<(), CssSyntaxError> {
    let node = Node::new(NodeData::Rule(nodes::RuleData::default()));
    self.init(&node, token_start(&token));
    {
      let mut borrowed = node.borrow_mut();
      borrowed.raws.set_text("between", String::new());
      if let Some(data) = borrowed.as_rule_mut() {
        data.selector = String::new();
      }
    }
    self.current = node;
    Ok(())
  }

  fn end(&mut self, token: Token) -> Result<(), CssSyntaxError> {
    {
      let mut current = self.current.borrow_mut();
      if !current.nodes.is_empty() {
        current
          .raws
          .set_text("semicolon", if self.semicolon { "true" } else { "false" });
      }
      let mut after = current.raws.get_text("after").unwrap_or("").to_string();
      after.push_str(&self.spaces);
      current.raws.set_text("after", after);
    }

    self.spaces.clear();
    self.semicolon = false;

    if let Some(parent) = Node::parent_ref(&self.current) {
      if let Some(end) = token_after(&token) {
        self.set_end_position(&self.current, end);
      }
      self.current = parent;
      Ok(())
    } else {
      self.unexpected_close(&token)
    }
  }

  fn end_file(&mut self) -> Result<(), CssSyntaxError> {
    if Node::parent_ref(&self.current).is_some() {
      self.unclosed_block()?;
    }

    {
      let mut current = self.current.borrow_mut();
      if !current.nodes.is_empty() {
        current
          .raws
          .set_text("semicolon", if self.semicolon { "true" } else { "false" });
      }
      let mut after = current.raws.get_text("after").unwrap_or("").to_string();
      after.push_str(&self.spaces);
      current.raws.set_text("after", after);
    }

    let position = self.tokenizer.position();
    self.set_end_position(self.root.raw(), position);
    Ok(())
  }

  fn atrule(&mut self, token: Token) -> Result<(), CssSyntaxError> {
    let name = token.value.trim_start_matches('@').to_string();
    if name.is_empty() {
      return self.unnamed_at_rule(&token);
    }

    let node = Node::new(NodeData::AtRule(nodes::AtRuleData::default()));
    {
      let mut borrowed = node.borrow_mut();
      if let Some(data) = borrowed.as_at_rule_mut() {
        data.name = name;
      }
    }

    self.init(&node, token_start(&token));

    let mut params: Vec<Token> = Vec::new();
    let mut brackets: Vec<TokenKind> = Vec::new();
    let mut open = false;
    let mut last = false;

    while !self.tokenizer.end_of_file() {
      let next_token = self.tokenizer.next_token(false)?;
      let Some(token) = next_token else {
        break;
      };
      let kind = token.kind.clone();

      match kind {
        TokenKind::OpenParenthesis => brackets.push(TokenKind::CloseParenthesis),
        TokenKind::OpenSquare => brackets.push(TokenKind::CloseSquare),
        TokenKind::OpenCurly if !brackets.is_empty() => brackets.push(TokenKind::CloseCurly),
        _ => {}
      }

      if !brackets.is_empty() {
        if kind == *brackets.last().unwrap() {
          brackets.pop();
        }
        params.push(token);
        continue;
      }

      match kind {
        TokenKind::Semicolon => {
          if let Some(end) = token_after(&token) {
            self.set_end_position(&node, end);
          }
          self.semicolon = true;
          break;
        }
        TokenKind::OpenCurly => {
          open = true;
          break;
        }
        TokenKind::CloseCurly => {
          if !params.is_empty() {
            if let Some(prev) = find_last_with_position(&params) {
              self.set_end_position(&node, prev + 1);
            }
          }
          self.end(token)?;
          return Ok(());
        }
        _ => params.push(token),
      }

      if self.tokenizer.end_of_file() {
        last = true;
        break;
      }
    }

    let between = self.spaces_and_comments_from_end(&mut params);
    node.borrow_mut().raws.set_text("between", between.clone());

    if !params.is_empty() {
      let after_name = self.spaces_and_comments_from_start(&mut params);
      node.borrow_mut().raws.set_text("afterName", after_name);
      self.raw(&node, "params", params.clone(), false);
      if last {
        if let Some(last_token) = params.last().and_then(token_after) {
          self.set_end_position(&node, last_token);
        }
        self.spaces = between;
        node.borrow_mut().raws.set_text("between", String::new());
      }
    } else {
      node.borrow_mut().raws.set_text("afterName", String::new());
      if let Some(data) = node.borrow_mut().as_at_rule_mut() {
        data.params = String::new();
      }
    }

    if open {
      node.borrow_mut().nodes = Vec::new();
      self.current = node;
    }

    Ok(())
  }

  fn decl(&mut self, mut tokens: Vec<Token>, custom_property: bool) -> Result<(), CssSyntaxError> {
    let node = Node::new(NodeData::Declaration(nodes::DeclarationData::default()));
    let start = tokens.first().and_then(token_start);
    self.init(&node, start);

    if let Some(last) = tokens.last() {
      if matches!(last.kind, TokenKind::Semicolon) {
        self.semicolon = true;
        tokens.pop();
      }
    }

    if let Some(last) = tokens.last() {
      if let Some(end) = token_after(last) {
        self.set_end_position(&node, end);
      } else if let Some(prev) = find_last_with_position(&tokens) {
        self.set_end_position(&node, prev + 1);
      }
    }

    let mut before = node
      .borrow()
      .raws
      .get_text("before")
      .unwrap_or("")
      .to_string();

    if tokens.len() == 1 && !matches!(tokens[0].kind, TokenKind::Word) {
      self.unknown_word(&tokens)?;
    }

    while let Some(token) = tokens.first() {
      if matches!(token.kind, TokenKind::Word) {
        break;
      }
      if tokens.len() == 1 {
        self.unknown_word(&tokens)?;
      }
      before.push_str(&token.value);
      tokens.remove(0);
    }
    node.borrow_mut().raws.set_text("before", before);

    if let Some(start) = tokens.first().and_then(token_start) {
      node.borrow_mut().source.start = Some(self.get_position(start));
    }

    let mut prop = String::new();
    while let Some(token) = tokens.first() {
      if matches!(
        token.kind,
        TokenKind::Colon | TokenKind::Space | TokenKind::Comment
      ) {
        break;
      }
      prop.push_str(&token.value);
      tokens.remove(0);
    }
    if let Some(data) = node.borrow_mut().as_declaration_mut() {
      data.prop = prop.clone();
    }
    node.borrow_mut().raws.set_text("between", String::new());

    let mut between = String::new();
    while let Some(token) = tokens.first() {
      let kind = token.kind.clone();
      let token = tokens.remove(0);
      if matches!(kind, TokenKind::Colon) {
        between.push_str(&token.value);
        break;
      } else {
        if matches!(kind, TokenKind::Word) && token.value.chars().any(|c| c.is_alphanumeric()) {
          self.unknown_word(std::slice::from_ref(&token))?;
        }
        between.push_str(&token.value);
      }
    }
    node.borrow_mut().raws.set_text("between", between);

    if let Some(first) = prop.chars().next() {
      if first == '_' || first == '*' {
        let mut borrowed = node.borrow_mut();
        let mut before = borrowed.raws.get_text("before").unwrap_or("").to_string();
        before.push(first);
        borrowed.raws.set_text("before", before);
        if let Some(data) = borrowed.as_declaration_mut() {
          data.prop = prop[first.len_utf8()..].to_string();
        }
      }
    }

    let mut first_spaces: Vec<Token> = Vec::new();
    while let Some(token) = tokens.first() {
      if matches!(token.kind, TokenKind::Space | TokenKind::Comment) {
        first_spaces.push(tokens.remove(0));
      } else {
        break;
      }
    }

    self.precheck_missed_semicolon(&tokens);

    for i in (0..tokens.len()).rev() {
      let token = tokens[i].clone();
      if token.value.eq_ignore_ascii_case("!important") {
        if let Some(data) = node.borrow_mut().as_declaration_mut() {
          data.important = true;
        }
        let tail = tokens.split_off(i);
        let mut string: String = tail.iter().map(|t| t.value.clone()).collect();
        let spaces = self.spaces_from_end(&mut tokens);
        string = format!("{}{}", spaces, string);
        if string != " !important" {
          node.borrow_mut().raws.set_text("important", string);
        }
        break;
      } else if token.value.eq_ignore_ascii_case("important") {
        let mut cache = tokens.clone();
        let mut str_value = String::new();
        while let Some(last) = cache.pop() {
          let kind = last.kind.clone();
          if str_value.trim_start().starts_with('!') && !matches!(kind, TokenKind::Space) {
            cache.push(last);
            break;
          }
          str_value = format!("{}{}", last.value, str_value);
        }
        if str_value.trim_start().starts_with('!') {
          if let Some(data) = node.borrow_mut().as_declaration_mut() {
            data.important = true;
          }
          node.borrow_mut().raws.set_text("important", str_value);
          tokens = cache;
        }
        break;
      }
      if !matches!(token.kind, TokenKind::Space | TokenKind::Comment) {
        break;
      }
    }

    let has_word = tokens
      .iter()
      .any(|t| !matches!(t.kind, TokenKind::Space | TokenKind::Comment));
    if has_word {
      let mut prefix = String::new();
      for token in &first_spaces {
        prefix.push_str(&token.value);
      }
      let mut borrowed = node.borrow_mut();
      let between = borrowed.raws.get_text("between").unwrap_or("").to_string();
      borrowed
        .raws
        .set_text("between", format!("{}{}", between, prefix));
    }

    let mut value_tokens = first_spaces.clone();
    if has_word {
      value_tokens.clear();
    }
    value_tokens.extend(tokens.clone());
    self.raw(&node, "value", value_tokens, custom_property);

    let value = {
      let borrowed = node.borrow();
      if let NodeData::Declaration(data) = &borrowed.data {
        data.value.clone()
      } else {
        String::new()
      }
    };

    if !custom_property && value.contains(':') {
      self.check_missed_semicolon(&tokens)?;
    }

    Ok(())
  }

  fn rule(&mut self, mut tokens: Vec<Token>) -> Result<(), CssSyntaxError> {
    tokens.pop();
    let node = Node::new(NodeData::Rule(nodes::RuleData::default()));
    let start = tokens.first().and_then(token_start);
    self.init(&node, start);
    let between = self.spaces_and_comments_from_end(&mut tokens);
    node.borrow_mut().raws.set_text("between", between);
    self.raw(&node, "selector", tokens, false);
    self.current = node;
    Ok(())
  }

  fn other(&mut self, start: Token) -> Result<(), CssSyntaxError> {
    let mut tokens = vec![start.clone()];
    let mut colon = false;
    let mut bracket: Option<Token> = None;
    let mut brackets: Vec<TokenKind> = Vec::new();
    let custom_property = start.value.starts_with("--");
    loop {
      let next = self.tokenizer.next_token(false)?;
      let Some(token) = next else {
        break;
      };
      let kind = token.kind.clone();
      tokens.push(token.clone());

      match kind {
        TokenKind::OpenParenthesis => {
          if bracket.is_none() {
            bracket = Some(token.clone());
          }
          brackets.push(TokenKind::CloseParenthesis);
        }
        TokenKind::OpenSquare => {
          if bracket.is_none() {
            bracket = Some(token.clone());
          }
          brackets.push(TokenKind::CloseSquare);
        }
        TokenKind::OpenCurly if custom_property && colon => {
          if bracket.is_none() {
            bracket = Some(token.clone());
          }
          brackets.push(TokenKind::CloseCurly);
        }
        _ => {}
      }

      if !brackets.is_empty() {
        if kind == *brackets.last().unwrap() {
          brackets.pop();
          if brackets.is_empty() {
            bracket = None;
          }
        }
        continue;
      }

      match kind {
        TokenKind::Semicolon => {
          self.decl(tokens, custom_property)?;
          return Ok(());
        }
        TokenKind::OpenCurly => {
          self.rule(tokens)?;
          return Ok(());
        }
        TokenKind::CloseCurly => {
          tokens.pop();
          self.tokenizer.back(token);
          break;
        }
        TokenKind::Colon => colon = true,
        _ => {}
      }

      if self.tokenizer.end_of_file() {
        break;
      }
    }

    if !brackets.is_empty() {
      if let Some(bracket_token) = bracket {
        self.unclosed_bracket(&bracket_token)?;
      }
    }

    if colon {
      if !custom_property {
        while let Some(token) = tokens.last() {
          if matches!(token.kind, TokenKind::Space | TokenKind::Comment) {
            let token = tokens.pop().unwrap();
            self.tokenizer.back(token);
          } else {
            break;
          }
        }
      }
      self.decl(tokens, custom_property)?;
    } else {
      self.unknown_word(&tokens)?;
    }

    Ok(())
  }

  fn precheck_missed_semicolon(&self, _tokens: &[Token]) {}

  fn unnamed_at_rule(&self, token: &Token) -> Result<(), CssSyntaxError> {
    let start = token_start(token).unwrap_or(0);
    let end = token_after(token).unwrap_or(start);
    Err(self.input.error(
      "At-rule without name",
      self.get_position(start),
      Some(self.get_position(end)),
    ))
  }

  fn unexpected_close(&self, token: &Token) -> Result<(), CssSyntaxError> {
    let start = token_start(token).unwrap_or(0);
    let end = token_after(token).unwrap_or(start);
    Err(self.input.error(
      "Unexpected }",
      self.get_position(start),
      Some(self.get_position(end)),
    ))
  }

  fn unknown_word(&self, tokens: &[Token]) -> Result<(), CssSyntaxError> {
    if let Some(token) = tokens.first() {
      let start = token_start(token).unwrap_or(0);
      let end = token_after(token).unwrap_or(start + token.value.len());
      Err(self.input.error(
        "Unknown word",
        self.get_position(start),
        Some(self.get_position(end)),
      ))
    } else {
      Err(self.input.error("Unknown word", self.get_position(0), None))
    }
  }

  fn unclosed_block(&self) -> Result<(), CssSyntaxError> {
    let pos = self
      .current
      .borrow()
      .source
      .start
      .clone()
      .unwrap_or_else(|| Position::new(1, 1, 0));
    Err(self.input.error("Unclosed block", pos, None))
  }

  fn unclosed_bracket(&self, token: &Token) -> Result<(), CssSyntaxError> {
    let start = token_start(token).unwrap_or(0);
    let end = token_after(token).unwrap_or(start + token.value.len());
    Err(self.input.error(
      "Unclosed bracket",
      self.get_position(start),
      Some(self.get_position(end)),
    ))
  }
}

pub fn parse(css: &str) -> Result<Root, ParseError> {
  parse_with_options(css, ParseOptions::default())
}

pub fn parse_with_options(css: &str, opts: ParseOptions) -> Result<Root, ParseError> {
  let input = Input::new(
    css.to_string(),
    InputOptions {
      from: opts.from.clone(),
      map: opts.map.clone(),
    },
  )?;
  let input_ref = InputRef::from(input);
  let parser = Parser::new(input_ref.clone(), opts.ignore_errors);
  Ok(parser.parse()?)
}
