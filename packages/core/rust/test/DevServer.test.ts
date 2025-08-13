import assert from 'assert';
import {atlaspackDevServerCreate, atlaspackDevServerStart} from '..';

describe('JsDevServer', () => {
  it('should be defined', async () => {
    const dataProvider = {
      getHTMLBundleFilePaths: () => Promise.resolve([]),
      requestBundle: () => Promise.resolve(),
    };

    const devServer = atlaspackDevServerCreate({
      host: 'localhost',
      port: 0,
      distDir: 'dist',
      dataProvider,
    });

    assert.ok(devServer);

    const result = await atlaspackDevServerStart(devServer);
    assert.ok(result);

    assert.ok(result.host);
    assert.ok(result.port);

    const response = await fetch(
      `http://localhost:${result.port}/__atlaspack__/api/health`,
    );
    assert.ok(response.ok);

    const body = await response.text();
    assert.ok(body.includes('ok'));
  });
});
