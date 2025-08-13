use std::path::PathBuf;

use anyhow::anyhow;
use async_trait::async_trait;
use atlaspack_dev_server::DevServer;
use atlaspack_dev_server::DevServerDataProvider;
use atlaspack_dev_server::DevServerOptions;
use atlaspack_napi_helpers::anyhow_to_napi;
use atlaspack_napi_helpers::js_callable::JsCallable;
use napi::bindgen_prelude::Array;
use napi::bindgen_prelude::External;
use napi::bindgen_prelude::FromNapiValue;
use napi::bindgen_prelude::Object;
use napi::JsObject;
use napi::JsString;
use napi_derive::napi;

#[derive(Debug)]
pub struct JsDevServerDataProvider {
  get_html_bundle_file_paths: JsCallable,
  request_bundle: JsCallable,
}

impl JsDevServerDataProvider {
  pub fn new(data_provider: JsObject) -> napi::Result<Self> {
    let get_html_bundle_file_paths =
      JsCallable::new_method_bound("getHTMLBundleFilePaths", &data_provider)?;
    let request_bundle = JsCallable::new_method_bound("requestBundle", &data_provider)?;

    Ok(Self {
      get_html_bundle_file_paths,
      request_bundle,
    })
  }
}

#[async_trait]
impl DevServerDataProvider for JsDevServerDataProvider {
  async fn get_html_bundle_file_paths(&self) -> anyhow::Result<Vec<String>> {
    let result = self
      .get_html_bundle_file_paths
      .call(
        |_| Ok(vec![]),
        |_env, value| {
          let array = Array::from_unknown(value)?;

          let mut strings = vec![];
          for i in 0..array.len() {
            let js_string = array
              .get::<JsString>(i)?
              .ok_or_else(|| napi::Error::from_reason("[napi] Expected string"))?
              .coerce_to_string()?;
            let js_string = js_string.into_utf8()?;
            let string = js_string.into_owned()?;
            strings.push(string);
          }

          Ok(strings)
        },
      )
      .await?;

    Ok(result)
  }

  async fn request_bundle(&self, requested_path: String) -> anyhow::Result<()> {
    self
      .request_bundle
      .call(
        move |env| {
          let requested_path = env.create_string_from_std(requested_path)?;
          let requested_path = requested_path.into_unknown();

          Ok(vec![requested_path])
        },
        |_env, value| {
          let js_string = value.coerce_to_string()?;
          let js_string = js_string.into_utf8()?;
          let string = js_string.into_owned()?;

          Ok(string)
        },
      )
      .await?;

    Ok(())
  }
}

#[napi(object)]
pub struct JsDevServerOptions {
  pub host: String,
  pub port: u16,
  pub public_url: Option<String>,
  pub dist_dir: String,
}

pub struct JsDevServer {
  dev_server: DevServer,
}

#[napi(object)]
pub struct JsDevServerStartResult {
  pub host: String,
  pub port: u16,
}

impl JsDevServer {
  pub fn new(options: JsDevServerOptions, data_provider: JsObject) -> napi::Result<Self> {
    let data_provider = JsDevServerDataProvider::new(data_provider)?;
    let dev_server_options = DevServerOptions {
      host: options.host,
      port: options.port,
      public_url: options.public_url,
      dist_dir: PathBuf::from(options.dist_dir),
      data_provider: Box::new(data_provider),
    };

    let dev_server = DevServer::new(dev_server_options);

    Ok(JsDevServer { dev_server })
  }

  pub async fn start(&self) -> napi::Result<JsDevServerStartResult> {
    let addr = self
      .dev_server
      .start()
      .await
      .map_err(|err| anyhow!("Failed to start dev server: {}", err))
      .map_err(anyhow_to_napi)?;

    Ok(JsDevServerStartResult {
      host: addr.ip().to_string(),
      port: addr.port(),
    })
  }

  pub async fn stop(&self) -> napi::Result<()> {
    self
      .dev_server
      .stop()
      .await
      .map_err(|err| anyhow!("Failed to stop dev server: {}", err))
      .map_err(anyhow_to_napi)?;

    Ok(())
  }
}

/// Create a new atlaspack dev-server instance.
#[napi]
pub fn atlaspack_dev_server_create(
  options: JsDevServerOptions,
  data_provider: Object,
) -> napi::Result<External<JsDevServer>> {
  let server = JsDevServer::new(options, data_provider)?;
  Ok(External::new(server))
}

#[napi]
pub async fn atlaspack_dev_server_start(
  server: External<JsDevServer>,
) -> napi::Result<JsDevServerStartResult> {
  let server = server.as_ref();
  let result = server
    .start()
    .await
    .map_err(|err| anyhow!("Failed to start dev server: {}", err))
    .map_err(anyhow_to_napi)?;

  Ok(result)
}

#[napi]
pub async fn atlaspack_dev_server_stop(server: External<JsDevServer>) -> napi::Result<()> {
  let server = server.as_ref();
  server
    .stop()
    .await
    .map_err(|err| anyhow!("Failed to stop dev server: {}", err))
    .map_err(anyhow_to_napi)?;

  Ok(())
}
