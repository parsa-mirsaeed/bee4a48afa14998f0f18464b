import { test, expect, type APIRequestContext, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { createHash, randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { enforceOfflineAllowlist, assertNoUnexpectedOrigins } from '../fixtures/network-policy';
import { watchConsole, assertNoConsoleErrors, allowHttpResponse } from '../fixtures/console-guard';

type Requests=Pick<APIRequestContext,'get'|'post'>;
// Chromium treats the loopback application as a secure context. Playwright's
// standalone HTTP client does not send Secure cookies over that same HTTP URL.
// Copy the actual HttpOnly session cookies only to the configured application
// origin; all authorization/session validation still runs in the real server.
function api(page:Page):Requests {
  const client=page.context().request;
  const origin=new URL(process.env.E2E_BASE_URL??'http://127.0.0.1:8080');
  const call=async(method:'get'|'post',url:string,options:any={})=>{
    const target=new URL(url,origin);
    const headers={...options.headers};
    if(target.origin===origin.origin&&origin.protocol==='http:'&&['127.0.0.1','localhost'].includes(origin.hostname)){
      headers.cookie=(await page.context().cookies()).filter(cookie=>
        cookie.domain===origin.hostname&&target.pathname.startsWith(cookie.path)
      ).map(cookie=>`${cookie.name}=${cookie.value}`).join('; ');
    }
    return client[method](url,{...options,headers,maxRedirects:0});
  };
  return {get:(url,options)=>call('get',url,options),post:(url,options)=>call('post',url,options)};
}
const root='/api/submissions/attachments';
const student='e2e-attachment-student@example.test';
async function login(request: Requests,email=student) {
  const response=await request.post('/api/auth/login',{data:{email,password:'e2e-password'}});
  expect(response.ok()).toBeTruthy();
  const {user}=await response.json();
  const identity=await request.post('/api/auth/whoami',{data:{}});
  expect(identity.ok()).toBeTruthy();
  expect((await identity.json()).id,`session must switch to ${email}`).toBe(user.id);
}
function pdf():Buffer {
  let text='%PDF-1.4\n';const offsets=[0];
  for(const [index,body] of ['<< /Type /Catalog /Pages 2 0 R >>','<< /Type /Pages /Kids [3 0 R] /Count 1 >>','<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>'].entries()) {
    offsets.push(Buffer.byteLength(text));text+=`${index+1} 0 obj\n${body}\nendobj\n`;
  }
  const xref=Buffer.byteLength(text);
  text+='xref\n0 4\n0000000000 65535 f \n';
  text+=offsets.slice(1).map(n=>`${String(n).padStart(10,'0')} 00000 n \n`).join('');
  text+=`trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(text);
}
const originals={
  pdf:{name:'homework.pdf',mimeType:'application/pdf',buffer:pdf()},
  png:{name:'handwritten.png',mimeType:'image/png',buffer:Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAIAAAACCAIAAAD91JpzAAAAFklEQVR4nGP8//8/AwMDEwMDAwMDAwAkBgMB/DXemwAAAABJRU5ErkJggg==','base64')},
  jpg:{name:'photo.jpg',mimeType:'image/jpeg',buffer:Buffer.from('/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/2wBDAQkJCQwLDBgNDRgyIRwhMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjIyMjL/wAARCAACAAIDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAAAgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREAAgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPExcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwD3+iiigD//2Q==','base64')},
};
const cases=[{id:1,locale:'en',files:[originals.pdf],text:''},{id:2,locale:'fa',files:[originals.png],text:''},{id:3,locale:'en',files:[originals.jpg],text:''},{id:4,locale:'fa',files:[originals.pdf,originals.png,originals.jpg],text:'Written answer with originals'}] as const;
function identity(project:string,caseId:number) {
  const suffix=`${project.includes('mobile')?2:1}${String(caseId).padStart(2,'0')}`;
  return {title:`E2E Attachment ${suffix}`,id:`f1600000-0000-0000-0000-${suffix.padStart(12,'0')}`};
}
async function list(request:Requests,assignment_id:string) {
  const result=await request.get(`${root}/list`,{params:{assignment_id}});expect(result.ok(),`${result.status()}: ${(await result.text()).slice(0,1000)}`).toBeTruthy();return result.json();
}
async function reserve(request:Requests,assignment_id:string,file=originals.pdf,request_id=randomUUID()) {
  return request.post(`${root}/reserve`,{data:{input:{assignment_id,request_id,filename:file.name,media_type:file.mimeType,byte_size:file.buffer.length,sha256:createHash('sha256').update(file.buffer).digest('hex')}}});
}

test.beforeEach(async({page})=>{await enforceOfflineAllowlist(page);watchConsole(page);});
test.afterEach(()=>{assertNoUnexpectedOrigins();assertNoConsoleErrors();});
for(const scenario of cases) {
  test(`private originals complete ${scenario.id} ${scenario.locale} @smoke @final @student @workflow-truth @accessibility`,async({page},info)=>{
    const identityForTest=identity(info.project.name,scenario.id);const fa=scenario.locale==='fa';
    await page.addInitScript(locale=>localStorage.setItem('edutalent_locale',locale),scenario.locale);
    await login(api(page));await page.goto('/dashboard/assignments');
    const card=page.getByText(identityForTest.title,{exact:true}).locator('xpath=ancestor::article[1]');
    await card.getByRole('button',{name:fa?'شروع تکلیف':'Start assignment',exact:true}).click();
    await page.getByRole('dialog').getByRole('button',{name:fa?'باز کردن ارسال من':'Open my submission',exact:true}).click();
    const dialog=page.getByRole('dialog');
    const text=dialog.getByLabel(fa?'ارسال من':'My submission',{exact:true});await expect(text).toBeEnabled();
    if(scenario.text)await text.fill(scenario.text);
    const picker=dialog.getByLabel(fa?'افزودن فایل‌ها':'Add files',{exact:true});
    await expect(picker).toBeEnabled();await picker.setInputFiles([...scenario.files]);
    const submit=dialog.getByRole('button',{name:fa?'ارسال کار':'Submit work',exact:true});
    const upload=dialog.getByRole('button',{name:fa?'بارگذاری / تلاش مجدد فایل‌ها':'Upload / retry files',exact:true});
    await expect(upload).toBeEnabled();
    if(scenario.id===4) {
      allowHttpResponse(`${root}/upload`,503);
      try {
        expect((await api(page).post('http://127.0.0.1:9100/__e2e/storage-mode',{data:{mode:'unavailable'}})).ok()).toBeTruthy();
        await upload.click();
        await expect(dialog.getByRole('alert')).toContainText('متن شما حفظ شده است');
        await expect(text).toHaveValue(scenario.text);
        await expect(submit).toBeDisabled();
        await expect(upload).toBeEnabled();
      } finally {await api(page).post('http://127.0.0.1:9100/__e2e/storage-mode',{data:{mode:'ready'}});}
    }
    let releaseList!:()=>void;
    let listReached!:()=>void;
    const held=new Promise<void>(resolve=>{releaseList=resolve;});
    const reached=new Promise<void>(resolve=>{listReached=resolve;});
    await page.route('**/api/submissions/attachments/list*',async route=>{listReached();await held;await route.continue();});
    try {
      await upload.click();
      await reached;
      await expect(upload).toHaveCount(0);
      await expect(submit).toBeDisabled();
    } finally {releaseList();}
    await expect(picker).toBeEnabled();
    await page.unroute('**/api/submissions/attachments/list*');
    await expect.poll(async()=> (await list(api(page),identityForTest.id)).filter((f:any)=>f.status==='ready').length).toBe(scenario.files.length);
    await expect(upload).toHaveCount(0);
    const axe=await new AxeBuilder({page}).include('[role="dialog"]').withTags(['wcag2a','wcag2aa','wcag21aa']).analyze();
    expect(axe.violations.filter(v=>v.impact==='serious'||v.impact==='critical')).toEqual([]);
    // Both locales expose the same file count and original bytes after finalize.
    const finalResponse=page.waitForResponse(r=>r.url().includes('/api/submissions/finalize'));
    await submit.click();expect((await finalResponse).ok()).toBeTruthy();await expect(dialog).toHaveCount(0);
    const files=await list(api(page),identityForTest.id);expect(files).toHaveLength(scenario.files.length);
    expect(files.every((f:any)=>f.status==='submitted')).toBeTruthy();
    for(const file of files) {
      const source=scenario.files.find(f=>f.name===file.filename)!;
      const downloaded=await api(page).get(`${root}/download`,{params:{id:file.id}});
      expect(downloaded.ok()).toBeTruthy();expect(downloaded.headers()['content-disposition']).toMatch(/^attachment;/);
      expect(downloaded.headers()['cache-control']).toContain('no-store');
      expect(await downloaded.body()).toEqual(source.buffer);
    }
    await login(api(page),'e2e-attachment-teacher@example.test');
    const teacherFiles=await list(api(page),identityForTest.id);expect(teacherFiles).toHaveLength(files.length);
    expect((await api(page).get(`${root}/download`,{params:{id:files[0].id}})).ok()).toBeTruthy();
    await page.goto('/dashboard/submissions');
    // The original control is also rendered in the real Teacher grading dialog.
    const teacherCard=page.getByText(identityForTest.title,{exact:true}).locator('xpath=ancestor::div[contains(@class,"rounded-xl")][1]');
    await teacherCard.getByRole('button').last().click();
    await expect(page.getByRole('dialog').getByRole('button',{name:fa?'دریافت اصل فایل':'Download original',exact:true})).toHaveCount(files.length);
    const downloadEvent=page.waitForEvent('download');
    await page.getByRole('dialog').getByRole('button',{name:fa?'دریافت اصل فایل':'Download original',exact:true}).first().click();
    const download=await downloadEvent;const path=await download.path();expect(path).not.toBeNull();
    expect(createHash('sha256').update(await readFile(path!)).digest('hex')).toBe(files[0].sha256);
    for(const email of ['e2e-student-b@example.test','e2e-teacher-b@example.test','e2e-parent-a@example.test','e2e-manager-a@example.test','e2e-admin@example.test']) {
      await login(api(page),email);
      expect((await api(page).get(`${root}/download`,{params:{id:files[0].id}})).status()).toBe(403);
    }
  });
}

test('upload retries, limits and incomplete finalization stay truthful @smoke @final @student @workflow-truth',async({page},info)=>{
  await login(api(page));const request=api(page);const {id}=identity(info.project.name,90);
  const token=randomUUID();let result=await reserve(request,id,originals.pdf,token);expect(result.ok(),`${result.status()}: ${(await result.text()).slice(0,1000)}`).toBeTruthy();const attachment=await result.json();
  result=await reserve(request,id,originals.pdf,token);expect(result.ok(),`${result.status()}: ${(await result.text()).slice(0,1000)}`).toBeTruthy();expect((await result.json()).id).toBe(attachment.id);
  expect(await list(request,id)).toHaveLength(1);
  let finalized=await request.post('/api/submissions/finalize',{data:{assignment_id:id,content:'preserved answer',request_id:randomUUID(),expected_revision:null,attachment_ids:[attachment.id]}});
  expect(finalized.ok()).toBeFalsy();
  let saved=await request.post('/api/submissions/get_for_assignment',{data:{assignment_id:id}});expect(saved.ok()).toBeTruthy();expect(await saved.json()).toBeNull();
  try {
    expect((await request.post('http://127.0.0.1:9100/__e2e/storage-mode',{data:{mode:'unavailable'}})).ok()).toBeTruthy();
    const upload=await request.post(`${root}/upload?id=${attachment.id}`,{data:originals.pdf.buffer,headers:{'content-type':'application/pdf'}});expect(upload.status()).toBe(503);
    expect((await list(request,id))[0].status).toBe('pending');
  } finally {await request.post('http://127.0.0.1:9100/__e2e/storage-mode',{data:{mode:'ready'}});}
  for(let attempt=0;attempt<2;attempt++) expect((await request.post(`${root}/upload?id=${attachment.id}`,{data:originals.pdf.buffer,headers:{'content-type':'application/pdf'}})).ok()).toBeTruthy();
  expect(await list(request,id)).toHaveLength(1);
  const finalizeToken=randomUUID();const data={assignment_id:id,content:'preserved answer',request_id:finalizeToken,expected_revision:null,attachment_ids:[attachment.id]};
  finalized=await request.post('/api/submissions/finalize',{data});expect(finalized.ok()).toBeTruthy();const first=await finalized.json();
  finalized=await request.post('/api/submissions/finalize',{data});expect(finalized.ok()).toBeTruthy();expect(await finalized.json()).toEqual(first);
  expect((await request.post('/api/submissions/finalize',{data:{...data,content:'stale overwrite',request_id:randomUUID()}})).ok()).toBeFalsy();
  expect((await request.post(`${root}/remove`,{data:{attachment_id:attachment.id}})).ok()).toBeFalsy();
  const bad={...originals.png,name:'pretend.pdf',mimeType:'application/pdf'};result=await reserve(request,id,bad);expect(result.ok(),`${result.status()}: ${(await result.text()).slice(0,1000)}`).toBeTruthy();const badId=(await result.json()).id;
  expect((await request.post(`${root}/upload?id=${badId}`,{data:bad.buffer,headers:{'content-type':'application/pdf'}})).status()).toBe(415);
  expect((await request.post(`${root}/remove`,{data:{attachment_id:badId}})).ok()).toBeTruthy();
  expect((await reserve(request,id,{...originals.pdf,name:'work.svg',mimeType:'image/svg+xml'})).ok()).toBeFalsy();
  const oversized={input:{assignment_id:id,request_id:randomUUID(),filename:'large.pdf',media_type:'application/pdf',byte_size:10485761,sha256:'a'.repeat(64)}};
  expect((await request.post(`${root}/reserve`,{data:oversized})).ok()).toBeFalsy();
  // Total bytes and file count are reserved under the assignment row lock,
  // including incomplete uploads. Failed uploads cannot evade either bound.
  const reserved:string[]=[];
  for(let n=0;n<2;n++) {
    const r=await request.post(`${root}/reserve`,{data:{input:{...oversized.input,request_id:randomUUID(),filename:`large${n}.pdf`,byte_size:9*1024*1024}}});
    expect(r.ok()).toBeTruthy();reserved.push((await r.json()).id);
  }
  expect((await request.post(`${root}/reserve`,{data:{input:{...oversized.input,request_id:randomUUID(),byte_size:9*1024*1024}}})).ok()).toBeFalsy();
  for(const attachment_id of reserved)expect((await request.post(`${root}/remove`,{data:{attachment_id}})).ok()).toBeTruthy();
  for(let n=0;n<4;n++)expect((await reserve(request,id)).ok()).toBeTruthy();
  expect((await reserve(request,id)).ok()).toBeFalsy();
  const unsubmitted=(await list(request,id)).find((f:any)=>f.status==='pending');
  expect(unsubmitted).toBeTruthy();
  await login(request,'e2e-student-b@example.test');
  expect((await request.post(`${root}/upload?id=${unsubmitted.id}`,{data:originals.pdf.buffer,headers:{'content-type':'application/pdf'}})).status()).toBe(403);
  expect((await request.post(`${root}/remove`,{data:{attachment_id:unsubmitted.id}})).ok()).toBeFalsy();
  await login(request);
  expect((await request.post(`${root}/upload?id=${unsubmitted.id}`,{data:originals.pdf.buffer,headers:{'content-type':'application/pdf'}})).ok()).toBeTruthy();
  const objectKeys=async()=> (await request.get('http://127.0.0.1:9100/__e2e/submission-objects')).json() as Promise<string[]>;
  expect((await objectKeys()).filter(k=>k.endsWith(`/${attachment.id}`))).toHaveLength(1);
  expect((await objectKeys()).filter(k=>k.endsWith(`/${unsubmitted.id}`))).toHaveLength(1);
  expect((await request.post(`${root}/remove`,{data:{attachment_id:unsubmitted.id}})).ok()).toBeTruthy();
  await expect.poll(async()=> (await objectKeys()).filter(k=>k.endsWith(`/${unsubmitted.id}`)).length,{timeout:25_000}).toBe(0);
  expect((await objectKeys()).filter(k=>k.endsWith(`/${attachment.id}`))).toHaveLength(1);

});
