use std::collections::HashMap;
use std::ops::Add;
use std::ops::Div;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use std::time::SystemTime;

use parking_lot::RwLock;

static PROFILES: LazyLock<Arc<RwLock<HashMap<String, Vec<SystemTime>>>>> =
  LazyLock::new(|| Default::default());

pub fn time(name: impl AsRef<str>) {
  PROFILES
    .write()
    .entry(name.as_ref().to_string())
    .or_default()
    .push(SystemTime::now());
}

pub fn lap(name: impl AsRef<str>) {
  time(name);
}

pub fn time_end(name: impl AsRef<str>) {
  time(&name);
  report(name).log_total();
}

pub fn report(name: impl AsRef<str>) -> ProfileReport {
  ProfileReport(
    PROFILES
      .write()
      .entry(name.as_ref().to_string())
      .or_default()
      .clone(),
    name.as_ref().to_string(),
  )
}

pub struct ProfileReport(Vec<SystemTime>, String);

impl ProfileReport {
  pub fn count(&self) -> usize {
    self.0.len()
  }

  pub fn median(&self) -> Duration {
    let durations = self.durations();
    let mid = durations.len() / 2;
    match durations.get(mid) {
      Some(d) => d.clone(),
      None => Default::default(),
    }
  }

  pub fn average(&self) -> Duration {
    let mut durations = self.durations();
    let mut sum = durations.get(0).unwrap().clone();
    for duration in durations.drain(1..) {
      sum = sum.add(duration);
    }
    sum.div(self.count() as u32)
  }

  pub fn max(&self) -> Duration {
    self.durations().into_iter().max().unwrap()
  }

  pub fn min(&self) -> Duration {
    self.durations().into_iter().min().unwrap()
  }

  pub fn total(&self) -> Duration {
    let mut durations = self.durations();
    let mut sum = durations.get(0).unwrap().clone();
    for duration in durations.drain(1..) {
      sum = sum.add(duration);
    }
    sum
  }

  pub fn log_median(&self) {
    println!(
      "{} ({}) [median]:  {:?}",
      self.1,
      self.count(),
      self.median()
    );
  }

  pub fn log_average(&self) {
    println!(
      "{} ({}) [average]: {:?}",
      self.1,
      self.count(),
      self.average()
    );
  }

  pub fn log_max(&self) {
    println!("{} ({}) [max]:     {:?}", self.1, self.count(), self.max());
  }

  pub fn log_min(&self) {
    println!("{} ({}) [min]:     {:?}", self.1, self.count(), self.min());
  }

  pub fn log_total(&self) {
    println!(
      "{} ({}) [total]:   {:?}",
      self.1,
      self.count(),
      self.total()
    );
  }

  fn durations(&self) -> Vec<Duration> {
    let mut last = None::<&SystemTime>;
    let mut results = vec![];

    for time in self.0.iter() {
      if let Some(last) = last {
        results.push(time.duration_since(*last).unwrap());
      };
      last = Some(time);
    }

    results
  }
}
