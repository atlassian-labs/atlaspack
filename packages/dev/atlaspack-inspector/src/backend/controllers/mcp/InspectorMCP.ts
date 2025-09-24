/* eslint-disable monorepo/no-internal-import */
import {
  McpServer,
  ResourceTemplate,
} from '@modelcontextprotocol/sdk/server/mcp.js';
import {
  isInitializeRequest,
  ListResourcesResult,
  ReadResourceResult,
  Resource,
} from '@modelcontextprotocol/sdk/types.js';
import {StreamableHTTPServerTransport} from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import {randomUUID} from 'crypto';
import {Request, Response} from 'express';
import {z} from 'zod';
import {LMDBLiteCache} from '@atlaspack/cache';
import {
  getBundleGraph,
  getCache,
  getTreemap,
} from '../../config/middleware/cacheDataMiddleware';
// @ts-expect-error TS2749
import BundleGraph from '@atlaspack/core/lib/BundleGraph.js';
import {Treemap} from '../../services/buildTreemap';

/**
 * Incomplete, toy MCP server for the Atlaspack Inspector.
 */
function makeInspectorMCPServer({
  cache,
  bundleGraph,
  treemap,
}: {
  cache: LMDBLiteCache;
  bundleGraph: BundleGraph;
  treemap: Treemap;
}): McpServer {
  const server = new McpServer({
    name: 'atlaspack-inspector-mcp',
    version: '1.0.0',
  });

  server.registerTool(
    'list-treemap-bundles',
    {
      title: 'List Treemap Bundles',
      description: `List the bundles in the treemap.`,
      inputSchema: {},
    },
    () => {
      return {
        content: [
          {
            type: 'text',
            text: [
              `Treemap bundles:`,
              ...treemap.bundles.flatMap((bundle) => [
                `- ${bundle.displayName} - ${bundle.id} - atlaspack://bundle-info/${bundle.id}`,
                `  * ${bundle.size} bytes`,
                `  * ${bundle.filePath} file path`,
                `  * Run 'query-treemap ${bundle.id}' to get more information about this bundle`,
              ]),
            ].join('\n'),
          },
        ],
      };
    },
  );

  server.registerTool(
    'query-treemap',
    {
      title: 'Query Treemap',
      description: `Query the treemap for information about a given bundle.`,
      inputSchema: {
        bundleId: z.string(),
      },
    },
    ({bundleId}) => {
      const bundle = treemap.bundles.find((bundle) => bundle.id === bundleId);

      if (!bundle) {
        return {
          content: [{type: 'text', text: `Bundle not found: ${bundleId}`}],
        };
      }

      return {
        content: [
          {
            type: 'text',
            text: [
              `Bundle: ${bundle.displayName}`,
              `Bundle size: ${bundle.size} bytes`,
            ].join('\n'),
          },
          {
            type: 'text',
            text: `Raw treemap JSON:\n${JSON.stringify(bundle)}`,
          },
        ],
      };
    },
  );

  server.registerTool(
    'get-resolved-imports',
    {
      title: 'Get Resolved Imports',
      description: `Get the resolved imports for a given JavaScript/TypeScript file, if it appears on the output bundles.`,
      inputSchema: {
        file: z.string(),
      },
    },
    ({file}) => {
      const assetId = bundleGraph._graph.nodes.find((node: any) => {
        return node.type === 'asset' && node.value.filePath === file;
      })?.id;

      if (!assetId) {
        return {
          content: [
            {type: 'text', text: `File not found in any bundles: ${file}`},
          ],
        };
      }

      const asset = bundleGraph.getAssetById(assetId);
      const bundlesWithAsset = bundleGraph.getBundlesWithAsset(asset);

      return {
        content: [
          {
            type: 'text',
            text: [
              `Bundles with asset:`,
              ...bundlesWithAsset.map(
                (bundle: any) =>
                  `- ${bundle.id} - ${bundle.displayName} - atlaspack://bundle-info/${bundle.id}`,
              ),
            ].join('\n'),
          },
          {
            type: 'text',
            text: `Raw asset JSON: ${JSON.stringify(asset)}`,
          },
        ],
      };
    },
  );

  server.registerResource(
    'treemap-bundle-info',
    new ResourceTemplate('atlaspack://bundle-info/{id}', {
      list(): ListResourcesResult {
        const resources: Resource[] = treemap.bundles
          .slice(0, 10)
          .map((bundle) => ({
            uri: `atlaspack://bundle-info/${bundle.id}`,
            name: `Bundle: ${bundle.displayName}`,
          }));

        return {
          resources,
        };
      },
    }),
    {
      name: 'Atlaspack Treemap Bundle Information',
      description: `Information about a built JavaScript bundle in the application and its bundle size. Can be used to list available bundles in the application.`,
    },
    (uri: URL): ReadResourceResult => {
      const id = uri.pathname.split('/').slice(1).join('/');
      const bundle = treemap.bundles.find((bundle) => bundle.id === id);

      if (!bundle) {
        return {
          contents: [],
        };
      }

      return {
        contents: [
          {
            uri: uri.toString(),
            text: [
              `Bundle: ${bundle.displayName}`,
              `Bundle size: ${bundle.size} bytes`,
            ].join('\n---\n'),
          },
        ],
      };
    },
  );

  server.registerResource(
    'cache-key',
    new ResourceTemplate('atlaspack://cache/{key}', {
      list: undefined,
    }),
    {
      name: 'Atlaspack Cache Entry',
    },
    (uri: URL): ReadResourceResult => {
      const key = uri.pathname.split('/').slice(1).join('/');
      const value = cache.getBlobSync(key);

      return {
        contents: [
          {
            uri: uri.toString(),
            type: 'text',
            text: [
              `Cache key: ${key}`,
              `Cache value total size: ${value.length} bytes`,
              `First 500KB of cache value:\n\n${value.subarray(0, 1024 * 500).toString()}`,
            ].join('\n---\n'),
          },
        ],
      };
    },
  );

  return server;
}

export interface InspectorMCPSession {
  transport: StreamableHTTPServerTransport;
  server: McpServer;
}

export class InspectorMCP {
  private sessions: Map<string, InspectorMCPSession> = new Map();

  constructor() {}

  getSession(request: Request): InspectorMCPSession | undefined {
    const sessionId = request.headers['mcp-session-id'] as string | undefined;
    if (!sessionId) {
      return undefined;
    }
    return this.sessions.get(sessionId);
  }

  async get(request: Request, response: Response): Promise<void> {
    const session = this.getSession(request);
    if (!session) {
      response.status(400).send('Invalid or missing session ID');
      return;
    }

    await session.transport.handleRequest(request, response);
  }

  async post(request: Request, response: Response): Promise<void> {
    const sessionId = request.headers['mcp-session-id'] as string | undefined;
    let transport: StreamableHTTPServerTransport;

    if (sessionId && this.sessions.has(sessionId)) {
      transport = this.sessions.get(sessionId)!.transport;
    } else if (!sessionId && isInitializeRequest(request.body)) {
      transport = await this.createSession(response);
    } else {
      response.status(400).json({
        jsonrpc: '2.0',
        error: {
          code: -32000,
          message: 'Bad Request: No valid session ID provided',
        },
        id: null,
      });
      return;
    }

    await transport.handleRequest(request, response, request.body);
  }

  private async createSession(
    response: Response,
  ): Promise<StreamableHTTPServerTransport> {
    const cache = getCache(response);
    const bundleGraph = getBundleGraph(response);
    const treemap = getTreemap(response);

    const server = makeInspectorMCPServer({
      cache,
      bundleGraph,
      treemap,
    });

    const transport = new StreamableHTTPServerTransport({
      sessionIdGenerator: () => randomUUID(),
      onsessioninitialized: (sessionId) => {
        this.sessions.set(sessionId, {
          transport,
          server,
        });
      },
    });

    transport.onclose = () => {
      if (transport.sessionId) {
        this.sessions.delete(transport.sessionId);
      }
    };

    await server.connect(transport);

    return transport;
  }
}
