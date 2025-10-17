export interface TransformerOpts {
  /**
   * Will import the React namespace if it is missing.
   * When using the `'automatic'` jsx runtime set this to `false`.
   *
   * Defaults to `true`
   */
  importReact?: boolean;

  /**
   * Security nonce that will be applied to inline style elements if defined.
   */
  nonce?: string;

  /**
   * Custom module origins that Compiled should compile when using APIs from.
   */
  importSources?: string[];

  /**
   * Will run additional cssnano plugins to normalize CSS during build.
   *
   * Defaults to `true`
   */
  optimizeCss?: boolean;

  /**
   * List of file extensions to traverse as code
   */
  extensions?: string[];

  /**
   * Add the component name as class name to DOM in non-production environment if styled is used
   *
   * Defults to `false`
   */
  addComponentName?: boolean;

  /**
   * Whether Compiled should process usages of xcss in the codebase.
   * Disable this if xcss is not implemented in your codebase using Compiled's xcss functionality.
   *
   * Defults to `true`
   */
  processXcss?: boolean;

  /**
   * Increases the specificity of all declared Compiled styles.
   * Generally you would only use this for migration purposes when mixing two or more styling
   * solutions.
   *
   * Defults to `false`
   */
  increaseSpecificity?: boolean;

  /**
   * Whether to sort at-rules, including media queries.
   * Defaults to `true`
   */
  sortAtRules?: boolean;

  /**
   * Adds a defined prefix to the generated classes' hashes.
   * Useful in micro frontend environments to avoid clashing/specificity issues.
   *
   * Avoid mixing this with extraction as this may throw an error if combined with extraction
   * or `extract: true` in Webpack loaders or Parcel tranformers.
   */
  classHashPrefix?: string;

  /**
   * Whether to flatten multiple selectors into separate rules to better deduplicate and sort styles.
   *
   * Defults to `true`.
   */
  flattenMultipleSelectors?: boolean;

  extract?: boolean;

  /**
   * Builds in a node environment.
   * Defaults to `false`.
   */
  ssr?: boolean;
}
