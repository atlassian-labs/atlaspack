pushd flow-to-typescript-codemod
yarn
yarn typescriptify convert --autoSuppressErrors --write --delete -p ../packages --ignore dist --ignore integration-tests
popd
