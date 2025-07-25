use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

use lmdb_js_lite::writer::{DatabaseWriter, LMDBOptions};

fn criterion_benchmark(c: &mut Criterion) {
  let input = {
    std::fs::remove_dir_all("benchmark-databases").unwrap();
    std::fs::create_dir_all("benchmark-databases").unwrap();
    DatabaseWriter::new(&LMDBOptions {
      path: "benchmark-databases/test.db".to_string(),
      async_writes: false,
      map_size: Some((1024 * 1024 * 1024) as f64),
    })
    .unwrap()
  };
  let mut txn = input.write_txn().unwrap();
  c.bench_function("inserting entries", |b| {
    b.iter(|| {
      input
        .put(&mut txn, black_box("key"), black_box(&[1, 2, 3, 4, 5]))
        .unwrap();
    })
  });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
