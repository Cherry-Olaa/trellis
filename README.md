<div align="center">

<!-- Animated SVG Banner with gradient glow -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 240" width="100%" style="max-width:800px;height:auto">
  <defs>
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a0e17" />
      <stop offset="50%" stop-color="#0f1729" />
      <stop offset="100%" stop-color="#0a0e17" />
    </linearGradient>
    <linearGradient id="titleGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#00c2ff" />
      <stop offset="50%" stop-color="#7c3aed" />
      <stop offset="100%" stop-color="#00c2ff" />
      <animate attributeName="x1" values="0%;100%;0%" dur="4s" repeatCount="indefinite" />
      <animate attributeName="x2" values="100%;0%;100%" dur="4s" repeatCount="indefinite" />
    </linearGradient>
    <linearGradient id="accentGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#00c2ff" stop-opacity="0.3" />
      <stop offset="50%" stop-color="#00c2ff" stop-opacity="0.8" />
      <stop offset="100%" stop-color="#00c2ff" stop-opacity="0.3" />
      <animate attributeName="x1" values="0%;100%;0%" dur="3s" repeatCount="indefinite" />
    </linearGradient>
    <filter id="glow">
      <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
      <feMerge>
        <feMergeNode in="coloredBlur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
  </defs>
  <!-- Background -->
  <rect width="800" height="240" rx="20" fill="url(#bgGrad)" stroke="#1e293b" stroke-width="1.5" />
  <!-- Animated grid dots -->
  <g opacity="0.15">
    <circle cx="40" cy="40" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2s" repeatCount="indefinite" begin="0s"/></circle>
    <circle cx="120" cy="60" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.5s" repeatCount="indefinite" begin="0.2s"/></circle>
    <circle cx="200" cy="30" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="3s" repeatCount="indefinite" begin="0.4s"/></circle>
    <circle cx="300" cy="50" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.2s" repeatCount="indefinite" begin="0.6s"/></circle>
    <circle cx="400" cy="35" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.8s" repeatCount="indefinite" begin="0.3s"/></circle>
    <circle cx="500" cy="55" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.1s" repeatCount="indefinite" begin="0.7s"/></circle>
    <circle cx="600" cy="40" r="1.5" fill="#7c3aed"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.6s" repeatCount="indefinite" begin="0.1s"/></circle>
    <circle cx="700" cy="45" r="1.5" fill="#7c3aed"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.3s" repeatCount="indefinite" begin="0.5s"/></circle>
    <circle cx="760" cy="35" r="1.5" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.7s" repeatCount="indefinite" begin="0.9s"/></circle>
    <circle cx="80" cy="120" r="1" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="3.1s" repeatCount="indefinite" begin="0.3s"/></circle>
    <circle cx="160" cy="140" r="1" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.4s" repeatCount="indefinite" begin="0.6s"/></circle>
    <circle cx="640" cy="130" r="1" fill="#7c3aed"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.9s" repeatCount="indefinite" begin="0.8s"/></circle>
    <circle cx="720" cy="150" r="1" fill="#00c2ff"><animate attributeName="opacity" values="0.2;1;0.2" dur="2.1s" repeatCount="indefinite" begin="0.2s"/></circle>
  </g>
  <!-- Connection lines -->
  <g opacity="0.08">
    <line x1="40" y1="40" x2="120" y2="60" stroke="#00c2ff" stroke-width="1">
      <animate attributeName="opacity" values="0;0.3;0" dur="3s" repeatCount="indefinite"/>
    </line>
    <line x1="200" y1="30" x2="300" y2="50" stroke="#00c2ff" stroke-width="1">
      <animate attributeName="opacity" values="0;0.3;0" dur="3.5s" repeatCount="indefinite" begin="0.5s"/>
    </line>
    <line x1="400" y1="35" x2="500" y2="55" stroke="#00c2ff" stroke-width="1">
      <animate attributeName="opacity" values="0;0.3;0" dur="2.8s" repeatCount="indefinite" begin="1s"/>
    </line>
    <line x1="600" y1="40" x2="700" y2="45" stroke="#7c3aed" stroke-width="1">
      <animate attributeName="opacity" values="0;0.3;0" dur="3.2s" repeatCount="indefinite" begin="0.3s"/>
    </line>
  </g>
  <!-- Shield icon -->
  <g transform="translate(60, 80)">
    <path d="M30 10 L60 10 L60 40 Q60 65 30 75 Q0 65 0 40 L0 10 Z" fill="none" stroke="#00c2ff" stroke-width="2.5" filter="url(#glow)">
      <animate attributeName="stroke-opacity" values="0.6;1;0.6" dur="2s" repeatCount="indefinite"/>
    </path>
    <path d="M30 25 L20 35 L30 45 L45 28" fill="none" stroke="#00c2ff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
      <animate attributeName="stroke-dashoffset" values="50;0" dur="1.5s" repeatCount="indefinite"/>
      <animate attributeName="stroke-dasharray" values="50" dur="1"/>
    </path>
  </g>
  <!-- Lock icon -->
  <g transform="translate(145, 80)">
    <rect x="10" y="28" width="30" height="25" rx="3" fill="none" stroke="#7c3aed" stroke-width="2"/>
    <path d="M10 28 L10 18 Q10 5 25 5 Q40 5 40 18 L40 28" fill="none" stroke="#7c3aed" stroke-width="2.5" stroke-linecap="round"/>
    <circle cx="25" cy="38" r="3" fill="#7c3aed"><animate attributeName="opacity" values="0.3;1;0.3" dur="2s" repeatCount="indefinite"/></circle>
  </g>
  <!-- Bolt icon -->
  <g transform="translate(225, 78)">
    <polygon points="25,5 15,35 22,35 18,55 35,22 28,22 32,5" fill="none" stroke="#f59e0b" stroke-width="2" stroke-linejoin="round">
      <animate attributeName="opacity" values="0.5;1;0.5" dur="1.5s" repeatCount="indefinite"/>
    </polygon>
  </g>
  <!-- Chain link icon -->
  <g transform="translate(305, 82)">
    <path d="M15 20 Q15 10 22 10 L28 10 Q35 10 35 20 L35 25 Q35 35 28 35 L25 35" fill="none" stroke="#00c2ff" stroke-width="2" stroke-linecap="round"/>
    <path d="M35 25 Q35 35 28 35 L22 35 Q15 35 15 25 L15 20 Q15 10 22 10 L25 10" fill="none" stroke="#00c2ff" stroke-width="2" stroke-linecap="round"/>
    <circle cx="25" cy="22" r="2" fill="none" stroke="#00c2ff" stroke-width="2"/>
  </g>
  <!-- Title -->
  <text x="400" y="150" font-family="system-ui, -apple-system, sans-serif" font-size="48" font-weight="800" fill="url(#titleGrad)" text-anchor="middle" letter-spacing="2">Trellis</text>
  <!-- Subtitle -->
  <text x="400" y="180" font-family="system-ui, -apple-system, sans-serif" font-size="15" fill="#94a3b8" text-anchor="middle" letter-spacing="1">Trustless Milestone Escrow on Stellar Soroban</text>
  <!-- Decorative line -->
  <rect x="250" y="195" width="300" height="1.5" rx="1" fill="url(#accentGrad)" />
  <!-- Pulsing live indicator -->
  <g transform="translate(400, 215)">
    <circle cx="-80" cy="0" r="5" fill="#22c55e">
      <animate attributeName="r" values="5;7;5" dur="1.5s" repeatCount="indefinite"/>
      <animate attributeName="opacity" values="1;0.5;1" dur="1.5s" repeatCount="indefinite"/>
    </circle>
    <text x="-70" y="4" font-family="system-ui, sans-serif" font-size="12" fill="#22c55e" font-weight="600">LIVE ON TESTNET</text>
    <text x="30" y="4" font-family="system-ui, sans-serif" font-size="12" fill="#64748b">|</text>
    <text x="42" y="4" font-family="system-ui, sans-serif" font-size="12" fill="#64748b">Contract: CAUAO7C...</text>
  </g>
</svg>

<br />

**Trustless, milestone-based escrow for freelance and remote work — built on Stellar's Soroban smart contract platform.**

[![Contract CI](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/contract-ci.yml/badge.svg)](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/contract-ci.yml)
[![Frontend CI](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/frontend-ci.yml/badge.svg)](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/frontend-ci.yml)
[![npm audit](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/npm-audit.yml/badge.svg)](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/npm-audit.yml)
[![CodeQL](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/codeql-analysis.yml/badge.svg)](https://github.com/Trellis-Ecosystem/trellis/actions/workflows/codeql-analysis.yml)
[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)]()
[![Soroban](https://img.shields.io/badge/soroban-sdk%2022.x-blue)]()
[![Deployed](https://img.shields.io/badge/testnet-live-success)]()
[![License](https://img.shields.io/badge/license-MIT-lightgrey)]()
[![Status](https://img.shields.io/badge/status-active%20development-yellow)]()
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.1-4baaaa.svg)](CODE_OF_CONDUCT.md)

</div>

---

## The Problem

Remote work and freelance contracting run on **trust that often doesn't exist** between strangers across borders. Clients hesitate to pay upfront. Workers hesitate to deliver without payment guarantees. The usual fix — a centralized escrow middleman — adds fees, delays, and a single point of failure.

> **Trellis removes the middleman.** Funds are locked on-chain, released milestone by milestone as work is verified, with a built-in dispute process if either party disagrees. No platform holds your money. The contract does.

This matters everywhere, but especially for contributors in emerging markets — where access to reliable, low-fee, borderless payment infrastructure can be the difference between taking on international work or not.

<details>
<summary>💡 <strong>Why Soroban?</strong></summary>
<br />
Contracts are written in <strong>real Rust</strong>, compiled to WASM, and run on the Stellar network — which has <strong>fast finality</strong>, <strong>low fees</strong> (~fractions of a cent), and an established <strong>USDC presence</strong> via Circle's Stellar Asset Contract. This makes it well-suited for cross-border payment use cases.
</details>

---

## Live on Testnet

<div align="center">

<!-- Animated Testnet Status SVG -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 600 100" width="100%" style="max-width:600px;height:auto">
  <defs>
    <linearGradient id="liveGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a0e17" />
      <stop offset="100%" stop-color="#052e16" />
    </linearGradient>
  </defs>
  <rect width="600" height="100" rx="14" fill="url(#liveGrad)" stroke="#166534" stroke-width="1.5" />
  <!-- Pulsing circles -->
  <circle cx="50" cy="50" r="8" fill="#22c55e">
    <animate attributeName="r" values="8;14;8" dur="2s" repeatCount="indefinite" />
  </circle>
  <circle cx="50" cy="50" r="8" fill="#22c55e" opacity="0.3">
    <animate attributeName="r" values="8;22;8" dur="2s" repeatCount="indefinite" />
  </circle>
  <text x="75" y="45" font-family="monospace" font-size="13" fill="#4ade80" font-weight="bold">LIVE ON STELLAR TESTNET</text>
  <text x="75" y="65" font-family="monospace" font-size="11" fill="#86efac">Network: Test SDF Network ; September 2015</text>
  <text x="75" y="82" font-family="monospace" font-size="10" fill="#a7f3d0">Contract: CAUAO7CYKULE2K4EJMQ6LLRUHP7Y7JYOH6G2VBXKYG7PTETE3UZ3DU7Q</text>
  <!-- Decorative corner dots -->
  <circle cx="570" cy="20" r="2" fill="#4ade80" opacity="0.5"><animate attributeName="opacity" values="0.2;0.8;0.2" dur="1.5s" repeatCount="indefinite"/></circle>
  <circle cx="580" cy="30" r="1.5" fill="#4ade80" opacity="0.3"><animate attributeName="opacity" values="0.3;0.9;0.3" dur="2s" repeatCount="indefinite" begin="0.5s"/></circle>
</svg>

</div>

A live test agreement exists on-chain and is queryable right now:

```bash
trellis status --agreement-id 0101010101010101010101010101010101010101010101010101010101010101
```

| Attribute | Value |
|---|---|
| **Contract ID** | `CAUAO7CYKULE2K4EJMQ6LLRUHP7Y7JYOH6G2VBXKYG7PTETE3UZ3DU7Q` |
| **Network** | Stellar Testnet |
| **Explorer** | [View on Stellar Lab](https://lab.stellar.org/r/testnet/contract/CAUAO7CYKULE2K4EJMQ6LLRUHP7Y7JYOH6G2VBXKYG7PTETE3UZ3DU7Q) |

> Full deployment details, every verified command, and step-by-step deployment instructions are in [DEPLOYMENT.md](./DEPLOYMENT.md).

---

## How It Works

Trellis models a freelance engagement as an **agreement** made up of one or more **milestones**, each with its own funding, work submission, and release lifecycle.

<!-- Animated State Machine SVG -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 820 460" width="100%" style="max-width:820px;height:auto">
  <defs>
    <linearGradient id="pendingGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e293b"/>
      <stop offset="100%" stop-color="#0f172a"/>
    </linearGradient>
    <linearGradient id="fundedGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a3d3d"/>
      <stop offset="100%" stop-color="#064e3b"/>
    </linearGradient>
    <linearGradient id="workGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e1b4b"/>
      <stop offset="100%" stop-color="#312e81"/>
    </linearGradient>
    <linearGradient id="completedGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#052e16"/>
      <stop offset="100%" stop-color="#14532d"/>
    </linearGradient>
    <linearGradient id="disputedGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#2d0a0a"/>
      <stop offset="100%" stop-color="#7f1d1d"/>
    </linearGradient>
    <linearGradient id="cancelledGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e293b"/>
      <stop offset="100%" stop-color="#0f172a"/>
    </linearGradient>
    <linearGradient id="refundedGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1e3a5f"/>
      <stop offset="100%" stop-color="#1e3a8a"/>
    </linearGradient>
    <!-- Arrow marker -->
    <marker id="arrow" viewBox="0 -5 10 10" refX="28" refY="0" markerWidth="8" markerHeight="8" orient="auto">
      <path d="M0,-5L10,0L0,5" fill="#64748b"/>
    </marker>
    <marker id="arrowRed" viewBox="0 -5 10 10" refX="28" refY="0" markerWidth="8" markerHeight="8" orient="auto">
      <path d="M0,-5L10,0L0,5" fill="#ef4444"/>
    </marker>
    <filter id="stateGlow">
      <feDropShadow dx="0" dy="0" stdDeviation="4" flood-color="#00c2ff" flood-opacity="0.15"/>
    </filter>
  </defs>

  <!-- Background -->
  <rect width="820" height="460" rx="16" fill="#0a0e17" stroke="#1e293b" stroke-width="1"/>

  <!-- State Nodes -->
  <!-- Pending -->
  <g filter="url(#stateGlow)">
    <rect x="20" y="35" width="110" height="44" rx="22" fill="url(#pendingGrad)" stroke="#475569" stroke-width="1.5"/>
    <text x="75" y="61" font-family="monospace" font-size="12" fill="#94a3b8" text-anchor="middle" font-weight="bold">PENDING</text>
  </g>

  <!-- Funded -->
  <g filter="url(#stateGlow)">
    <rect x="210" y="35" width="110" height="44" rx="22" fill="url(#fundedGrad)" stroke="#10b981" stroke-width="1.5"/>
    <text x="265" y="61" font-family="monospace" font-size="12" fill="#6ee7b7" text-anchor="middle" font-weight="bold">FUNDED</text>
  </g>

  <!-- Work Submitted -->
  <g filter="url(#stateGlow)">
    <rect x="400" y="35" width="145" height="44" rx="22" fill="url(#workGrad)" stroke="#6366f1" stroke-width="1.5"/>
    <text x="472" y="61" font-family="monospace" font-size="11" fill="#a5b4fc" text-anchor="middle" font-weight="bold">WORK SUBMITTED</text>
  </g>

  <!-- Completed -->
  <g filter="url(#stateGlow)">
    <rect x="640" y="35" width="120" height="44" rx="22" fill="url(#completedGrad)" stroke="#22c55e" stroke-width="1.5"/>
    <text x="700" y="61" font-family="monospace" font-size="12" fill="#4ade80" text-anchor="middle" font-weight="bold">COMPLETED</text>
  </g>

  <!-- Disputed -->
  <g filter="url(#stateGlow)">
    <rect x="310" y="320" width="120" height="44" rx="22" fill="url(#disputedGrad)" stroke="#ef4444" stroke-width="1.5"/>
    <text x="370" y="345" font-family="monospace" font-size="12" fill="#fca5a5" text-anchor="middle" font-weight="bold">DISPUTED</text>
  </g>

  <!-- Refunded -->
  <g filter="url(#stateGlow)">
    <rect x="130" y="400" width="120" height="44" rx="22" fill="url(#refundedGrad)" stroke="#3b82f6" stroke-width="1.5"/>
    <text x="190" y="425" font-family="monospace" font-size="12" fill="#93c5fd" text-anchor="middle" font-weight="bold">REFUNDED</text>
  </g>

  <!-- Cancelled -->
  <g filter="url(#stateGlow)">
    <rect x="20" y="210" width="120" height="44" rx="22" fill="url(#cancelledGrad)" stroke="#475569" stroke-width="1.5"/>
    <text x="80" y="233" font-family="monospace" font-size="12" fill="#94a3b8" text-anchor="middle" font-weight="bold">CANCELLED</text>
  </g>

  <!-- Edges -->
  <!-- Pending -> Funded -->
  <path d="M130 57 L195 57" stroke="#94a3b8" stroke-width="1.5" fill="none" marker-end="url(#arrow)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1s" repeatCount="indefinite"/>
  </path>
  <text x="162" y="50" font-family="monospace" font-size="8" fill="#94a3b8" text-anchor="middle">lock_funds</text>

  <!-- Funded -> WorkSubmitted -->
  <path d="M320 57 L385 57" stroke="#94a3b8" stroke-width="1.5" fill="none" marker-end="url(#arrow)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1s" repeatCount="indefinite"/>
  </path>
  <text x="352" y="50" font-family="monospace" font-size="8" fill="#94a3b8" text-anchor="middle">submit_work</text>

  <!-- WorkSubmitted -> Completed -->
  <path d="M545 57 L625 57" stroke="#94a3b8" stroke-width="1.5" fill="none" marker-end="url(#arrow)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1s" repeatCount="indefinite"/>
  </path>
  <text x="585" y="50" font-family="monospace" font-size="8" fill="#94a3b8" text-anchor="middle">approve & release</text>

  <!-- Pending -> Cancelled -->
  <path d="M75 79 L75 196" stroke="#64748b" stroke-width="1.5" fill="none" marker-end="url(#arrow)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1.5s" repeatCount="indefinite"/>
  </path>
  <text x="85" y="145" font-family="monospace" font-size="8" fill="#64748b">cancel_unfunded</text>

  <!-- Funded -> Disputed -->
  <path d="M265 79 L265 185 Q265 220 310 265 Q340 300 355 320" stroke="#ef4444" stroke-width="1.5" fill="none" marker-end="url(#arrowRed)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1.2s" repeatCount="indefinite"/>
  </path>
  <text x="275" y="145" font-family="monospace" font-size="8" fill="#ef4444">raise_dispute</text>

  <!-- WorkSubmitted -> Disputed -->
  <path d="M472 79 L472 185 Q472 250 420 300 Q390 315 385 320" stroke="#ef4444" stroke-width="1.5" fill="none" marker-end="url(#arrowRed)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1.2s" repeatCount="indefinite"/>
  </path>
  <text x="482" y="200" font-family="monospace" font-size="8" fill="#ef4444">raise_dispute</text>

  <!-- Disputed -> Refunded -->
  <path d="M340 364 L340 380 Q340 390 310 400 L265 400 Q250 400 250 400" stroke="#3b82f6" stroke-width="1.5" fill="none" marker-end="url(#arrow)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1.3s" repeatCount="indefinite"/>
  </path>
  <text x="282" y="388" font-family="monospace" font-size="8" fill="#60a5fa" text-anchor="middle">resolve (refund)</text>

  <!-- Disputed -> Completed -->
  <path d="M430 342 L530 342 Q600 342 650 200 Q680 100 700 80" stroke="#22c55e" stroke-width="1.5" fill="none" marker-end="url(#arrow)" stroke-dasharray="5,3">
    <animate attributeName="stroke-dashoffset" values="0;-16" dur="1.3s" repeatCount="indefinite"/>
  </path>
  <text x="610" y="310" font-family="monospace" font-size="8" fill="#4ade80" text-anchor="middle">resolve (release)</text>

  <!-- Legend -->
  <g transform="translate(20, 400)">
    <circle cx="8" cy="8" r="5" fill="#22c55e"><animate attributeName="opacity" values="0.3;1;0.3" dur="2s" repeatCount="indefinite"/></circle>
    <text x="20" y="12" font-family="monospace" font-size="9" fill="#475569">Payer action</text>
  </g>
  <g transform="translate(170, 400)">
    <circle cx="8" cy="8" r="5" fill="#6366f1"><animate attributeName="opacity" values="0.3;1;0.3" dur="2.5s" repeatCount="indefinite"/></circle>
    <text x="20" y="12" font-family="monospace" font-size="9" fill="#475569">Payee action</text>
  </g>
  <g transform="translate(340, 400)">
    <circle cx="8" cy="8" r="5" fill="#ef4444"><animate attributeName="opacity" values="0.3;1;0.3" dur="3s" repeatCount="indefinite"/></circle>
    <text x="20" y="12" font-family="monospace" font-size="9" fill="#475569">Either party</text>
  </g>
  <g transform="translate(500, 400)">
    <circle cx="8" cy="8" r="5" fill="#3b82f6"><animate attributeName="opacity" values="0.3;1;0.3" dur="1.8s" repeatCount="indefinite"/></circle>
    <text x="20" y="12" font-family="monospace" font-size="9" fill="#475569">Resolver action</text>
  </g>
</svg>

### The Roles

| Role | Responsibility |
|---|---|
| **Payer** 🧑‍💼 | Funds milestones and approves completed work |
| **Payee** 👨‍💻 | Submits proof of completed work and receives payment on approval |
| **Dispute Resolver** ⚖️ | A neutral third party who can rule on disputes, releasing funds to either side |

### The Guarantees

- ✅ **Funds are held by the contract** — not by either party or a platform
- ✅ **Either party can raise a dispute** — neither can unilaterally freeze funds by going silent
- ✅ **Unfunded milestones can be cancelled** — walk away cleanly with no cost
- ✅ **Every state transition emits an on-chain event** — off-chain clients track progress in real time without polling

---

## Architecture

Trellis is a monorepo with three layers:

<!-- Animated Architecture SVG -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 340" width="100%" style="max-width:760px;height:auto">
  <defs>
    <linearGradient id="archBg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a0e17"/>
      <stop offset="100%" stop-color="#0f1729"/>
    </linearGradient>
    <linearGradient id="rustGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#2d0a0a"/>
      <stop offset="100%" stop-color="#7f1d1d"/>
    </linearGradient>
    <linearGradient id="cliGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a0a2d"/>
      <stop offset="100%" stop-color="#1e1b4b"/>
    </linearGradient>
    <linearGradient id="webGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#0a2d1a"/>
      <stop offset="100%" stop-color="#064e3b"/>
    </linearGradient>
    <filter id="archGlow">
      <feDropShadow dx="0" dy="4" stdDeviation="4" flood-opacity="0.2"/>
    </filter>
    <marker id="archArrow" viewBox="0 -5 10 10" refX="18" refY="0" markerWidth="6" markerHeight="6" orient="auto">
      <path d="M0,-5L10,0L0,5" fill="#334155"/>
    </marker>
  </defs>
  <rect width="760" height="340" rx="16" fill="url(#archBg)" stroke="#1e293b" stroke-width="1"/>

  <!-- Layer 1: Smart Contract -->
  <g filter="url(#archGlow)">
    <rect x="30" y="30" width="700" height="70" rx="12" fill="url(#rustGrad)" stroke="#dc2626" stroke-width="1.2"/>
    <text x="50" y="55" font-family="monospace" font-size="11" fill="#fca5a5" font-weight="bold">LAYER 1 · ON-CHAIN</text>
    <text x="50" y="82" font-family="monospace" font-size="14" fill="#f87171" font-weight="bold">contracts/trellis_core/</text>
    <text x="300" y="82" font-family="monospace" font-size="12" fill="#fca5a5">Soroban Smart Contract (Rust → WASM)</text>
    <!-- Pill tags -->
    <rect x="490" y="70" width="44" height="18" rx="9" fill="#dc2626" opacity="0.3"/>
    <text x="512" y="83" font-family="monospace" font-size="8" fill="#fca5a5" text-anchor="middle">lib.rs</text>
    <rect x="538" y="70" width="52" height="18" rx="9" fill="#dc2626" opacity="0.3"/>
    <text x="564" y="83" font-family="monospace" font-size="8" fill="#fca5a5" text-anchor="middle">types.rs</text>
    <rect x="594" y="70" width="60" height="18" rx="9" fill="#dc2626" opacity="0.3"/>
    <text x="624" y="83" font-family="monospace" font-size="8" fill="#fca5a5" text-anchor="middle">storage.rs</text>
    <rect x="658" y="70" width="56" height="18" rx="9" fill="#dc2626" opacity="0.3"/>
    <text x="686" y="83" font-family="monospace" font-size="8" fill="#fca5a5" text-anchor="middle">events.rs</text>
  </g>

  <!-- Layer 2: CLI -->
  <g filter="url(#archGlow)">
    <rect x="30" y="125" width="700" height="70" rx="12" fill="url(#cliGrad)" stroke="#6366f1" stroke-width="1.2"/>
    <text x="50" y="150" font-family="monospace" font-size="11" fill="#a5b4fc" font-weight="bold">LAYER 2 · OFF-CHAIN</text>
    <text x="50" y="177" font-family="monospace" font-size="14" fill="#818cf8" font-weight="bold">cli/trellis_cli/</text>
    <text x="300" y="177" font-family="monospace" font-size="12" fill="#a5b4fc">Command-Line Interface (Rust + clap + reqwest)</text>
    <rect x="545" y="165" width="38" height="18" rx="9" fill="#6366f1" opacity="0.3"/>
    <text x="564" y="178" font-family="monospace" font-size="8" fill="#a5b4fc" text-anchor="middle">main.rs</text>
    <rect x="587" y="165" width="52" height="18" rx="9" fill="#6366f1" opacity="0.3"/>
    <text x="613" y="178" font-family="monospace" font-size="8" fill="#a5b4fc" text-anchor="middle">config.rs</text>
    <rect x="643" y="165" width="38" height="18" rx="9" fill="#6366f1" opacity="0.3"/>
    <text x="662" y="178" font-family="monospace" font-size="8" fill="#a5b4fc" text-anchor="middle">rpc.rs</text>
  </g>

  <!-- Layer 3: Frontend -->
  <g filter="url(#archGlow)">
    <rect x="30" y="220" width="700" height="70" rx="12" fill="url(#webGrad)" stroke="#10b981" stroke-width="1.2"/>
    <text x="50" y="245" font-family="monospace" font-size="11" fill="#6ee7b7" font-weight="bold">LAYER 3 · WEB</text>
    <text x="50" y="272" font-family="monospace" font-size="14" fill="#34d399" font-weight="bold">frontend/</text>
    <text x="300" y="272" font-family="monospace" font-size="12" fill="#6ee7b7">Web Dashboard (React + Vite + TypeScript + Tailwind)</text>
    <rect x="560" y="260" width="44" height="18" rx="9" fill="#10b981" opacity="0.3"/>
    <text x="582" y="273" font-family="monospace" font-size="8" fill="#6ee7b7" text-anchor="middle">App.tsx</text>
    <rect x="608" y="260" width="80" height="18" rx="9" fill="#10b981" opacity="0.3"/>
    <text x="648" y="273" font-family="monospace" font-size="8" fill="#6ee7b7" text-anchor="middle">components/</text>
    <rect x="692" y="260" width="28" height="18" rx="9" fill="#10b981" opacity="0.3"/>
    <text x="706" y="273" font-family="monospace" font-size="8" fill="#6ee7b7" text-anchor="middle">lib/</text>
  </g>

  <!-- Animated connection dots -->
  <circle cx="60" cy="103" r="3" fill="#f87171">
    <animate attributeName="cy" values="103;108;103" dur="1.5s" repeatCount="indefinite"/>
  </circle>
  <circle cx="60" cy="198" r="3" fill="#818cf8">
    <animate attributeName="cy" values="198;203;198" dur="1.5s" repeatCount="indefinite" begin="0.3s"/>
  </circle>

  <!-- Network bubble -->
  <g transform="translate(560, 308)">
    <rect x="0" y="0" width="170" height="26" rx="13" fill="#1e293b" stroke="#334155" stroke-width="1"/>
    <circle cx="20" cy="13" r="4" fill="#22c55e">
      <animate attributeName="r" values="4;5.5;4" dur="1.5s" repeatCount="indefinite"/>
    </circle>
    <text x="32" y="17" font-family="monospace" font-size="10" fill="#94a3b8">Stellar Network</text>
  </g>

  <!-- Animated arrows to network -->
  <path d="M380 290 L380 308 Q380 320 568 320" stroke="#334155" stroke-width="1" fill="none" marker-end="url(#archArrow)" stroke-dasharray="4,3">
    <animate attributeName="stroke-dashoffset" values="0;-14" dur="1s" repeatCount="indefinite"/>
  </path>
  <path d="M380 195 L380 185 Q380 175 380 175" stroke="#334155" stroke-width="1" fill="none" marker-end="url(#archArrow)" stroke-dasharray="4,3">
    <animate attributeName="stroke-dashoffset" values="0;-14" dur="1s" repeatCount="indefinite" begin="0.5s"/>
  </path>
</svg>

### Contract Entrypoints

| Function | Caller | Effect |
|---|---|---|
| `init` | Payer | Creates a new agreement with one or more milestones (each `amount` must be strictly positive) |
| `lock_funds` | Payer | Deposits funds for a milestone into the contract |
| `submit_work` | Payee | Submits proof of completed work for a funded milestone |
| `approve_and_release` | Payer | Approves submitted work, releases funds to payee |
| `raise_dispute` | Payer or Payee | Flags a milestone for resolver review |
| `resolve_dispute` | Dispute Resolver | Rules on a dispute — refunds payer or pays payee |
| `cancel_unfunded_milestone` | Payer | Cancels a milestone that was never funded |
| `get_agreement` | Anyone | Returns the full current state of an agreement (read-only) |
| `get_total_amount` | Anyone | Returns the agreement's total value — sum of all milestone amounts (read-only) |
| `extend_agreement_ttl` | Anyone | Renews an agreement's ledger TTL to avoid archival |

<details>
<summary>📦 <strong>Storage Lifetime</strong></summary>
<br />
Soroban archives persistent ledger entries once their TTL expires, so an agreement that is never touched would eventually be lost. Every state-mutating entrypoint renews the agreement's TTL to ~30 days automatically. Agreements that stay idle longer than that — a long delivery window, a stalled dispute — need <code>extend_agreement_ttl</code> called before the TTL runs out; any address may call it, and the caller pays the rent.
</details>

### Tech Stack

<div align="center">

| Category | Technologies |
|---|---|
| **Smart Contract** | [Soroban](https://developers.stellar.org/docs/build/smart-contracts) · soroban-sdk 22.x · Rust (`#![no_std]` → WASM) |
| **CLI** | clap 4 · clap_complete · reqwest · serde + serde_json · dotenvy |
| **Frontend** | React 19 · Vite · TypeScript · Tailwind CSS · React Router |
| **Stellar SDK** | @stellar/stellar-sdk · @stellar/freighter-api · Soroban RPC |

</div>

---

## Quickstart

### Prerequisites

- Rust (stable toolchain)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [`stellar` CLI](https://developers.stellar.org/docs/tools/cli/install-cli) 26.x+
- Node.js 20+ (for the frontend)

### 🖥️ Run the frontend locally

```bash
cd frontend
cp .env.example .env
npm install
npm run dev
```

Open **http://localhost:5173** to see the animated landing page with the particle network background, typewriter effects, live contract activity stats (agreements created / milestones locked, within the RPC event retention window), and full agreement management UI.

### 🛠️ Build and test the contract

```bash
cd contracts/trellis_core
cargo test
```

All 9 integration tests run in the Soroban sandbox — happy path, double-init protection, dispute resolution, milestone cancellation (including the state-transition guard on already-funded milestones), positive-amount validation on `init`, the pre-computed `total_amount`, and the `get_agreement` view function.

### 📦 Build everything at once

```bash
make build
```

Runs the contract WASM build, CLI binary build, and frontend bundle in sequence. See `make help` for all available targets.

### 🔒 Dependency & supply-chain audit

The workspace's crate graph is scanned for known vulnerabilities, disallowed
licences, and unexpected source registries. This runs automatically in CI
(the `supply-chain` job in `.github/workflows/contract-ci.yml`) and can be run
locally:

```bash
cargo install --locked cargo-deny cargo-audit
cargo deny check      # enforces deny.toml: advisories, licences, duplicate versions, sources
cargo audit           # RustSec advisory database check
```

Note: Keep Cargo.lock in sync with Cargo.toml. CI now verifies the
lockfile with `cargo check --locked --workspace` and will fail if it's out
of date. To update the lockfile locally after changing dependencies run:

```bash
cargo update -p <pkg>
# or to generate/update the lockfile explicitly:
cargo generate-lockfile
```

Policy lives in [`deny.toml`](./deny.toml) at the repository root. A new
RustSec advisory against any dependency (direct or transitive) fails the
build.

Frontend npm dependencies are scanned by `npm audit` in CI
(`.github/workflows/npm-audit.yml`) — run it locally with:

```bash
cd frontend
npm audit --audit-level=high
```

Known false positives are documented in [`frontend/.audit-allowlist.json`](./frontend/.audit-allowlist.json).

### Building the contract WASM

```bash
cd contracts/trellis_core
cargo rustc --manifest-path=Cargo.toml --crate-type=cdylib --target=wasm32-unknown-unknown --release
```

> See [DEPLOYMENT.md](./DEPLOYMENT.md) for the full deployment walkthrough and why this command is used instead of `stellar contract build`.

### ⌨️ CLI Usage

```bash
cd cli/trellis_cli
cargo build --release

export STELLAR_RPC_URL="https://soroban-testnet.stellar.org"
export STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
export TRELLIS_CONTRACT_ID="CAUAO7CYKULE2K4EJMQ6LLRUHP7Y7JYOH6G2VBXKYG7PTETE3UZ3DU7Q"
export TRELLIS_SOURCE_KEY="<your stellar identity name>"

# Create a new agreement
trellis init \
  --agreement-id <hex-id> \
  --payer <payer-address> \
  --payee <payee-address> \
  --token <token-contract-address> \
  --resolver <resolver-address> \
  --milestones "1000,2000"

# Check status
trellis status --agreement-id <hex-id>

# Fund the first milestone
trellis lock-funds --agreement-id <hex-id> --milestone-id 0
```

All **8 CLI commands** are implemented — `init`, `lock-funds`, `submit-work`, `approve-release`, `raise-dispute`, `resolve-dispute`, `cancel-milestone`, and `status`. See [DEPLOYMENT.md](./DEPLOYMENT.md) for the full command reference.

#### Global Output Flags

These flags work with every command and can be combined with `.env`/env-var configuration:

```bash
# Preview without submitting
trellis lock-funds --agreement-id <hex-id> --milestone-id 0 --dry-run

# Machine-parseable JSON
trellis status --agreement-id <hex-id> --json

# Suppress everything except final JSON (implies --json)
trellis status --agreement-id <hex-id> --quiet

# Colorized summary
trellis status --agreement-id <hex-id> --human-readable   # or -H
```

`--dry-run` prints the `stellar contract invoke` command that would be executed
without actually running it or submitting anything on-chain. Because it never
spawns the `stellar` binary, it works on machines where the Stellar CLI is not
installed — useful for previewing command construction in CI or on a fresh
checkout.

`--json` takes priority over `--human-readable` when both are passed.

#### Network Passphrase Verification

Before any command runs, the CLI calls the configured RPC endpoint's `getNetwork`
method and compares the returned `passphrase` against `--network-passphrase`
(or `STELLAR_NETWORK_PASSPHRASE`). If they differ, the command fails early with
an error naming both values, so a mismatched `--rpc-url` (e.g. mainnet RPC with a
testnet passphrase) is caught immediately instead of surfacing as a confusing
downstream failure. This check is skipped under `--dry-run`.

#### Shell Completions

```bash
# bash
trellis completion bash > /etc/bash_completion.d/trellis

# zsh
trellis completion zsh > "${fpath[1]}/_trellis"

# fish
trellis completion fish > ~/.config/fish/completions/trellis.fish
```

Supported shells: `bash`, `zsh`, `fish`, `elvish`, `powershell`.

---

## Project Status

### ✅ Complete

- Core Soroban escrow contract — all 10 entrypoints implemented and tested
- Full state machine — happy path, dispute resolution, and cancellation paths
- Integration test suite — 41/41 passing in the Soroban sandbox
- Full CLI — all 8 commands wired end-to-end with JSON, dry-run, and human-readable output modes
- Deployed live on Stellar testnet — `init` and `status` verified against the live contract
- Frontend dashboard — 5 pages, 28 components, 12 custom hooks, animated particle network background
- Wallet connect — Freighter wallet integration with connection states
- Event feed — real-time on-chain event history per agreement (limited to the last ~100k ledgers, ~6 days, that RPC providers retain; full history awaits an event-indexing service, #496)
- Shell completions — bash, zsh, fish, elvish, powershell
- Network passphrase verification — native `getNetwork` check against the configured RPC endpoint, with a clear mismatch error naming both values

### 🚧 Open for Contribution

| Area | Description | Difficulty |
|---|---|---|
| Frontend — Agreement Status page | Polish and edge cases for live agreement state display | Intermediate |
| Frontend — Create Agreement form | Validation UX, milestone builder refinements | Intermediate |
| Frontend — Milestone actions | lock, submit, approve, dispute button workflows | Intermediate |
| Frontend — Event feed enhancements | Real-time updates, filtering, pagination | Intermediate |
| Native RPC client | Replace stellar CLI shell-out with native Rust HTTP client | Advanced |
| Documentation | CONTRIBUTING.md and contributor onboarding guide | Beginner |

See [Issues](../../issues) for the full task list — each issue has exact requirements, acceptance criteria, a suggested branch name, and a timeframe.

---

## Contributing

Trellis is built in the open and welcomes contributors of all experience levels — from documentation and frontend components to contract enhancements and tooling.

1. **Fork** the repo and clone it locally
2. Browse [open issues](../../issues) tagged `good first issue` or `help wanted`
3. **Comment** on the issue to claim it — wait for maintainer confirmation before starting
4. **Branch:** `git checkout -b feat/your-feature-name`
5. **Verify:** run `cargo test` in `contracts/trellis_core`
6. **PR:** open a PR referencing the issue with `Closes #X`

> New to Soroban? Start with the [official Soroban docs](https://developers.stellar.org/docs/build/smart-contracts/overview).  
> New to React + Stellar? Read through `frontend/src/lib/config.ts` and the [Stellar SDK docs](https://stellar.github.io/js-stellar-sdk/).

---

## License

MIT — see [LICENSE](./LICENSE) for the full text.

---

<div align="center">

<!-- Animated Footer SVG -->
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 500 60" width="100%" style="max-width:500px;height:auto">
  <defs>
    <linearGradient id="footerGrad" x1="0%" y1="0%" x2="100%" y2="0%">
      <stop offset="0%" stop-color="#00c2ff"/>
      <stop offset="50%" stop-color="#7c3aed"/>
      <stop offset="100%" stop-color="#00c2ff"/>
      <animate attributeName="x1" values="0%;100%;0%" dur="4s" repeatCount="indefinite"/>
      <animate attributeName="x2" values="100%;0%;100%" dur="4s" repeatCount="indefinite"/>
    </linearGradient>
  </defs>
  <text x="250" y="25" font-family="system-ui, sans-serif" font-size="13" fill="#64748b" text-anchor="middle">Built by Allen · Trellis Ecosystem</text>
  <text x="250" y="50" font-family="system-ui, sans-serif" font-size="11" fill="url(#footerGrad)" text-anchor="middle" font-weight="600">✦ Trustless Escrow for the Global Workforce ✦</text>
</svg>

</div>