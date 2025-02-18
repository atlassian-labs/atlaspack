use std::path::PathBuf;

use atlaspack_core::types::Entry;

use crate::compilation::Compilation;

pub async fn build_entry_dependencies(
  Compilation {
    cache,
    options,
    project_root,
    fs,
    entries,
    ..
  }: &mut Compilation,
) -> anyhow::Result<()> {
  for entry_path_str in options.entries.iter() {
    let entry = cache
      .get_or_init(entry_path_str, || async {
        // TODO: Handle globs and directories
        let mut entry_path = PathBuf::from(entry_path_str);
        if entry_path.is_relative() {
          entry_path = project_root.join(entry_path);
        };

        if fs.is_file(&entry_path) {
          return Ok(Some(Entry {
            file_path: entry_path,
            target: None,
          }));
        }
        Ok(None)
      })
      .await?;

    match entry {
      Some(entry) => entries.push(entry),
      None => anyhow::bail!("Unknown entry: {}", entry_path_str),
    }
  }

  Ok(())
}
