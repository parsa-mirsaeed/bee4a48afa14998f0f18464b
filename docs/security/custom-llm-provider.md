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
AI_LLM_BASE_URL=https://ai.parspack.com/v1/
LLM_PROFILE=custom-v1:flash-4.1
LLM_MODEL=flash-4.1
LLM_API_KEY=<operator-provided-secret>
```

This ParsPack URL/model pair is an illustrative configuration; verify that the provider supports the exact model and required OpenAI-compatible JSON response format before production use.

The model suffix in `LLM_PROFILE` must exactly equal `LLM_MODEL`. Allowed
model names are 1–96 ASCII characters, start with a letter or digit, and
otherwise contain only letters, digits, `.`, `_`, `-`, `/`, or `:`.
The HTTPS endpoint must have a public DNS hostname (no IP literals, localhost,
or private/internal naming suffixes), use port 443, and have no URL credentials,
query or fragment. Prefix paths such as `/openai/v1/` are supported.

No fixed provider IP is required. For each new custom-provider connection, the
gateway resolves DNS and passes **only publicly routable IPv4 addresses** to the
HTTP transport, rejecting private, loopback, metadata, reserved, and IPv6
addresses. DNS is not re-resolved between filtering and TCP connection, so DNS
rebinding cannot change a vetted socket address. The custom LLM uses a separate
HTTP client with all inherited proxies and redirects disabled, HTTPS port 443,
and normal TLS hostname verification. DNS rotation is handled on new connections.
DNS names that have no approved public IPv4 address fail closed. IPv6-only
custom providers are not yet supported; never loosen the filter to work around
this.

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
- Gateway auth, school-scoped quotas, timeouts, dynamic public-IP-only DNS resolution, redirects disabled, response
  validation, and circuit breaking remain enforced.
- Provider keys are gateway-only, never persisted in the database.
- Local embeddings/Qdrant and tenant authorization are unchanged.
- An arbitrary HTTPS service is not automatically OpenAI-compatible: verify
  the provider's response schema and exact model echo before production use.
