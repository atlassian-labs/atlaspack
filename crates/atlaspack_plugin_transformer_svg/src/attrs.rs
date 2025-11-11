use html5ever::tendril::fmt::UTF8;
use markup5ever::tendril::Tendril;
use markup5ever::{Attribute, ExpandedName, QualName};

pub struct Attrs<'a> {
  attributes: &'a mut Vec<Attribute>,
}

impl<'a> Attrs<'a> {
  pub fn new(attributes: &'a mut Vec<Attribute>) -> Self {
    Self { attributes }
  }

  pub fn get(&self, name: ExpandedName) -> Option<&Tendril<UTF8>> {
    self
      .attributes
      .iter()
      .find(|attr| attr.name.expanded() == name)
      .map(|attr| &attr.value)
  }

  pub fn delete(&mut self, name: ExpandedName) {
    *self.attributes = self
      .attributes
      .iter()
      .filter(|attr| attr.name.expanded() != name)
      .cloned()
      .collect();
  }

  pub fn set(&mut self, name: ExpandedName, value: &str) {
    if let Some(attribute) = self
      .attributes
      .iter_mut()
      .find(|attr| attr.name.expanded() == name)
    {
      attribute.value = value.into();
    } else {
      self.attributes.push(Attribute {
        name: QualName::new(None, name.ns.clone(), name.local.clone()),
        value: value.into(),
      });
    }
  }
}
