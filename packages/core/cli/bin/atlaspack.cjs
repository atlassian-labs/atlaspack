#!/usr/bin/env node

/*
  This will conditionally require the source or dist files
  based on the condition supplied when initializing Atlaspack

  # Will use dist
  npx atlaspack build

  # Will use sources
  env NODE_OPTIONS="conditions=@atlaspack::sources" npx atlaspack build
*/
require("#bin")
