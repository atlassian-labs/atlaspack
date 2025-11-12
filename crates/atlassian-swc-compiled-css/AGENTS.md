- This project (compiled platform/crates/compiled) exists in a large monorepo of other projects.
- Never install or change dependencies in Cargo.toml or anyone else's Cargo.toml
- This plugin is a replacement for compiled/packages/babel-plugin AND compiled/packages/babel-plugin-strip-runtime as new singular plugin built in Rust/SWC native transformer (NO WASM EVER)
- extract: true mimics both babel plugins combined (no CC/CS/variables emitted) style rules are emitted as a data structure in Rust not as part of the code
- extract: false mimics JUST packages/babel-plugin where CC/CS and varialbes will be emitted) style rules are emitted as variables as part of the JS source code in this case.
- Hashes MUST Be the same between plugins and same with the 'value' portion of style rules. Order is not important. Hash implementation is already identical between Rust/Babel (We think) differences are normally caused by the fact that babel-plugin runs the input through a PostCSS pipeline before inputting it in to be hashed. We have to replicate this without installing dependencies and you may come across difference in hashes caused by PostCSS normalisation differences.
- Babel is the source of truth here, and our behaviour should match for everything that isn't a cosmetic difference see guidelines below about how we classify differences.
- We have severael css-in-js API's css({..}) css`color: red;` cssMap (which is for variants), keyframes({..}) and finally styled.div({color: red}) or styled.div`color: red`; the supports complex css expressions and nesting, pseudo selectors, arbitary nesting.
- We also have several behaviours around css attributes, xcss attributes and others, these behaviours need to match babel-plugin
- Invesitgate packages/babel-plugin if you're ever confused about what the behaviour SHOULD be.
- token('...') syntax is handled by a seperate plugin (platform/crates/swc-design-system-tokens) which inlines a css variable via another SWC/Rust native transformer.


## Differences we don't care about between the babel-plugin AND Rust/SWC transformer rewrite
- Import order is just cosmetic, not behavioural
- style-rule ordering is cosmetic, not behavioural
- JSX preserved <CC> vs not preserved createElement(CC) or jsx(CC) is cosmetic not behavioural
- There is a 'special' case with rules containing like translate(5px,5px) where the 'value' portion of the hash (hashes are structured like this <selector hash><value hash>) can change based on abitary white space; this is incorrect and a bug in the babel-plugin. we don't want to replicate that bug in our new plugin (as then this rule wouldn't atomically deduplicate when white-space is different; Therefore differences in either hash or value [aslong as it's whitespace]) for rules using translate can be ignored. HOWEVER If the difference is in the <selector hash> portion then this IS A DIFFERENCE WE CARE ABOUT AS IT WILL BREAK DOWNSTREAM BEHAVIOUR.

## Differences we do care about between the babel-plugin AND Rust/SWC transformer rewrite
- Hashes must be identical this is behvioural
- Values must be identical this is beahvioural
- Number of rules generated must be identical this is behavioural
- JSX output must be the same (but preserved or not doesn't matter as said above) I.E if an element is wrapped in <CC> or <CS> tags in babel output it must also be wrapped in swc output.

## Guidelines for fixture tests
- in.jsx is the input file of a fixture. actual.js is the result of running our transformer over in.jsx. out.js is what we're expecting. babel-out.js is just a cosmetic file that shows us the babel-plugins output for reference for comparison.
- actual.js can be regenerated using cargo run --example generate_fixture <fixture_name>

