# Provider setup and diagnostics

Cadence keeps provider setup app-local and desktop-first. Provider profiles store non-secret metadata in the profile registry and store API keys or token links in the app-local credential store. Diagnostics and copied doctor reports should describe enough setup state to repair a profile without exposing raw secrets.

## OpenAI-compatible recipes

The Providers settings surface includes first-class recipes that save through the existing `openai_api` provider profile contract:

- LiteLLM proxy
- LM Studio
- Mistral
- Groq
- Together AI
- DeepSeek
- NVIDIA NIM
- MiniMax
- Azure AI Foundry
- Atomic Chat local
- Custom `/v1` gateway

Recipes prefill label, model id, base URL expectations, key requirements, catalog expectations, and repair copy. They do not create a separate runtime. Runtime launch, profile validation, model catalog probing, stale binding detection, and doctor diagnostics all continue through the OpenAI-compatible provider path.

Hosted recipes require an app-local API key unless the recipe explicitly says the key is optional. Local recipes such as LM Studio and Atomic Chat local do not store placeholder keys. If a local server exposes a different port, update the base URL before saving the profile.

Azure AI Foundry uses the OpenAI-compatible endpoint route in the `openai_api` recipe path. Deployment-level Azure OpenAI endpoints that require `api-version` metadata should use the dedicated Azure OpenAI preset instead.

## GitHub Models onboarding

Cadence supports GitHub Models through a saved app-local token on the `github_models` provider profile. GitHub device-flow onboarding is intentionally out of scope for the current desktop auth model because it would need a dedicated auth flow, cancellation handling, token-link storage, and redaction coverage. Users should add a token in Providers settings, then run Check connection or an extended doctor report.

## Diagnostics workflow

Use Check connection on an individual provider profile when editing credentials, endpoint metadata, or model ids. Use quick diagnostics when the issue looks local to saved profiles, runtime state, MCP settings, notification routes, or app paths. Use extended diagnostics when reachability, hosted provider auth, model catalogs, or local model servers need to be probed.

Doctor JSON is intended for support and future CLI/headless surfaces. It should remain redacted, stable, and useful even when a subset of provider checks fails.
