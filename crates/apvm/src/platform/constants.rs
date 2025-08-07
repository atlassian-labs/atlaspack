pub static LINK_META_FILE: &str = "meta.json";
pub static PACKAGE_META_FILE: &str = "meta.json";

// NPM API
pub static NPM_API_URL: &str = "https://registry.npmjs.org/atlaspack";

// GitHub Releases
/// "RELEASE_URL/v${version}/${RELEASE_TAR}"
pub static RELEASE_URL: &str = "https://github.com/atlassian-labs/atlaspack/releases/download";
pub static GITHUB_URL: &str = "https://github.com/atlassian-labs/atlaspack/archive/";

// "GITHUB_RAW/${commit_hash}/${...filepath}"
pub static GITHUB_RAW: &str = "http://raw.githubusercontent.com/atlassian-labs/atlaspack";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub static RELEASE_NAME: &str = "atlaspack-macos-arm64";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
pub static RELEASE_NAME: &str = "atlaspack-macos-amd64";

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
pub static RELEASE_NAME: &str = "atlaspack-linux-arm64";

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub static RELEASE_NAME: &str = "atlaspack-linux-amd64";

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
pub static RELEASE_NAME: &str = "atlaspack-windows-arm64";

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
pub static RELEASE_NAME: &str = "atlaspack-windows-amd64";

#[cfg(unix)]
pub static NM_BIN_NAME: &str = "atlaspack";

#[cfg(windows)]
pub static NM_BIN_NAME: &str = "atlaspack.exe";
