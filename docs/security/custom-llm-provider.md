# Operator-configured custom LLM provider

EduTalent's assignment personalization always calls the internal AI gateway.
The gateway is the only process carrying the external LLM API key and external
egress network membership. Embeddings and Qdrant remain independent; the local
BGE profile does not need to change.

## Fast manual provider switch

Use this when the two pinned choices are unavailable, **only** with a provider
offering an OpenAI-compatible `POST /chat/completions` API, a Bearer API key,
JSON-object response mode, and responses whose `model` field exactly matches
the requested model. The operator, not schools or teachers, chooses the URL.

In the local mode-600 `deploy/production/.env.edutalent`:

```dotenv
AI_GATEWAY_MODE=offline
AI_LLM_MODE=connected
AI_LLM_BASE_URL=https://api.your-provider.tld/openai/v1/
AI_LLM_CUSTOM_PUBLIC_IP=<verified-public-ipv4-for-the-provider>
LLM_PROFILE=custom-v1:vendor/model-name
LLM_MODEL=vendor/model-name
LLM_API_KEY=<operator-provided-secret>
```

The model suffix in `LLM_PROFILE` must exactly equal `LLM_MODEL`. Allowed
model names are 1–96 ASCII characters, start with a letter or digit, and
otherwise contain only letters, digits, `.`, `_`, `-`, `/`, or `:`.
The HTTPS endpoint must have a public DNS hostname (no IP literals, localhost,
or private/internal naming suffixes), use port 443, and have no URL credentials,
query or fragment. The operator must also supply a **globally routable IPv4 address
pin** for the chosen hostname. The gateway bypasses proxies and pins every HTTPS
connection to that address while verifying the hostname against the provider's
TLS certificate. This prevents DNS rebinding toward private/metadata addresses;
refresh and review the IP pin if the provider rotates its addresses. Use a
real provider IP, not the example address in configuration comments. Prefix paths such as `/openai/v1/` are supported.

The app receives **only** `LLM_PROFILE` and `LLM_MODEL`; no provider credentials
or external URL. Compose passes those variables to the AI gateway, which
enforces the exact active model and provider endpoint. The database stores
the selected model and provider with each job, so existing queued jobs retain
their original audit contract. Platform Admin will offer the configured
`custom-v1:...` policy choice. Schools and teachers cannot create new egress
destinations.

1. Review the new provider's data-processing/security terms: minimized
   student learning context may leave the deployment when it is enabled.
2. Back up configuration, run `bash edutalent production-validate`, apply the
   additive custom-profile migration, then restart affected services using
   the established production upgrade procedure.
3. In Platform Admin > Personalization, select the matching new custom
   profile for the relevant school/teacher policy; default policies stay pinned
   to DeepSeek unless explicitly changed.
4. Verify `bash edutalent production-gateway-check` and
   `bash edutalent production-ai-check`, then complete a controlled sample
   personalization with an appropriately authorized student/test fixture.
5. To fall back, switch the three profile/model/origin values and API key to
   the pinned DeepSeek or GapGPT contract, validate and restart, then update
   the affected Platform Admin policies. Do not rewrite existing succeeded
   jobs or silently re-target their model.

This feature is **manual provider selection**, not transparent per-request
failover. Provider outages should surface through the existing retry/circuit
breaker and job status controls; automatic fallback is deliberately excluded
from this initial scope to prevent unnoticed transfers of student data.

## Invariants

- Fixed DeepSeek and GapGPT origin/profile/model pairs remain exact and
  unchanged.
- Only one operator-owned custom origin/model is active; no arbitrary
  client-provided URL/model and no wildcard tenant-controlled egress.
- Gateway auth, school-scoped quotas, timeouts, redirects disabled, response
  validation, and circuit breaking remain enforced.
- Provider keys are gateway-only, never persisted in the database.
- Local embeddings/Qdrant and tenant authorization are unchanged.
- An arbitrary HTTPS service is not automatically OpenAI-compatible: verify
  the provider's response schema and exact model echo before production use.
