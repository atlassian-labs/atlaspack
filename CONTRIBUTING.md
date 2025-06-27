# Contributing to Atlaspack

Thank you for considering a contribution to Atlaspack! Unfortunately pull requests, issues and comments are not being accepted outside of Atlassian at this time.

---

## Contribution standards

For pull requests, please:

- Add tests for new features and bug fixes
- Follow the existing style
- Separate unrelated changes into multiple pull requests

You should also add documentation to `docs/` if you are adding new features or adding new API options. Note that our documentation is currently very barebones, so every little bit helps!

<!-- See the existing issues for things to start contributing.

For bigger changes, please make sure you start a discussion first by creating an issue and explaining the intended change. -->

> **If you're an Atlassian employee:** you shouldn't sign the CLA. Instead, please search for the "Link your Github & Atlassian accounts" page on Confluence, and follow the instructions there to link your GitHub account to the Atlassian system.

Atlassian requires contributors to sign a Contributor License Agreement, known as a CLA. This serves as a record stating that the contributor is entitled to contribute the code / documentation / translation to the project and is willing to have it used in distributions and derivative works (or is willing to transfer ownership).

Prior to accepting your contributions we ask that you please follow the appropriate link below to digitally sign the CLA. The Corporate CLA is for those who are contributing as a member of an organization and the individual CLA is for those contributing as an individual.

- [CLA for corporate contributors](https://opensource.atlassian.com/corporate)
- [CLA for individuals](https://opensource.atlassian.com/individual)

## Getting started with development

Optional:

```sh
nvm use
```

These are required:

```sh
yarn
yarn build-native-release
yarn build
```

Then you can try out some of the examples:

```sh
cd packages/examples/kitchen-sink/
yarn start
```

## Releasing new versions

You will need to set-up a Github personal access token with `read:user` and
`read:repo` permissions and set it to the `GITHUB_TOKEN` environment variable.

### Create a changeset with

```
yarn changeset
```

### Update package versions to bump pending changesets

```
yarn changeset version
```

### Publishing to NPM (CI should do this automatically)

```
yarn changeset publish
```
