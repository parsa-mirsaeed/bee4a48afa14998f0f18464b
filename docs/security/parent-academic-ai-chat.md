# Parent academic AI chat: privacy and authorization boundary

## Scope

The Parent AI chat is read-only guidance for **one selected, currently linked student**.
It is not a source of truth for grades and does not edit assignments, submissions, or
Teacher records. The original Parent grades and assignment screens remain authoritative.

Each request authenticates the canonical active Parent session, checks
`students.parent_id = authenticated Parent user ID` under transaction-scoped RLS,
then verifies the student's and Parent's active school identities. The browser's
child ID is only a selector, never authority or a school ID. A change to linkage,
role, school, or account activation must be rejected on the next request.

The outbound academic view is limited to **20 published assignment records** for
that linked child, ordered by due date. Each includes a bounded assignment title,
class name, due date, persisted status, and optional grade with its original numeric
value, scale and timestamp. It does **not** include names of students, private
submissions, file contents, full Teacher feedback, talent/health profiles, siblings,
or parent contact details. Empty or older results must not be represented as
complete academic history.

## Processing and user choice

Before asking, the Parent is shown a provider-processing notice and must
affirmatively check a consent acknowledgment for the selected child. The request
carries `consent_to_provider=true`; the server rejects requests without it.
Changing the child resets this acknowledgment and the client-side conversation.

The API builds fresh authorized facts for **every** request. It sends the bounded
facts, current question and up to three prior Parent questions (never previous
model answers) through the **fixed internal AI gateway** and operator-approved
model. The external provider may process the minimized prompt according to its
own contractual retention terms: **the application cannot promise immediate
provider-side deletion**. Before enabling external LLM processing for students,
the operator must confirm the applicable DPA, jurisdiction, safeguarding
requirements, provider retention, and school/parent disclosure obligations.

EduTalent does not persist Parent chat transcripts in its database; at most six
question/answer pairs are kept in the current in-memory web component and cleared
when changing child or leaving the component. No provider credential is sent to
the browser. Application errors are bounded to safe, non-secret codes.

## Abuse and failure controls

- Maximum 800 characters per current question and 400 per previous question;
  at most three previous questions.
- Maximum 15,000 serialized prompt bytes and 2,000 response characters.
- Per-Parent budget: 12 requests over five minutes per application process,
  with bounded key storage. The gateway independently applies school-level quotas.
  Multi-replica deployments must not rely on process-local limits as the only
  shared quota.
- Request timeout is capped at 30 seconds and output budget at 768 tokens.
- Reject credential-shaped strings before gateway forwarding, require JSON
  with a plain-text answer, validate model identity and suppress raw gateway
  errors. The UI renders model output as text, not HTML.
- Do not invent academic facts during outages; direct the Parent to original
  grade/assignment records. Responses are AI-generated advice and require
  human verification.

## Validation boundaries

Required release evidence includes Parent/other-role and cross-school denials,
revoked child links, current data authorization under NOBYPASSRLS, prompt injection
resistance, privacy-safe logging and retention, bilingual mobile browser behavior,
and a controlled successful AI gateway response. Offline-unavailable browser proof
is **not** evidence of a successful model-answer journey.
