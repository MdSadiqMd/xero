# Provider Setup And Diagnostics

Xero keeps provider setup app-local and desktop-first. Provider profiles store non-secret metadata in the provider profile registry, while API keys and token links stay in the app-local credential store. Diagnostics and copied doctor reports are designed to show enough setup state to repair a profile without exposing raw keys, OAuth tokens, bearer headers, local credential file contents, or secret-bearing paths.

## Supported Provider Paths

Use the first-class provider presets when your provider is listed directly in Providers settings:

| Provider | Auth mode | Setup notes |
| --- | --- | --- |
| OpenAI Codex | OAuth | Sign in from Providers or runtime bind. Xero stores a redacted app-local session link rather than a checked-in token. |
| OpenRouter | API key | Add an app-local key, choose a model, then run Check connection for live catalog and reachability diagnostics. |
| Anthropic | API key | Add an app-local Anthropic key. Manual model entry remains available if catalog discovery is unavailable. |
| DeepSeek | API key | Add an app-local DeepSeek key. Xero uses the hosted `https://api.deepseek.com` endpoint, live `/models` discovery, and DeepSeek V4 reasoning replay metadata. |
| GitHub Models | API token | Add a GitHub token in Providers settings. Device-flow onboarding is intentionally out of scope for this phase. |
| xAI / Grok | OAuth, device code, or API key | Sign in with an eligible Grok or X subscription account, use device-code login for SSH and remote contexts, or add an app-local xAI API key. Xero uses `https://api.x.ai/v1/responses` with `grok-4.3` seeded in the catalog. |
| OpenAI-compatible | API key or custom endpoint | Use for OpenAI, LiteLLM, Mistral, hosted compatible gateways, and custom `/v1` routes that do not have a first-class provider preset. |
| Ollama | Local | Start Ollama and use the default local endpoint or an edited local base URL. No placeholder API key is stored. |
| Azure OpenAI | API key | Use for deployment URLs that need Azure `api-version` metadata. Model ID should match the deployment name. |
| Gemini AI Studio | API key | Add an app-local Gemini key and use the built-in Gemini runtime path. |
| Amazon Bedrock | Ambient AWS | Provide region metadata. Xero checks ambient AWS readiness and does not store cloud keys. |
| Google Vertex AI | Ambient ADC | Provide region and project metadata. Xero checks ambient ADC readiness and does not store cloud keys. |

For a common cloud setup, pick the provider preset, fill the required key or ambient metadata, save the profile, then run Check connection. For a local setup, start the local server first, confirm its base URL, save the profile without a fake key, then run Check connection. For a custom gateway, use an OpenAI-compatible recipe so Xero can prefill the correct runtime shape.

## OpenAI-Compatible Recipes

The Providers settings surface includes recipe metadata that saves through the existing `openai_api` provider profile contract. Recipes do not create a separate runtime. Runtime launch, profile validation, model catalog probing, stale binding detection, and doctor diagnostics all continue through the OpenAI-compatible provider path.

| Recipe | Default base URL | Key mode | Catalog expectation |
| --- | --- | --- | --- |
| LiteLLM proxy | `http://127.0.0.1:4000/v1` | Optional | Live or manual |
| LM Studio | `http://127.0.0.1:1234/v1` | None | Live or manual |
| Mistral | `https://api.mistral.ai/v1` | Required | Live or manual |
| Groq | `https://api.groq.com/openai/v1` | Required | Live or manual |
| Together AI | `https://api.together.xyz/v1` | Required | Live or manual |
| NVIDIA NIM | `https://integrate.api.nvidia.com/v1` | Required | Live or manual |
| MiniMax | `https://api.minimax.io/v1` | Required | Live or manual |
| Azure AI Foundry | User supplied | Required | Manual |
| Atomic Chat local | `http://127.0.0.1:1337/v1` | None | Live or manual |
| Custom `/v1` gateway | User supplied | Required | Live or manual |

Hosted recipes require an app-local API key unless the recipe explicitly marks the key optional. Local recipes such as LM Studio and Atomic Chat local do not store placeholder keys. If a local server uses a different port, update the base URL before saving the profile.

Azure AI Foundry uses the OpenAI-compatible endpoint route. If the endpoint is an Azure OpenAI deployment URL that requires `api-version` metadata, use the dedicated Azure OpenAI preset instead.

DeepSeek is now a first-class provider, not an OpenAI-compatible recipe. Hosted DeepSeek uses OpenAI-style chat completions and tool calls through `https://api.deepseek.com`; local/self-hosted DeepSeek DSML prompt encoding is a separate protocol path and is not mixed into the hosted adapter.

## xAI / Grok Onboarding

Xero supports xAI / Grok as a first-class provider through browser OAuth, device-code OAuth, or an app-local xAI API key. API keys come from the xAI console and can also be represented by `XAI_API_KEY` for scripts and imported setup, but the desktop app stores configured credentials through app-local provider credential state.

Browser OAuth uses xAI's shared public OAuth client and stores only the provider credential link plus refreshable session tokens in the app-local credential store. End users do not configure an OAuth client id, and Xero does not use X Developer Portal OAuth clients for this flow. OAuth eligibility depends on xAI account and subscription policy. If a signed-in account is ineligible, re-run Check connection after switching accounts or use an API key from the xAI console.

Use device-code login when the app is running through SSH, on a remote desktop, or in any context where a localhost browser callback is inconvenient. Xero shows the user code and verification URL, polls without printing tokens, and stores the completed session through the same xAI OAuth credential path.

The browser callback matches the working xAI local-agent contract used by OpenClaw: `http://127.0.0.1:56121/callback`. xAI may label the consent application as Grok Build because this is a shared OAuth client for local Grok agents. For Xero-specific client experiments, set `XERO_XAI_OAUTH_CLIENT_ID` at build time or in the local process environment; Xero ignores ordinary X Developer Portal OAuth client ids because they are for X API endpoints such as `api.x.com` and are rejected by `auth.x.ai`. The xAI API console currently exposes API keys, not self-service OAuth client registration.

xAI runs use the native Responses API at `https://api.x.ai/v1/responses`, not the generic OpenAI-compatible profile. The initial catalog seeds `grok-4.3` with documented 1,000,000-token context and reasoning support, then learns additional models from live catalog refresh when credentials are available. xAI-native X Search, web search, code execution, file attachments, image generation, and voice are not enabled in this provider path yet.

## GitHub Models Onboarding

Xero supports GitHub Models through a saved app-local token on the `github_models` provider profile. GitHub device-flow onboarding is intentionally out of scope for the current desktop auth model because it would need a dedicated auth flow, cancellation handling, token-link storage, and redaction coverage.

Set up GitHub Models by adding a token in Providers settings, saving the profile, then running Check connection or an extended doctor report. Diagnostics use the same provider readiness, catalog, runtime binding, and stale-token checks as other API-key providers.

## Diagnostics Workflow

Use Check connection on an individual provider profile when editing credentials, endpoint metadata, or model IDs. It is the fastest way to answer "is this profile usable right now?" without scanning unrelated runtime state.

Use quick diagnostics when the issue looks local to saved profiles, runtime state, MCP settings, notification routes, or app paths. Quick mode does not probe hosted provider APIs or local model servers, so it is safe when offline and useful for support reports that only need local state.

Use extended diagnostics when reachability, hosted provider auth, model catalogs, or local model servers need to be probed. Extended mode may contact configured provider endpoints and local services, but copied report output remains redacted.

Diagnostic states mean:

- Passed: Xero could validate the checked dependency.
- Warning: Xero found a recoverable issue or stale-but-usable state, such as a cached model catalog after a retryable refresh failure.
- Failed: Xero found a blocking issue, such as missing credentials, malformed profile metadata, bad endpoint shape, failed runtime binding, or an unavailable required file.
- Skipped: Xero intentionally did not run the check, usually because the related feature is not configured or quick mode skipped network probing.

Support engineers should start with the report summary, then inspect failed and warning groups in this order: provider profiles, model catalogs, runtime supervisor, MCP dependencies, and settings dependencies. A single provider problem can cascade into runtime failures, so repair provider-profile failures first and re-run diagnostics before chasing secondary runtime messages.

## Doctor JSON Privacy Contract

Doctor JSON is intended for support and future CLI/headless surfaces. It should be stable enough to copy into an issue and private enough to share without manual scrubbing.

The diagnostics contract redacts:

- API keys and assignment-style secrets, including common OpenAI, Anthropic, GitHub, AWS, OAuth, session, and bearer-token names.
- Authorization and bearer header values, including opaque tokens that do not use recognizable prefixes.
- Local credential paths such as ADC files, AWS credential files, app-data paths, and temp paths.
- Endpoint credentials in URLs, including usernames, passwords, sensitive query parameters, and secret-looking path segments.
- Nested diagnostic checks before report rendering, so copied JSON is redacted even if a check was constructed outside the normal diagnostic factory.

Reports intentionally keep non-secret repair metadata such as provider ID, profile ID, endpoint host, region, project ID, model catalog source, retryability, and remediation text. This is the metadata support needs to explain the failure without asking users for raw secrets.
