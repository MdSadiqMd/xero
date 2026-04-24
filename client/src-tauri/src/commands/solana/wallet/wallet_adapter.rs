//! Legacy `@solana/wallet-adapter-react` scaffold.
//!
//! Produces a minimal React + Vite app that wires the wallet-adapter
//! provider + a ConnectButton. Every concrete adapter we bundle is the
//! free-tier option — Phantom, Solflare, Backpack are auto-discovered
//! via wallet-standard; we include Ledger as a concrete fallback.

use super::{ScaffoldMeta, WalletScaffoldContext};

pub fn render(ctx: &WalletScaffoldContext) -> (Vec<(String, String)>, ScaffoldMeta) {
    let files = vec![
        ("package.json".into(), package_json(ctx)),
        ("tsconfig.json".into(), tsconfig()),
        ("vite.config.ts".into(), vite_config()),
        ("index.html".into(), index_html(ctx)),
        ("src/main.tsx".into(), main_tsx(ctx)),
        ("src/App.tsx".into(), app_tsx(ctx)),
        ("src/WalletProviders.tsx".into(), providers_tsx(ctx)),
        ("src/ConnectButton.tsx".into(), connect_button_tsx()),
        ("src/app.css".into(), css()),
        ("README.md".into(), readme(ctx)),
        (".gitignore".into(), gitignore()),
    ];
    let meta = ScaffoldMeta {
        entrypoint: Some("src/main.tsx".into()),
        start_command: "pnpm dev".into(),
        api_key_env: None,
        next_steps: vec![
            "pnpm install".into(),
            "pnpm dev  # opens the connect page on http://localhost:5173".into(),
            "Swap `CLUSTER` in src/WalletProviders.tsx once you're ready to move past devnet."
                .into(),
        ],
    };
    (files, meta)
}

fn package_json(ctx: &WalletScaffoldContext) -> String {
    format!(
        r#"{{
  "name": "{slug}",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "dev": "vite",
    "build": "tsc --noEmit && vite build",
    "preview": "vite preview",
    "typecheck": "tsc --noEmit"
  }},
  "dependencies": {{
    "@solana/wallet-adapter-base": "^0.9.23",
    "@solana/wallet-adapter-react": "^0.15.35",
    "@solana/wallet-adapter-react-ui": "^0.9.35",
    "@solana/wallet-adapter-wallets": "^0.19.32",
    "@solana/web3.js": "^1.95.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1"
  }},
  "devDependencies": {{
    "@types/react": "^18.3.3",
    "@types/react-dom": "^18.3.0",
    "@vitejs/plugin-react": "^4.3.1",
    "typescript": "^5.5.0",
    "vite": "^5.3.0"
  }}
}}
"#,
        slug = ctx.project_slug,
    )
}

fn tsconfig() -> String {
    r#"{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "esModuleInterop": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "allowSyntheticDefaultImports": true
  },
  "include": ["src"]
}
"#
    .into()
}

fn vite_config() -> String {
    r#"import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"

export default defineConfig({
  plugins: [react()],
  define: {
    // wallet-adapter-base pulls in a few polyfills; Vite handles the rest.
    "process.env": {},
  },
})
"#
    .into()
}

fn index_html(ctx: &WalletScaffoldContext) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{app_name}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#,
        app_name = ctx.app_name,
    )
}

fn main_tsx(ctx: &WalletScaffoldContext) -> String {
    format!(
        r#"import React from "react"
import ReactDOM from "react-dom/client"
import {{ WalletProviders }} from "./WalletProviders"
import {{ App }} from "./App"
import "@solana/wallet-adapter-react-ui/styles.css"
import "./app.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <WalletProviders>
      <App appName="{app_name}" />
    </WalletProviders>
  </React.StrictMode>,
)
"#,
        app_name = escape_ts_string(&ctx.app_name),
    )
}

fn providers_tsx(ctx: &WalletScaffoldContext) -> String {
    format!(
        r#"import {{ useMemo, type ReactNode }} from "react"
import {{ ConnectionProvider, WalletProvider }} from "@solana/wallet-adapter-react"
import {{ WalletModalProvider }} from "@solana/wallet-adapter-react-ui"
import {{ LedgerWalletAdapter }} from "@solana/wallet-adapter-wallets"

// Cluster baked in at scaffold time. Swap this constant to move to
// mainnet or a custom RPC.
const RPC_URL = "{rpc_url}"

export function WalletProviders({{ children }}: {{ children: ReactNode }}) {{
  // Wallet-standard-aware wallets (Phantom, Solflare, Backpack, etc.)
  // are auto-discovered. We still bundle Ledger explicitly because the
  // hardware wallet does not announce itself via wallet-standard.
  const wallets = useMemo(() => [new LedgerWalletAdapter()], [])

  return (
    <ConnectionProvider endpoint={{RPC_URL}}>
      <WalletProvider wallets={{wallets}} autoConnect>
        <WalletModalProvider>{{children}}</WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  )
}}
"#,
        rpc_url = escape_ts_string(&ctx.rpc_url),
    )
}

fn connect_button_tsx() -> String {
    r#"import { WalletMultiButton } from "@solana/wallet-adapter-react-ui"

export function ConnectButton() {
  return <WalletMultiButton />
}
"#
    .into()
}

fn app_tsx(_ctx: &WalletScaffoldContext) -> String {
    r#"import { useWallet } from "@solana/wallet-adapter-react"
import { ConnectButton } from "./ConnectButton"

interface AppProps {
  appName: string
}

export function App({ appName }: AppProps) {
  const { publicKey, connected } = useWallet()
  return (
    <main className="app">
      <header>
        <h1>{appName}</h1>
        <ConnectButton />
      </header>
      <section className="status">
        {connected && publicKey ? (
          <p>
            Connected as <code>{publicKey.toBase58()}</code>
          </p>
        ) : (
          <p>Click connect to choose a wallet.</p>
        )}
      </section>
    </main>
  )
}
"#
    .into()
}

fn css() -> String {
    r#"body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
  background: #0a0a0a;
  color: #f5f5f5;
}
.app {
  min-height: 100vh;
  padding: 32px;
}
.app header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  padding-bottom: 16px;
  margin-bottom: 24px;
}
.status {
  font-size: 14px;
  line-height: 1.6;
}
.status code {
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  word-break: break-all;
}
"#
    .into()
}

fn readme(ctx: &WalletScaffoldContext) -> String {
    format!(
        r#"# {slug}

`@solana/wallet-adapter-react` scaffold generated by Cadence's Solana
workbench. Renders a connect button and reports the connected wallet's
public key.

## Run

```bash
pnpm install
pnpm dev
```

## Cluster

Baked in at scaffold time — currently **{cluster}** (RPC: `{rpc_url}`).
Update `RPC_URL` in `src/WalletProviders.tsx` to switch clusters.

## What's included

- Wallet-standard-aware wallet discovery (Phantom, Solflare, Backpack,
  any wallet that announces via the wallet-standard protocol).
- Ledger support via the explicit `LedgerWalletAdapter`.
- Auto-connect after first consent.

## What's not

- No staking/signing UI — `useWallet()` gives you `signMessage`,
  `signTransaction`, and `sendTransaction`; wire those into your feature
  screens.
- No mobile deep-linking — see the `mwa-stub` scaffold in the workbench
  if you need mobile support.
"#,
        slug = ctx.project_slug,
        cluster = ctx.cluster.as_str(),
        rpc_url = ctx.rpc_url,
    )
}

fn gitignore() -> String {
    "node_modules\ndist\n.DS_Store\n".into()
}

pub(crate) fn escape_ts_string(value: &str) -> String {
    value
        .replace('\\', r"\\")
        .replace('"', r#"\""#)
        .replace('\n', r"\n")
}
