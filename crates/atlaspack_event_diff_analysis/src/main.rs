use serde::{Deserialize, Serialize};
use std::fs::File;

fn main() {
  // there's a CSV file
  // id,parentId,category,name,action,source,product,user,environment,repository,startTimestamp,value,status,git,build,runtime,system,attributes,featureFlags,day,year,month,user_division,user_department
  // we want to read the file and print the number of rows
  let file = File::open("/Users/pyamada/Downloads/VCS_event_diff.csv").unwrap();
  let mut reader = csv::Reader::from_reader(file);

  let ignored_paths = vec![
    "/.afm-cache",
    "/.bazel",
    "/.afm",
    "/.bitbucket",
    "/.git-hooks",
    "/.atlaspack",
    "/infra-feature-flags-cache.json",
    "/jira-ssr/build/",
    "/routes-manifest.json",
    "/.almd.git-telemetry.json",
    "/.editorconfig",
    "/.git-blame-ignore-revs",
    "/.vscode",
    "/.prebuilt",
    "/facts-map-output.log",
    "/.idea",
    "/node_modules/.cache",
    "/.pillar-cache",
    "/.DS_Store",
  ];

  // let mut events = Vec::new();
  let mut total = 0;
  let mut skipped = 0;

  for record in reader.records() {
    let record = record.unwrap();
    // println!("{:?}", record);
    // events.push(record);
    // get the attributes JSON value

    let attributes = record.get(17).unwrap();
    let attributes: serde_json::Value = serde_json::from_str(attributes).unwrap();
    // println!("{:?}", attributes);

    // get the "vcs" key
    let vcs = attributes.get("vcs").unwrap().clone();
    let vcs = vcs.as_str().unwrap();
    let vcs: serde_json::Value = serde_json::from_str(vcs).unwrap();

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Vcs {
      watcher_event_count: u32,
      vcs_event_count: u32,
      diff_count: u32,
      top_events_diff: Vec<String>,
    }

    let mut vcs: Vcs = serde_json::from_value(vcs).unwrap();
    vcs.top_events_diff = vcs
      .top_events_diff
      .into_iter()
      .filter(|event| {
        // sample event == "/Users/redacted/atlassian/atlassian-frontend-monorepo/.afm-cache",
        !ignored_paths.iter().any(|path| event.contains(path))
      })
      .collect();

    total += 1;
    if vcs.top_events_diff.len() > 0 {
      println!("{:#?}", vcs);
    } else {
      skipped += 1;
    }
  }

  println!("total: {}", total);
  println!("skipped: {}", skipped);
}
