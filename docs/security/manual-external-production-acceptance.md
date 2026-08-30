# Manual and External Production Acceptance

## Purpose

This document is the single owner-directed record for **human-only and externally qualified production-readiness evidence** that repository CI cannot legitimately produce.

Automated accessibility checks, GitHub Actions, synthetic load, CI recovery drills, generated documentation, or AI review cannot substitute for the human/external decisions recorded here. Nothing in this document is considered PASS until the named qualified person or external reviewer completes and signs the corresponding record against the exact frozen release candidate.

## Clean-repository migration status

This record was rebuilt in the new public EduTalent repository after repository-history sanitation and security remediation.

- Repository: `parsa-mirsaeed/bee4a48afa14998f0f18464b`
- Migration baseline on `main` when this record was rebuilt: `5ed53ab35552859b52d5595a27de19ec733e8ca2`
- Frozen release source SHA: **PENDING — must be recorded after the active engineering PR sequence is complete**
- Exact validated release-candidate head: **PENDING**
- Final Release Acceptance workflow run: **PENDING — must be generated in this repository**
- Final release evidence artifact digest: **PENDING — must be generated in this repository**
- Appliance manifest SHA-256: **PENDING — must be generated for the frozen release candidate**
- Packaged model revision: **PENDING — must be verified for the frozen release candidate**
- Automated release classification: **PENDING fresh exact-head validation**
- Automated `ready_for_contracted_production`: **must remain false until all required automated and human/external acceptance is complete**
- Qualification start date (UTC):
- Qualification owner:

Historical workflow run IDs, artifact digests, source SHAs, signatures, attestations, and other commit-addressed evidence from the retired repository are **superseded and are not valid evidence for this repository's final release candidate**. Do not copy them into the fields above. Generate fresh evidence after the release candidate is frozen here.

Human/external testing must identify the exact installed artifact/build digest as well as the source SHA. If the candidate source SHA, signed release artifact, deployment configuration, or any material input changes after a check whose result could be affected, rerun that check or explicitly record a legitimate applicability rationale.

---

## A. Human keyboard and screen-reader acceptance

Detailed procedure: `docs/security/pr-12-manual-accessibility-acceptance.md`.

Required evidence:

- tester identity;
- exact installed/tested release artifact and source SHA;
- browser/version and operating system;
- screen reader/version;
- keyboard-only results for authentication/logout, role navigation, student submission, teacher grading/feedback, persisted grade view, and Persian/RTL journeys;
- findings and permitted owner/date/expiry risk acceptance;
- final exact tested SHA.

Status: **PENDING HUMAN TEST**

---

## B. Clean target-host qualification and operator acceptance

Primary engineering procedure: `deploy/production/operations/TARGET_HOST_ACCEPTANCE.md`.

Required real-host evidence:

- clean supported replacement-host identity and supported-host preflight output;
- exact signed release/artifact installed and its digests;
- OS/kernel/CPU/RAM/storage/inode/filesystem evidence;
- actual at-rest encryption evidence;
- firewall/network/DNS/time-sync evidence;
- rootless Docker evidence or a reviewed rootful-Docker exception;
- tailored CIS/container-hardening assessment;
- immutable locked-release image/digest inventory;
- genuinely off-appliance encrypted backup and WAL destination;
- separate backup-passphrase escrow;
- fresh installation of the frozen release;
- encrypted backup restore, PostgreSQL PITR and Qdrant restore/reindex decision on the replacement host;
- disk-low/full, TLS, corrupt-config, failed-migration and AI/Qdrant/provider outage acceptance;
- measured school-specific RPO and RTO;
- school-scale load/soak and capacity-headroom results;
- patch/certificate/key/Supabase/Qdrant/model upgrade/rollback rehearsal as applicable;
- explicit single-node/not-HA acknowledgement;
- residual-risk owner/date and operator/security sign-off.

Status: **PENDING CONTROLLED HOST QUALIFICATION**

---

## C. Independent authorization/security review

Required reviewer evidence:

- reviewer/organization and independence statement;
- exact release candidate reviewed;
- authorization/tenant-isolation review scope;
- authentication/session review scope;
- privileged/operator surface review scope;
- AI Gateway/provider egress and tenant-boundary review scope;
- backup/recovery and secret-handling review scope;
- findings with severity and reproduction/evidence;
- remediation PR/issue references;
- residual findings with explicit owner/date/acceptance rationale;
- reviewer disposition.

Status: **PENDING INDEPENDENT REVIEW**

---

## D. Penetration test and remediation disposition

Required evidence:

- qualified tester/organization;
- dates and exact release candidate/environment;
- agreed scope and exclusions;
- test methodology and authenticated roles used;
- findings and severity;
- remediation references;
- retest results for remediated findings;
- explicit disposition for every unresolved finding;
- final tester/report sign-off.

Status: **PENDING EXTERNAL PENETRATION TEST**

---

## E. Privacy, legal and contract sign-off

The repository documentation package is an engineering/business input, not legal advice or legal approval.

Qualified legal/privacy/business reviewers must approve or explicitly disposition, as applicable:

- controller/processor role allocation;
- processing/data inventory and data-flow accuracy;
- subprocessors/providers, data location and international-transfer assessment;
- retention/deletion and data-subject procedures;
- breach/incident notification procedure;
- DPIA trigger/template and AI-use/human-oversight notice;
- school provisioning and parent/student notice responsibilities;
- end-of-contract export/return/deletion process;
- proprietary deployment/use grant and contracted feature/exclusion schedule;
- support/escalation/maintenance-window terms;
- availability definition and accepted RPO/RTO responsibilities;
- customer hardware/network responsibilities;
- security-incident cooperation;
- AI-provider responsibility/outage behavior;
- acceptance procedure and warranty/limitation language.

Record:

- legal/privacy reviewer(s):
- business/contract owner(s):
- review date(s):
- exact document/release revision reviewed:
- required changes and references:
- final approval/disposition:

Status: **PENDING QUALIFIED LEGAL/PRIVACY/BUSINESS REVIEW**

---

## F. Operator acceptance and incident rehearsal

Required human operational evidence:

- named primary and backup operator(s);
- exact release/deployment configuration used;
- installation/upgrade/rollback walkthrough completed;
- alert handling and escalation walkthrough completed;
- backup/restore/PITR procedure walkthrough completed;
- certificate/key rotation walkthrough completed;
- simulated incident with declared incident lead, communications/escalation and recovery actions;
- recovery/communication timestamps;
- gaps/actions with owners and due dates;
- operator acceptance sign-off.

Status: **PENDING OPERATOR REHEARSAL**

---

## G. Final manual/external release disposition

The final human/external acceptance decision must confirm all applicable sections above are complete and reconcile every residual risk with the exact frozen release candidate. Green automated proof is an input to this decision, not a replacement for it.

Required sign-offs:

- accessibility tester:
- target-host/operator owner:
- security reviewer:
- penetration-test owner:
- privacy/legal approver:
- contract/business approver:
- release owner:

Open residual risks (owner + expiry/review date):

-

Final manual/external classification — select exactly one only after the required qualified evidence exists:

- [ ] not accepted
- [ ] safe to continue developing
- [ ] ready for final validation
- [ ] ready for limited pilot
- [ ] ready for contracted production

Decision rationale:

Decision date:

Frozen release source SHA verified unchanged:

Installed signed artifact/digest verified:
