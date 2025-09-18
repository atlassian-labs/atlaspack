import {Router} from 'express';
import {InspectorMCP} from './InspectorMCP';

export function makeInspectorMCPController(): Router {
  const router = Router();
  const mcp = new InspectorMCP();

  router.post('/api/mcp', (req, res) => {
    mcp.post(req, res);
  });

  router.get('/api/mcp', (req, res) => {
    mcp.get(req, res);
  });

  return router;
}
