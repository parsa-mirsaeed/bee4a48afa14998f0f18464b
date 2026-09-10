use crate::i18n::use_locale;
use api::server_functions::submission_attachment_functions::*;
use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
struct LocalFile {
    request_id: Uuid,
    filename: String,
    media_type: String,
    bytes: Vec<u8>,
    reserved_id: Option<Uuid>,
}

#[component]
pub fn SubmissionFiles(
    assignment_id: Uuid,
    #[props(default = false)] editable: bool,
    attachments: Signal<Vec<SubmissionAttachment>>,
    busy: Signal<bool>,
    loaded: Signal<bool>,
    unuploaded: Signal<bool>,
) -> Element {
    let locale = use_locale();
    let mut pending = use_signal(Vec::<LocalFile>::new);
    let mut error = use_signal(|| false);
    let mut epoch = use_signal(|| 0_u64);
    let mut current = use_resource(move || {
        let _ = epoch();
        async move { list_submission_attachments(assignment_id).await }
    });
    use_effect(move || match current.read().as_ref() {
        Some(Ok(files)) => {
            attachments.set(files.clone());
            loaded.set(true);
        }
        Some(Err(_)) => {
            loaded.set(false);
            error.set(true);
        }
        None => {}
    });
    use_effect(move || unuploaded.set(!pending().is_empty()));
    let upload = move |_| {
        if busy() || pending().is_empty() {
            return;
        }
        busy.set(true);
        error.set(false);
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                while let Some(mut file) = pending().first().cloned() {
                    use sha2::{Digest, Sha256};
                    let intent = AttachmentIntent {
                        assignment_id,
                        request_id: file.request_id,
                        filename: file.filename.clone(),
                        media_type: file.media_type.clone(),
                        byte_size: file.bytes.len() as i64,
                        sha256: format!("{:x}", Sha256::digest(&file.bytes)),
                    };
                    let reserved = match reserve_submission_attachment(intent).await {
                        Ok(value) => value,
                        Err(_) => {
                            error.set(true);
                            break;
                        }
                    };
                    file.reserved_id = Some(reserved.id);
                    pending.with_mut(|files| {
                        if let Some(first) = files.first_mut() {
                            first.reserved_id = Some(reserved.id);
                        }
                    });
                    let result = gloo_net::http::Request::post(&format!(
                        "/api/submissions/attachments/upload?id={}",
                        reserved.id
                    ))
                    .header("Content-Type", &file.media_type)
                    .body(file.bytes);
                    let success = match result {
                        Ok(request) => request.send().await.is_ok_and(|r| r.ok()),
                        Err(_) => false,
                    };
                    if !success {
                        error.set(true);
                        break;
                    }
                    pending.with_mut(|files| {
                        files.remove(0);
                    });
                }
                // Refresh even after failure: a lost response may have committed
                // the intent or verified bytes. The same token remains retryable.
                loaded.set(false);
                epoch.set(epoch() + 1);
            }
            busy.set(false);
        });
    };
    rsx! {
        section { class:"space-y-3", "aria-label":locale.t("submission.files.title"),
            h3 { class:"font-semibold", "{locale.t(\"submission.files.title\")}" }
            if error() {
                p { id:"submission-files-error",role:"alert",class:"text-sm text-red-800 dark:text-red-200", "{locale.t(\"submission.files.error\")}" }
            }
            if !loaded() {
                button { class:"et-ui-button et-ui-button--md et-ui-button--secondary", disabled:busy(), onclick:move |_|{error.set(false);current.restart();}, "{locale.t(\"common.refresh\")}" }
            }
            for file in attachments().into_iter().filter(|f| !pending().iter().any(|p| p.reserved_id==Some(f.id))) {
                div { key:"{file.id}",class:"flex flex-wrap items-center gap-3 rounded-lg border p-3",
                    span { class:"min-w-0 break-all", "{file.filename}" }
                    bdi { dir:"ltr", class:"text-sm", "{file.byte_size} B" }
                    span { class:"text-sm", {locale.t(match file.status {AttachmentStatus::Pending=>"submission.files.pending",AttachmentStatus::Ready=>"submission.files.ready",AttachmentStatus::Submitted=>"submission.files.submitted"})} }
                    if file.status!=AttachmentStatus::Pending {
                        DownloadOriginal { file:file.clone() }
                    }
                    if editable && file.status!=AttachmentStatus::Submitted {
                        button { class:"et-ui-button et-ui-button--md et-ui-button--secondary",disabled:busy(),
                            onclick:move |_|{busy.set(true);spawn(async move{if remove_submission_attachment(file.id).await.is_err(){error.set(true);}loaded.set(false);epoch.set(epoch()+1);busy.set(false);});},
                            "{locale.t(\"submission.files.remove\")}"
                        }
                    }
                }
            }
            for file in pending() {
                div { key:"{file.request_id}",class:"flex flex-wrap items-center gap-3 rounded-lg border p-3",
                    span {class:"break-all", "{file.filename}"}
                    span {"{locale.t(\"submission.files.pending\")}"}
                    button {class:"et-ui-button et-ui-button--md et-ui-button--secondary",disabled:busy(),
                        onclick:move |_|{
                            let file=file.clone();busy.set(true);
                            spawn(async move{
                                let removed=match file.reserved_id{Some(id)=>remove_submission_attachment(id).await.is_ok(),None=>true};
                                if removed{pending.with_mut(|files|files.retain(|p|p.request_id!=file.request_id));loaded.set(false);epoch.set(epoch()+1);}else{error.set(true);}
                                busy.set(false);
                            });
                        },
                        "{locale.t(\"submission.files.remove\")}"
                    }
                }
            }
            if editable {
                p { id:"submission-files-help",class:"text-sm text-gray-700 dark:text-gray-300", "{locale.t(\"submission.files.limits\")}" }
                label { r#for:"submission-files-picker",class:"block font-medium", "{locale.t(\"submission.files.add\")}" }
                input { id:"submission-files-picker",r#type:"file",multiple:true,accept:"application/pdf,image/jpeg,image/png,.pdf,.jpg,.jpeg,.png",disabled:busy() || !loaded(),
                    "aria-describedby":"submission-files-help submission-files-error",
                    onchange:move |_|{
                        #[cfg(target_arch="wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            let input=web_sys::window().and_then(|w|w.document()).and_then(|d|d.get_element_by_id("submission-files-picker")).and_then(|e|e.dyn_into::<web_sys::HtmlInputElement>().ok());
                            if let Some(input)=input {
                                let files:Vec<_>=input.files().map(|files|(0..files.length()).filter_map(|i|files.get(i)).collect()).unwrap_or_default();
                                let remote:Vec<_>=attachments().into_iter().filter(|f| !pending().iter().any(|p| p.reserved_id==Some(f.id))).collect();
                                let bytes=files.iter().map(|f|f.size()).sum::<f64>()+pending().iter().map(|f|f.bytes.len() as f64).sum::<f64>()+remote.iter().map(|f|f.byte_size as f64).sum::<f64>();
                                if files.len()+pending().len()+remote.len()>MAX_SUBMISSION_FILES || bytes>MAX_SUBMISSION_FILE_BYTES as f64 || files.iter().any(|f|f.size()<=0.0 || f.size()>MAX_ATTACHMENT_BYTES as f64) {error.set(true);input.set_value("");return;}
                                busy.set(true);error.set(false);
                                spawn(async move{
                                    for file in files {
                                        match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                                            Ok(buffer)=>pending.with_mut(|files|files.push(LocalFile{request_id:Uuid::new_v4(),filename:file.name(),media_type:file.type_(),bytes:js_sys::Uint8Array::new(&buffer).to_vec(),reserved_id:None})),
                                            Err(_)=>{error.set(true);break;},
                                        }
                                    }
                                    input.set_value("");busy.set(false);
                                });
                            }
                        }
                    }
                }
                if !pending().is_empty() {
                    button {class:"et-ui-button et-ui-button--md et-ui-button--secondary",disabled:busy() || !loaded(),onclick:upload,"{locale.t(\"submission.files.upload_retry\")}"}
                    p {role:"status", "{locale.t(\"submission.files.upload_before_submit\")}"}
                }
            }
            if busy() { p {role:"status","aria-live":"polite","{locale.t(\"submission.files.working\")}"} }
        }
    }
}

#[component]
pub fn SubmittedOriginals(assignment_id: Uuid) -> Element {
    let files = use_signal(Vec::new);
    let busy = use_signal(|| false);
    let loaded = use_signal(|| false);
    let unuploaded = use_signal(|| false);
    rsx! { SubmissionFiles { assignment_id, attachments:files,busy,loaded,unuploaded } }
}

#[component]
fn DownloadOriginal(file: SubmissionAttachment) -> Element {
    let locale = use_locale();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| false);
    rsx! {
        button {class:"et-ui-button et-ui-button--md et-ui-button--secondary",disabled:busy(),
            onclick:move |_|{
                let file=file.clone();busy.set(true);error.set(false);
                spawn(async move{
                    #[cfg(target_arch="wasm32")]
                    if download(&file).await.is_err(){error.set(true);}
                    busy.set(false);
                });
            },
            "{locale.t(\"submission.files.download\")}"
        }
        if error(){span{role:"alert",class:"text-sm text-red-800 dark:text-red-200","{locale.t(\"submission.files.download_error\")}"}}
    }
}

#[cfg(target_arch = "wasm32")]
async fn download(file: &SubmissionAttachment) -> Result<(), ()> {
    use wasm_bindgen::JsCast;
    let response = gloo_net::http::Request::get(&format!(
        "/api/submissions/attachments/download?id={}",
        file.id
    ))
    .send()
    .await
    .map_err(|_| ())?;
    if !response.ok() {
        return Err(());
    }
    let data = response.binary().await.map_err(|_| ())?;
    let array = js_sys::Uint8Array::from(data.as_slice());
    let sequence = js_sys::Array::new();
    sequence.push(&array);
    let blob = web_sys::Blob::new_with_u8_array_sequence(&sequence).map_err(|_| ())?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|_| ())?;
    let result = (|| {
        let document = web_sys::window().and_then(|w| w.document()).ok_or(())?;
        let link = document.create_element("a").map_err(|_| ())?;
        link.set_attribute("href", &url).map_err(|_| ())?;
        link.set_attribute("download", &file.filename)
            .map_err(|_| ())?;
        link.dyn_into::<web_sys::HtmlElement>()
            .map_err(|_| ())?
            .click();
        Ok(())
    })();
    gloo_timers::future::TimeoutFuture::new(1000).await;
    web_sys::Url::revoke_object_url(&url).map_err(|_| ())?;
    result
}
