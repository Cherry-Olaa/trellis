import { http, HttpResponse } from 'msw'
import { RPC_URL } from '../../lib/config'

/**
 * Mock handlers for Soroban RPC endpoints used in tests.
 * Provides deterministic responses for getEvents and getHealth queries.
 */

/** Shape of the JSON-RPC request bodies the Soroban RPC endpoint receives. */
interface RpcRequestBody {
  jsonrpc: string
  id: number | string
  method: string
  params?: unknown
}

export const handlers = [
  // Mock getEvents RPC endpoint
  http.post(RPC_URL, async ({ request }) => {
    const body = (await request.json()) as RpcRequestBody

    if (body.method === 'getEvents') {
      return HttpResponse.json({
        jsonrpc: '2.0',
        id: body.id,
        result: {
          events: [
            {
              type: 'contract',
              ledger: 1000,
              ledgerClosedAt: '2024-01-01T00:00:00Z',
              contractId: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC',
              id: '0001',
              pagingToken: '0001',
              topic: ['AAAADwAAAAdjcmVhdGVk'],
              value: {
                xdr: 'AAAABAAAAAEAAAACAAAADwAAAAdhY2NvdW50AAAAAAAA',
              },
              inSuccessfulContractCall: true,
              txHash: 'abc123',
            },
            {
              type: 'contract',
              ledger: 1001,
              ledgerClosedAt: '2024-01-01T00:01:00Z',
              contractId: 'CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC',
              id: '0002',
              pagingToken: '0002',
              topic: ['AAAADwAAAAZsb2NrZWQ='],
              value: {
                xdr: 'AAAABAAAAAEAAAACAAAADwAAAAdhY2NvdW50AAAAAAAA',
              },
              inSuccessfulContractCall: true,
              txHash: 'def456',
            },
          ],
          latestLedger: 1001,
        },
      })
    }

    if (body.method === 'getHealth') {
      return HttpResponse.json({
        jsonrpc: '2.0',
        id: body.id,
        result: {
          status: 'healthy',
          latestLedger: 1001,
          oldestLedger: 1,
          ledgerRetentionWindow: 17280,
        },
      })
    }

    // Default fallback for unknown methods
    return HttpResponse.json({
      jsonrpc: '2.0',
      id: body.id,
      error: {
        code: -32601,
        message: 'Method not found',
      },
    })
  }),
]
