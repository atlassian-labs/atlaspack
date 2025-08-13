import {Reporter} from '@atlaspack/plugin';
import HMRServer from './HMRServer';
import Server from './Server';
import {getFeatureFlag} from '@atlaspack/feature-flags';
import {
  ReporterEvent,
  PluginOptions,
  PluginLogger,
  PluginTracer,
  ServerOptions,
} from '@atlaspack/types';
import {
  atlaspackDevServerCreate,
  atlaspackDevServerStart,
  atlaspackDevServerStop,
  JsDevServer,
} from '@atlaspack/rust';
import {StaticServerDataProvider} from './StaticServerDataProvider';

class DevServerReporter {
  private dataProvider: StaticServerDataProvider | null = null;
  private serverInstances: Map<string, JsDevServer> = new Map();

  constructor() {}

  async report({
    event,
    options,
  }: {
    event: ReporterEvent;
    options: PluginOptions;
    logger: PluginLogger;
    tracer: PluginTracer;
  }) {
    if (event.type === 'log') {
      return;
    }

    const {serveOptions} = options;
    if (!serveOptions || !serveOptions.port) {
      return;
    }

    await this.getServer(serveOptions);

    switch (event.type) {
      case 'watchStart': {
        break;
      }
      case 'watchEnd': {
        const server = await this.getServer(serveOptions);
        atlaspackDevServerStop(server);
        this.serverInstances.delete(serveOptions.port.toString());
        break;
      }
      case 'buildStart': {
        this.getDataProvider(serveOptions).onBuildStart();
        break;
      }
      case 'buildProgress': {
        break;
      }
      case 'buildSuccess': {
        this.getDataProvider(serveOptions).onBuildSuccess(
          event.bundleGraph,
          event.requestBundle,
        );
        break;
      }
      case 'buildFailure': {
        await this.getDataProvider(serveOptions).onBuildFailure(
          options,
          event.diagnostics,
        );
        break;
      }
    }
  }

  async getServer(serveOptions: ServerOptions) {
    const existing = this.serverInstances.get(serveOptions.port.toString());
    if (existing) {
      return existing;
    }

    const server = atlaspackDevServerCreate(
      {
        port: serveOptions.port,
        host: 'localhost',
        distDir: serveOptions.distDir,
        publicUrl: serveOptions.publicUrl ?? '/',
      },
      this.getDataProvider(serveOptions),
    );

    await atlaspackDevServerStart(server);

    this.serverInstances.set(serveOptions.port.toString(), server);

    return server;
  }

  getDataProvider(serveOptions: ServerOptions): StaticServerDataProvider {
    if (this.dataProvider) {
      return this.dataProvider;
    }

    this.dataProvider = new StaticServerDataProvider(serveOptions.distDir);

    return this.dataProvider;
  }
}

const reporter = new DevServerReporter();

let servers: Map<number, Server> = new Map();
let hmrServers: Map<number, HMRServer> = new Map();

export default new Reporter({
  async report(params) {
    if (getFeatureFlag('rustDevServer')) {
      await reporter.report(params);
      return;
    }

    const {event, options, logger} = params;

    let {serveOptions, hmrOptions} = options;
    let server = serveOptions ? servers.get(serveOptions.port) : undefined;
    let hmrPort =
      (hmrOptions && hmrOptions.port) || (serveOptions && serveOptions.port);
    let hmrServer = hmrPort ? hmrServers.get(hmrPort) : undefined;
    switch (event.type) {
      case 'watchStart': {
        if (serveOptions) {
          // If there's already a server when watching has just started, something
          // is wrong.
          if (server) {
            return logger.warn({
              message: 'Trying to create the devserver but it already exists.',
            });
          }

          let serverOptions = {
            ...serveOptions,
            projectRoot: options.projectRoot,
            cacheDir: options.cacheDir,
            // Override the target's publicUrl as that is likely meant for production.
            // This could be configurable in the future.
            publicUrl: serveOptions.publicUrl ?? '/',
            inputFS: options.inputFS,
            outputFS: options.outputFS,
            packageManager: options.packageManager,
            logger,
            hmrOptions,
          };

          server = new Server(serverOptions);
          servers.set(serveOptions.port, server);
          const devServer = await server.start();

          if (hmrOptions && hmrOptions.port === serveOptions.port) {
            let hmrServerOptions = {
              port: serveOptions.port,
              host: hmrOptions.host,
              devServer,
              // @ts-expect-error TS7006
              addMiddleware: (handler) => {
                server?.middleware.push(handler);
              },
              logger,
              https: options.serveOptions ? options.serveOptions.https : false,
              cacheDir: options.cacheDir,
              inputFS: options.inputFS,
              outputFS: options.outputFS,
            };
            hmrServer = new HMRServer(hmrServerOptions);
            hmrServers.set(serveOptions.port, hmrServer);
            await hmrServer.start();
            return;
          }
        }

        let port = hmrOptions?.port;
        if (typeof port === 'number') {
          let hmrServerOptions = {
            port,
            host: hmrOptions?.host,
            logger,
            https: options.serveOptions ? options.serveOptions.https : false,
            cacheDir: options.cacheDir,
            inputFS: options.inputFS,
            outputFS: options.outputFS,
          };
          hmrServer = new HMRServer(hmrServerOptions);
          hmrServers.set(port, hmrServer);
          await hmrServer.start();
        }
        break;
      }
      case 'watchEnd':
        if (serveOptions) {
          if (!server) {
            return logger.warn({
              message:
                'Could not shutdown devserver because it does not exist.',
            });
          }
          await server.stop();
          servers.delete(server.options.port);
        }
        if (hmrOptions && hmrServer) {
          await hmrServer.stop();
          hmrServers.delete(hmrServer.wss.options.port);
        }
        break;
      case 'buildStart':
        if (server) {
          server.buildStart();
        }
        break;
      case 'buildProgress':
        if (
          event.phase === 'bundled' &&
          hmrServer &&
          // Only send HMR updates before packaging if the built in dev server is used to ensure that
          // no stale bundles are served. Otherwise emit it for 'buildSuccess'.
          options.serveOptions !== false
        ) {
          await hmrServer.emitUpdate(event);
        }
        break;
      case 'buildSuccess':
        if (serveOptions) {
          if (!server) {
            return logger.warn({
              message:
                'Could not send success event to devserver because it does not exist.',
            });
          }

          server.buildSuccess(event.bundleGraph, event.requestBundle);
        }
        if (hmrServer && options.serveOptions === false) {
          await hmrServer.emitUpdate(event);
        }
        break;
      case 'buildFailure':
        // On buildFailure watchStart sometimes has not been called yet
        // do not throw an additional warning here
        if (server) {
          await server.buildError(options, event.diagnostics);
        }
        if (hmrServer) {
          await hmrServer.emitError(options, event.diagnostics);
        }
        break;
    }
  },
}) as Reporter;
