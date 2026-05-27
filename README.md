# DOM Wallet

**Deterministic Monetary Network** — a portable Windows desktop client for the DOM Protocol.

Single executable. No installer. No AppData. The full node, wallet, miner, and replay log live beside the `.exe`.

---

## What this is

DOM Wallet is the first operational client of the DOM Protocol — a deterministic monetary network. It is **not a mockup, not a webview, not a wrapper**. It is a native Rust application built on `eframe`/`egui` that embeds:

- a real DOM node with chain state, mempool, and peer registry
- a deterministic wallet (24-word seed, Ed25519 signing, AES-256-GCM at rest)
- an optional CPU miner with proof-of-work
- a replay diagnostics log of operationally significant events (canonical tip changes, reorgs, restart recovery, mempool reconciliation)
- a daily update check against the GitHub releases manifest

The node, sync, and miner all run **independently of the wallet unlock state**. The password protects only operations that require the signing key — sending transactions and revealing the seed. The wallet operates as monetary infrastructure: it stays connected and produces blocks even when the UI is locked.

---

## Design constraints

These are non-negotiable and define the look-and-feel:

- **Cinematic dark interface.** Black + amber palette only. `#06070A` background, `#D6A85F` primary accent, `#F0C674` highlight. No neon, no rainbow gradients, no playful color.
- **Centerpiece composition.** A glowing halo around the DOM coin, mirrored on a reflective water surface. The two-line aphorism in Portuguese: *"A ordem nasce do determinismo. A moeda é a medida da liberdade."*
- **Restrained typography.** Generous whitespace, monospace for addresses and hashes, all-caps section labels with letter spacing.
- **Brazilian number format.** Balances render as `3.482,2456 DOM` — thousand separator `.`, decimal separator `,`, four fractional digits.
- **Portable directory layout.** Everything lives beside the executable:
  ```
  DOM Wallet.exe
  chain/       blockchain data
  wallet/      encrypted wallet store
  config/      defaults + user overrides
  peers/       peer registry (backbone seeded)
  logs/        rolling log files
  snapshots/   replay event log (JSONL)
  runtime/     runtime state (last height, flags)
  updates/     staged update payloads
  ```

---

## Running it

### From a release

1. Download `DOM-Wallet-Windows-Portable.zip` from the [Releases](https://github.com/dom-protocol/dom-wallet/releases) page.
2. Extract anywhere — Desktop, a USB drive, a network share.
3. Double-click `DOM Wallet.exe`.

That is the entire install procedure.

### From source

Requires Rust stable (≥ 1.76) and the MSVC toolchain on Windows.

```powershell
git clone https://github.com/dom-protocol/dom-wallet
cd dom-wallet
cargo build --release --target x86_64-pc-windows-msvc
```

The resulting binary is at `target/x86_64-pc-windows-msvc/release/dom-wallet-app.exe`.

### CLI flags

| Flag       | Effect                                                |
| ---------- | ----------------------------------------------------- |
| `--hidden` | Start with the window hidden (system tray only)       |
| `--mine`   | Start mining immediately, overriding persisted state  |

---

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                        DOM Wallet (single .exe)                    │
│                                                                    │
│  ┌──────────────┐    ┌────────────────────────────────────────┐    │
│  │   eframe UI  │◄──►│             embedded Node              │    │
│  │  (main thr.) │    │   ┌──────────┐ ┌──────────┐ ┌────────┐ │    │
│  │              │    │   │  Chain   │ │ Mempool  │ │ Miner  │ │    │
│  │  Sidebar     │    │   │  state   │ │          │ │        │ │    │
│  │  Hero        │    │   └──────────┘ └──────────┘ └────────┘ │    │
│  │  Views (7)   │    │   ┌──────────┐ ┌──────────┐ ┌────────┐ │    │
│  │  Modals      │    │   │   P2P    │ │  Peers   │ │ Snap.  │ │    │
│  │              │    │   │ supervisor│ │ registry │ │  log   │ │    │
│  └──────────────┘    │   └──────────┘ └──────────┘ └────────┘ │    │
│         ▲            └────────────────────────────────────────┘    │
│         │              owned tokio runtime · 2 worker threads      │
│         │                                                          │
│  ┌──────┴─────────┐                                                │
│  │     Wallet     │  bip39 (24 words) → Ed25519 → SHA-256 → bs58  │
│  │  (encrypted    │  AES-256-GCM + PBKDF2(SHA-256, 200k iters)    │
│  │   container)   │  wallet/wallet.json                            │
│  └────────────────┘                                                │
└────────────────────────────────────────────────────────────────────┘
        │
        └─►  backbone peer: 168.100.8.245:33370
```

### Code map

```
src/
  main.rs                  entry point, tracing, eframe bootstrap
  chain/
    block.rs               BlockHeader, Block, merkle, genesis
    tx.rs                  TxBody, Transaction, TxStatus, format_dom
    state.rs               ChainState (Arc/RwLock) + persistence
  wallet/mod.rs            keygen, AES-GCM seal, sign, parse_dom_amount
  mining/mod.rs            PoW miner (16-bit difficulty, mempool drain)
  net/
    peer.rs                PeerRegistry, PeerInfo, health tracking
    p2p.rs                 TCP supervisor, JSON protocol, handshake
  node/mod.rs              orchestrator owning the tokio runtime
  persist/
    paths.rs               portable_beside_executable()
    snapshot.rs            SnapshotLog (JSONL events)
    runtime_state.rs       restart-safe state (last_height, flags)
  update/mod.rs            GitHub releases check
  ui/
    app.rs                 DomApp eframe::App, Modal state, sidebar mux
    theme.rs               palette + visuals install
    sidebar.rs             7-section nav with monochrome glyphs
    hero.rs                background + water + halo + coin + reflection
    views/
      common.rs            panel, metric, status_pill, tactile_button
      inicio.rs            hero column + status column
      carteira.rs          identity card + sensitive operations
      transacoes.rs        tx history with lifecycle pills
      rede.rs              backbone + peer registry
      mineracao.rs         mining toggle + hashrate
      diagnosticos.rs      snapshot log + node logs
      configuracoes.rs     execution + updates + portable path
```

---

## Determinism

DOM is deterministic by construction:

- **Chain ID** is a constant string (`dom-devnet-1` on devnet) — no automatic forks, no contested branches.
- **Wallet seed** is generated once at first launch and persists forever; the same machine boots into the same address every time.
- **Block hashing** uses canonical bincode serialization → SHA-256. No floating-point, no system-dependent randomness in the hash path.
- **Mining** searches an `u64` nonce space deterministically. Two miners with the same mempool produce identical candidate blocks until one wins on nonce ordering.
- **Replay snapshots** record every operationally significant transition. The diagnostics view reconstructs the runtime story from these events.

If anything in this client behaves non-deterministically across two identical runs from the same persisted state, that is a bug — please open an issue.

---

## Security model

| Asset                    | Protection                                                                 |
| ------------------------ | -------------------------------------------------------------------------- |
| Wallet seed              | AES-256-GCM under PBKDF2-SHA-256 with 200,000 iterations and a random salt |
| Signing key (in memory)  | Held only while the wallet is unlocked; zeroized on lock or drop           |
| Public address           | Stored in cleartext so the UI can display balance while locked             |
| Chain data, peers, logs  | No encryption — portable inspection is intentional                         |
| Transport (P2P)          | TCP + JSON length-prefixed messages on devnet (noise/quic planned)         |
| Auto-update              | Daily GitHub Releases manifest check; staged into `updates/` only          |

**No telemetry. No analytics. No phone-home.** The only outbound traffic the client originates is (1) the backbone peer connection and (2) the GitHub releases API check.

---

## Releases

Tagged `v*.*.*` pushes trigger the workflow at `.github/workflows/release.yml`, which:

1. Builds for `x86_64-pc-windows-msvc` in release mode.
2. Stages the portable directory skeleton.
3. Bundles the executable, default `config/default.toml`, and backbone-seeded `peers/peers.json`.
4. Produces `DOM-Wallet-Windows-Portable.zip` with a companion `.sha256` file.
5. Attaches both to the GitHub Release.

The artifact is identical bit-for-bit when built from the same Cargo.lock, the same rustc, and the same source revision.

---

## License

MIT. See `LICENSE`.

---

## Where this is headed

DEVNET v0 is the floor. The trajectory:

- replace the best-effort TCP/JSON p2p with a proper noise/quic transport
- header-first IBD with checkpoint verification
- LMDB-backed block and state storage
- system tray with global hotkey to summon the window
- real fiat oracle for the balance display
- Linux and macOS portable builds with the same single-file ethos

The constraint that never changes: **the client must remain a single portable executable with all state beside it**.
