#!/usr/bin/env node
// CI-only synthetic HTTPS first-party provider for the REAL native-PDF gateway.
// Binds to loopback, presents ephemeral api.openai.com SAN certificate, rejects
// wrong model/route/token/bytes, stores only SHA/length/revision, not private PDF.
import https from 'node:https';
import http from 'node:http';
import fs from 'node:fs';
import crypto from 'node:crypto';

if (process.env.CI !== 'true' || process.env.E2E_NATIVE_PDF_REAL_GATEWAY !== '1') {
  throw new Error('Synthetic provider requires isolated CI-only mode');
}
const [keyPath, certPath] = process.argv.slice(2);
if (!keyPath || !certPath) throw new Error('Ephemeral TLS key and certificate required');
const key = fs.readFileSync(keyPath);
const cert = fs.readFileSync(certPath);
const keyToken = 'ci-real-gateway-provider-key-abcdefghijklmnopqrstuvwxyz';
const calls = [];
function respond(res, status, payload) {
  res.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store' });
  res.end(JSON.stringify(payload));
}
function deny(res, reason) {
  console.warn(`synthetic_https_provider_denied=${reason}`);
  respond(res, 422, { error: { message: 'Synthetic provider rejected native PDF input', type: 'invalid_request_error' } });
}
const provider = https.createServer({ key, cert }, (req, res) => {
  if (req.socket.remoteAddress !== '127.0.0.1' &&
      req.socket.remoteAddress !== '::ffff:127.0.0.1' &&
      req.socket.remoteAddress !== '::1') return deny(res, 'non_loopback');
  if (req.method === 'GET' && req.url === '/v1/healthz') return respond(res, 200, { status: 'ok' });
  if (req.method !== 'POST' || req.url !== '/v1/chat/completions' ||
      req.headers.host !== 'api.openai.com' ||
      req.headers.authorization !== `Bearer ${keyToken}`) return deny(res, 'route_or_credentials');

  const chunks = [];
  let total = 0;
  req.on('data', chunk => {
    total += chunk.length;
    if (total > 1_500_000) { req.destroy(); return; }
    chunks.push(chunk);
  });
  req.on('end', () => {
    try {
      const payload = JSON.parse(Buffer.concat(chunks).toString('utf8'));
      const encoded = payload.messages?.[1]?.content?.[1]?.file?.file_data;
      const context = payload.messages?.[1]?.content?.[0];
      const system = payload.messages?.[0]?.content;
      if (payload.model !== 'gpt-4o' ||
          payload.response_format?.type !== 'json_object' ||
          context?.type !== 'text' || typeof context?.text !== 'string' ||
          payload.messages?.[1]?.content?.[1]?.type !== 'file' ||
          payload.messages[1].content[1].file?.filename !== 'submission.pdf' ||
          typeof system !== 'string' || !system.includes('untrusted DATA') ||
          typeof encoded !== 'string' ||
          !encoded.startsWith('data:application/pdf;base64,')) return deny(res, 'native_contract');
      const pdf = Buffer.from(encoded.slice('data:application/pdf;base64,'.length), 'base64');
      const rubric = JSON.parse(context.text);
      const sha256 = crypto.createHash('sha256').update(pdf).digest('hex');
      if (pdf.length < 50 || pdf.length > 1_000_000 ||
          !pdf.subarray(0, 8).toString().startsWith('%PDF-1.') ||
          !pdf.toString('latin1').trimEnd().endsWith('%%EOF') ||
          !/^[0-9a-f-]{36}$/.test(rubric.submission_revision) ||
          rubric.grade_scale !== 20 || !Array.isArray(rubric.rubric) ||
          rubric.rubric.length !== 1 ||
          typeof rubric.rubric[0].criterion_id !== 'string' ||
          !rubric.rubric[0].criterion_id.trim() ||
          rubric.rubric[0].possible_points !== 20) return deny(res, 'invalid_original_or_rubric');
      calls.push({
        revision: rubric.submission_revision,
        sha256,
        input_bytes: pdf.length,
        provider_model: payload.model,
      });
      console.log('synthetic_https_provider_native_pdf_accepted=1');
      const suggestion = {
        contract_version: 1,
        submission_revision: rubric.submission_revision,
        grade_scale: rubric.grade_scale,
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
      respond(res, 200, {
        model: 'gpt-4o',
        choices: [{
          index: 0,
          message: { role: 'assistant', content: JSON.stringify(suggestion) },
          finish_reason: 'stop',
        }],
      });
    } catch {
      deny(res, 'invalid_json');
    }
  });
});
provider.listen(9443, '127.0.0.1', () => {
  console.log('synthetic HTTPS provider bound to loopback:9443');
});

// Strictly read-only, separate diagnostic port. This NEVER serves originals
// and cannot accept requests causing provider inference.
const evidence = http.createServer((req, res) => {
  if (req.method !== 'GET' || req.url !== '/__e2e/pdf-calls') {
    res.writeHead(404);
    res.end();
    return;
  }
  respond(res, 200, calls);
});
evidence.listen(9444, '127.0.0.1', () => {
  console.log('synthetic provider metadata evidence ready on loopback:9444');
});
