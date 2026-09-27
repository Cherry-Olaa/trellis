# Trellis Frontend

React + TypeScript + Vite single-page app for Trellis, a Soroban escrow contract for milestone-based agreements on Stellar. The frontend lets payers and payees create agreements, lock funds, submit work proofs, and manage disputes through a browser wallet (Freighter).

## Architecture

- **React 19 + React Router** for the SPA, with client-side routing (see [DEPLOYMENT.md](../DEPLOYMENT.md#frontend-spa-routing) for host routing fallback config).
- **`@stellar/stellar-sdk`** to build and submit Soroban transactions against the RPC endpoint.
- **`@stellar/freighter-api`** to connect the user's Freighter wallet and request signatures.
- Source layout under `src/`: `pages/` for routes, `components/` for shared UI, `hooks/` and `context/` for wallet/contract state, `lib/` for Stellar SDK helpers.

## Environment variables

Copy `.env.example` to `.env` and set:

| Variable | Description |
|---|---|
| `VITE_CONTRACT_ID` | Deployed Trellis contract ID (see [DEPLOYMENT.md](../DEPLOYMENT.md) for the current testnet contract). |
| `VITE_RPC_URL` | Soroban RPC endpoint, e.g. `https://soroban-testnet.stellar.org`. |
| `VITE_NETWORK_PASSPHRASE` | Network passphrase, e.g. `Test SDF Network ; September 2015`. |

## Development

Install dependencies and start the dev server:

```bash
npm install
npm run dev
```

The app runs at `http://localhost:5173` by default. Freighter must be installed in your browser and set to the same network as `VITE_NETWORK_PASSPHRASE`.

### Connecting to testnet

1. Set the env vars above to point at Stellar testnet (defaults in `.env.example` already do).
2. Install [Freighter](https://www.freighter.app/) and switch it to Testnet.
3. Fund a testnet account via [Friendbot](https://developers.stellar.org/docs/tools/quickstart#friendbot) if you need test XLM.

### Available scripts

| Script | Description |
|---|---|
| `npm run dev` | Start the Vite dev server with HMR. |
| `npm run build` | Type-check (`tsc -b`) and build for production into `dist/`. |
| `npm run preview` | Preview the production build locally. |
| `npm run lint` | Run Oxlint. |
| `npm run typecheck` | Run TypeScript in check-only mode. |
| `npm test` | Run the Vitest suite once. |
| `npm run test:watch` | Run Vitest in watch mode. |
| `npm run test:coverage` | Run Vitest with coverage report. |

### Building for production

```bash
npm run build
```

Output is written to `dist/`. Serve it with any static host; see [DEPLOYMENT.md](../DEPLOYMENT.md#frontend-spa-routing) for SPA routing fallback configuration required on nginx/Apache/Netlify/Vercel.

### Lint rules

Oxlint is configured in [`.oxlintrc.json`](./.oxlintrc.json). The `typescript/no-explicit-any` rule is enabled as a **warning** (not an error) so a type-safety regression cannot be merged silently, while leaving room to migrate any remaining untyped code.

Migration path when a warning appears:

1. **Model the shape.** For RPC responses, declare an `interface` with the fields actually read — see `src/test/mocks/handlers.ts` for the `RpcRequestBody` pattern.
2. **Prefer `unknown` over `any`** for values you only pass through or narrow later; narrow with a type guard or `instanceof` rather than a cast.
3. **When a type is genuinely external and unmodelled**, use a narrowly-scoped intersection type instead of widening the whole value — see `createAbortSignalFallback` in `src/lib/abort.ts` for the `AbortSignal & { _timeoutId?: ... }` pattern.
4. **Do not silence the rule with an inline disable** unless the case is unavoidable and commented with why. Once the warning count reaches zero, the rule should be promoted to `error`.

`src/` currently has no `any` usages, so a new one is always a regression rather than pre-existing debt.


## Contributing

See the root [CONTRIBUTING.md](../CONTRIBUTING.md) for repository-wide setup and PR guidelines. For Stellar/Freighter API details, see the [Stellar developer docs](https://developers.stellar.org/) and [Freighter docs](https://docs.freighter.app/).
