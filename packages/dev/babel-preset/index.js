module.exports = (api) => {
  let name = api.caller((caller) => caller && caller.name);
  if (name === 'parcel') {
    return {
      presets: [require('@babel/preset-flow')],
      plugins: [],
    };
  }

  return {
    presets: [
      [
        require('@babel/preset-env'),
        {
          modules: false,
          targets: {
            node: 16,
          },
        },
      ],
      require('@babel/preset-react'),
      require('@babel/preset-flow'),
    ],
    plugins: [
      [
        require('@babel/plugin-transform-modules-commonjs'),
        {
          lazy: true,
        },
      ],
    ],
    env: {
      production: {
        plugins: [],
      },
    },
  };
};
