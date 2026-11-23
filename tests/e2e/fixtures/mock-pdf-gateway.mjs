#!/usr/bin/env node
// Synthetic CI-only INTERNAL gateway contract fixture, NOT the real AI gateway.
// Tests the release-mode application worker, immutable original bytes, RLS,
// durable budget, and Teacher review without provider access or external egress.
// The real gateway -> first-party mock provider is separately covered by
// native_pdf_two_hop_mock_gateway_preserves_original_and_rubric.
import http from 'node:http';
import crypto from 'node:crypto';

const port = 8090;
const token = 'e2e-browser-proof-token-0123456789abcdef';
const school = 'a0000000-0000-0000-0000-0000000000c1';
const calls = [];
const fail = (res, code, reason = 'contract_denied') => {
  // Fixed reason codes only: never log PDF bytes, school prompts or tokens.
  console.warn(`synthetic_pdf_gateway_deny=${reason} status=${code}`);
  res.writeHead(code, { 'content-type': 'application/json' });
  res.end(JSON.stringify({ error: { code: 'fixture_rejected', message: 'Invalid synthetic PDF gateway request' } }));
};
const ok = (res, payload) => {
  res.writeHead(200, { 'content-type': 'application/json' });
  res.end(JSON.stringify(payload));
};
const server = http.createServer((req, res) => {
  // This server is bound to loopback only and has no provider credentials.
  if (req.socket.remoteAddress !== '127.0.0.1' &&
      req.socket.remoteAddress !== '::ffff:127.0.0.1' &&
      req.socket.remoteAddress !== '::1') return fail(res, 403, 'non_loopback');
  if (req.method === 'GET' && req.url === '/__e2e/pdf-calls') {
    return ok(res, calls);
  }
  if (req.method !== 'POST' || req.url !== '/v1/grading/pdf')
    return fail(res, 403, 'wrong_endpoint');
  if (req.headers.host !== 'ai-gateway:8090')
    return fail(res, 403, 'wrong_internal_origin');
  if (req.headers.authorization !== `Bearer ${token}`)
    return fail(res, 403, 'wrong_internal_token');
  if (req.headers['x-edutalent-school-id'] !== school)
    return fail(res, 403, 'wrong_school');
  const chunks = [];
  let count = 0;
  req.on('data', part => {
    count += part.length;
    if (count > 1_500_000) { req.destroy(); return; }
    chunks.push(part);
  });
  req.on('end', () => {
    try {
      const p = JSON.parse(Buffer.concat(chunks).toString('utf8'));
      const pdf = Buffer.from(p.pdf_base64, 'base64');
      const digest = crypto.createHash('sha256').update(pdf).digest('hex');
      const rubric = JSON.parse(p.teacher_rubric_prompt);
      if (p.model !== 'gpt-4o' ||
          p.pdf_sha256 !== digest || pdf.length < 50 || pdf.length > 1_000_000 ||
          !pdf.subarray(0, 8).toString().startsWith('%PDF-1.') ||
          !pdf.toString('latin1').trimEnd().endsWith('%%EOF') ||
          !/^[0-9a-f-]{36}$/.test(p.submission_id) ||
          p.submission_revision !== rubric.submission_revision ||
          rubric.grade_scale !== 20 ||
          typeof rubric.assignment_instructions !== 'string' ||
          !rubric.assignment_instructions.trim() ||
          !Array.isArray(rubric.rubric) || rubric.rubric.length !== 1 ||
          typeof rubric.rubric[0].criterion_id !== 'string' ||
          !rubric.rubric[0].criterion_id.trim() ||
          rubric.rubric[0].possible_points !== 20 ||
          typeof p.system_instructions !== 'string' ||
          !p.system_instructions.includes('untrusted DATA')) {
        return fail(res, 422, 'invalid_pdf_or_rubric_binding');
      }
      // Never store original bytes or student text; only identity and digest.
      calls.push({
        submission_id: p.submission_id,
        revision: p.submission_revision,
        attachment_id: p.attachment_id,
        sha256: digest,
        input_bytes: pdf.length,
      });
      const suggestion = {
        contract_version: 1,
        submission_revision: p.submission_revision,
        grade_scale: 20,
        suggested_grade: 18,
        confidence: 0.85,
        feedback: 'Synthetic PDF grading evidence: Teacher review required.',
        criteria: [{
          criterion_id: rubric.rubric[0].criterion_id,
          awarded_points: 18,
          possible_points: 20,
          evidence: 'Synthetic document matches the approved accuracy criterion.',
        }],
      };
      console.log('synthetic_pdf_gateway_accepted=1');
      ok(res, { model: 'gpt-4o', choices: [{
        index: 0,
        message: { role: 'assistant', content: JSON.stringify(suggestion) },
        finish_reason: 'stop',
      }] });
    } catch {
      fail(res, 422, 'malformed_request');
    }
  });
});
server.listen(port, '127.0.0.1', () => console.log('synthetic gateway contract listening on loopback'));
