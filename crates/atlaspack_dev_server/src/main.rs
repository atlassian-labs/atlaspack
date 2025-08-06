use atlaspack_dev_server::start_dev_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Starting Atlaspack Dev Server...");

  // Start the dev server on port 3000
  start_dev_server(Some(3000)).await?;

  Ok(())
}
