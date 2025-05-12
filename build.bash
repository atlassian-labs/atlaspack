cargo build --release --target x86_64-unknown-linux-gnu --package apvm
rm -rf ./target/x86_64-unknown-linux-gnu/release/atlaspack
ln ./target/x86_64-unknown-linux-gnu/release/apvm ./target/x86_64-unknown-linux-gnu/release/atlaspack
