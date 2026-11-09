use anyhow::{Context, Result, ensure};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{
        IntoResponse, Response, Sse,
        sse::{Event, KeepAlive},
    },
    routing::{get, post},
};
use clap::Parser;
use claw_lm::{
    Assets, Decoder, GenerationOptions, GenerationResult, Mode,
    chat::{Chat, TextStream, parse_tool_calls, visible_text},
    cpu::CpuDecoder,
    generate,
};
use serde_json::{Value, json};
use std::{
    convert::Infallible,
    net::SocketAddr,
    path::PathBuf,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    task::{Context as TaskContext, Poll},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Semaphore, mpsc};
use tokio_stream::{Stream, wrappers::ReceiverStream};

#[derive(Parser)]
struct Args {
    #[arg(long, env = "MODEL_DIR")]
    model_dir: PathBuf,
    #[arg(long, default_value = "127.0.0.1:8093")]
    bind: SocketAddr,
    #[arg(long, default_value = "nemotron-labs-diffusion-3b")]
    model: String,
    #[arg(long, default_value = "cuda")]
    backend: String,
    #[arg(long)]
    device: Option<i32>,
    #[arg(long, default_value_t = 16384)]
    context: usize,
    #[arg(long, default_value_t = 1024)]
    max_output: usize,
    #[arg(long, default_value = "autoregressive")]
    mode: String,
}
#[derive(Clone)]
struct App {
    engine: Arc<Mutex<Box<dyn Decoder>>>,
    chat: Arc<Chat>,
    slot: Arc<Semaphore>,
    cancel: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    alias: String,
    context: usize,
    max_output: usize,
    mode: Mode,
    eos: u32,
    mask: u32,
    counter: Arc<AtomicU64>,
}
const NAMED_MODES: [(Mode, &str); 3] = [
    (Mode::Autoregressive, "autoregressive"),
    (Mode::Diffusion, "block-diffusion"),
    (Mode::SelfSpeculation, "self-speculation"),
];
fn selected_mode(app: &App, model: &str, explicit: Option<Mode>) -> Result<Mode> {
    if model == app.alias {
        return Ok(explicit.unwrap_or(app.mode));
    }
    for (mode, suffix) in NAMED_MODES {
        if model == format!("{}-{suffix}", app.alias) {
            ensure!(
                explicit.is_none_or(|m| m == mode),
                "Model ID conflicts with generation_mode"
            );
            return Ok(mode);
        }
    }
    anyhow::bail!("Unknown model; use GET /v1/models")
}
fn model_listing(app: &App) -> Value {
    let mut cards = vec![
        json!({"id":app.alias,"object":"model","owned_by":"local","context_window":app.context,"generation_mode":app.mode}),
    ];
    for (mode, suffix) in NAMED_MODES {
        cards.push(json!({"id":format!("{}-{suffix}",app.alias),"object":"model","owned_by":"local","context_window":app.context,"generation_mode":mode}));
    }
    json!({"object":"list","data":cards})
}
#[derive(Debug)]
struct ContextOverflow {
    prompt: usize,
    output: usize,
    limit: usize,
}
impl std::fmt::Display for ContextOverflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "This model's maximum context length is {} tokens; this request has {} prompt tokens and {} output tokens. Compact the conversation or reduce the requested output budget.",
            self.limit, self.prompt, self.output
        )
    }
}
impl std::error::Error for ContextOverflow {}
struct ApiError(StatusCode, String, &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.0,
            Json(json!({"error":{"message":self.1,"type":"invalid_request_error","code":self.2}})),
        )
            .into_response()
    }
}
fn invalid(e: anyhow::Error) -> ApiError {
    let code = if e.is::<ContextOverflow>() {
        "context_length_exceeded"
    } else {
        "invalid_request_error"
    };
    ApiError(StatusCode::BAD_REQUEST, e.to_string(), code)
}
fn internal(e: impl std::fmt::Display) -> ApiError {
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        e.to_string(),
        "internal_error",
    )
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}
struct ResponseStream {
    inner: ReceiverStream<std::result::Result<Event, Infallible>>,
    guard: CancelOnDrop,
}
impl Stream for ResponseStream {
    type Item = std::result::Result<Event, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let _ = &self.guard;
        Pin::new(&mut self.inner).poll_next(cx)
    }
}
struct Request {
    model: String,
    ids: Vec<u32>,
    tools: Vec<Value>,
    stops: Vec<String>,
    options: GenerationOptions,
    stream: bool,
    usage: bool,
    parallel: bool,
    id: String,
    created: u64,
}
fn prepare(app: &App, body: Value) -> Result<Request> {
    let object = body.as_object().context("Request must be an object")?;
    let allowed = [
        "model",
        "messages",
        "max_tokens",
        "max_completion_tokens",
        "temperature",
        "top_p",
        "stream",
        "stream_options",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "stop",
        "generation_mode",
        "block_length",
        "threshold",
        "n",
        "frequency_penalty",
        "presence_penalty",
    ];
    for key in object.keys() {
        ensure!(
            allowed.contains(&key.as_str()),
            "Unsupported request option: {key}"
        );
    }
    let model = body["model"].as_str().context("Model ID is required")?;
    let explicit_mode = body
        .get("generation_mode")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()?;
    let mode = selected_mode(app, model, explicit_mode)?;
    ensure!(
        body.get("n").is_none_or(|v| v.as_u64() == Some(1)),
        "Only n=1 is supported"
    );
    for key in ["temperature", "frequency_penalty", "presence_penalty"] {
        ensure!(
            body.get(key).is_none_or(|v| v.as_f64() == Some(0.)),
            "Only {key}=0 is currently supported"
        );
    }
    ensure!(
        body.get("top_p").is_none_or(|v| v.as_f64() == Some(1.)),
        "Only top_p=1 is currently supported"
    );
    ensure!(
        body.get("stream").is_none_or(Value::is_boolean),
        "stream must be boolean"
    );
    ensure!(
        body.get("parallel_tool_calls")
            .is_none_or(Value::is_boolean),
        "parallel_tool_calls must be boolean"
    );
    ensure!(
        !(body.get("max_tokens").is_some() && body.get("max_completion_tokens").is_some()),
        "Specify only one output budget"
    );
    let max = body
        .get("max_tokens")
        .or_else(|| body.get("max_completion_tokens"))
        .map(|v| {
            v.as_u64()
                .context("Output budget must be a nonnegative integer")
        })
        .transpose()?
        .unwrap_or(256) as usize;
    ensure!(
        max <= app.max_output,
        "Output budget exceeds server --max-output"
    );
    let block = body
        .get("block_length")
        .map(|v| v.as_u64().context("block_length must be an integer"))
        .transpose()?
        .unwrap_or(32) as usize;
    ensure!((1..=128).contains(&block), "block_length must be 1..128");
    let threshold = body
        .get("threshold")
        .map(|v| v.as_f64().context("threshold must be numeric"))
        .transpose()?
        .unwrap_or(0.9) as f32;
    ensure!(
        threshold.is_finite() && (0. ..=1.).contains(&threshold),
        "threshold must be between 0 and 1"
    );
    let messages = body["messages"]
        .as_array()
        .context("messages must be an array")?;
    let tools = body
        .get("tools")
        .map(|v| v.as_array().cloned().context("tools must be an array"))
        .transpose()?
        .unwrap_or_default();
    ensure!(
        body.get("tool_choice")
            .is_none_or(|v| v.as_str() == Some("auto") || v.as_str() == Some("none")),
        "Only tool_choice auto or none is supported"
    );
    let tools = if body["tool_choice"].as_str() == Some("none") {
        Vec::new()
    } else {
        tools
    };
    let text = app.chat.render(messages, &tools)?;
    let ids = app.chat.encode(&text)?;
    if !ids.len().checked_add(max).is_some_and(|n| n <= app.context) {
        return Err(ContextOverflow {
            prompt: ids.len(),
            output: max,
            limit: app.context,
        }
        .into());
    }
    let stops = match body.get("stop") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .context("stop entries must be strings")
            })
            .collect::<Result<Vec<_>>>()?,
        _ => anyhow::bail!("Invalid stop option"),
    };
    TextStream::new(stops.clone())?;
    let usage = if let Some(options) = body.get("stream_options") {
        let options = options
            .as_object()
            .context("stream_options must be an object")?;
        ensure!(
            options.keys().all(|k| k == "include_usage"),
            "Unsupported stream_options field"
        );
        ensure!(
            options.get("include_usage").is_none_or(Value::is_boolean),
            "include_usage must be boolean"
        );
        options
            .get("include_usage")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    } else {
        false
    };
    Ok(Request {
        model: model.to_string(),
        ids,
        tools,
        stops,
        options: GenerationOptions {
            mode,
            max_new_tokens: max,
            block_length: block,
            threshold: Some(threshold),
            temperature: 0.,
            eos_token_id: app.eos,
            mask_token_id: app.mask,
        },
        stream: body["stream"].as_bool().unwrap_or(false),
        usage,
        parallel: body
            .get("parallel_tool_calls")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        id: format!(
            "chatcmpl-claw-{}",
            app.counter.fetch_add(1, Ordering::Relaxed)
        ),
        created: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    })
}
fn chunk(_app: &App, r: &Request, delta: Value, finish: Value) -> Value {
    json!({"id":r.id,"object":"chat.completion.chunk","created":r.created,"model":r.model,"choices":[{"index":0,"delta":delta,"finish_reason":finish}]})
}
fn run(
    app: &App,
    r: &Request,
    cancel: &AtomicBool,
    mut send: impl FnMut(Value) -> Result<()>,
) -> Result<(GenerationResult, String, Vec<Value>)> {
    let mut model = app
        .engine
        .lock()
        .map_err(|_| anyhow::anyhow!("Model lock poisoned"))?;
    let mut decoder = app.chat.tokenizer.decode_stream(true);
    let mut text = String::new();
    let mut filter = TextStream::new(r.stops.clone())?;
    let tool_mode = !r.tools.is_empty();
    let result = generate(model.as_mut(), &r.ids, &r.options, cancel, |token| {
        if let Some(part) = decoder
            .step(token)
            .map_err(|e| anyhow::anyhow!("Streaming decoder: {e}"))?
        {
            text.push_str(&part);
            if !tool_mode {
                let (delta, stop) = filter.push(&part, false)?;
                if !delta.is_empty() {
                    send(chunk(app, r, json!({"content":delta}), Value::Null))?;
                }
                if stop {
                    return Ok(false);
                }
            }
        }
        Ok(!cancel.load(Ordering::Relaxed))
    })?;
    if tool_mode {
        let mut bounded = TextStream::new(r.stops.clone())?;
        let (all, _) = bounded.push(&text, true)?;
        let (content, calls) = parse_tool_calls(&all, &r.tools, &r.id)?;
        ensure!(
            r.parallel || calls.len() <= 1,
            "Model emitted multiple calls while parallel_tool_calls=false"
        );
        if !content.is_empty() {
            send(chunk(app, r, json!({"content":content}), Value::Null))?;
        }
        for (index, call) in calls.iter().enumerate() {
            let mut call = call.clone();
            call["index"] = json!(index);
            send(chunk(app, r, json!({"tool_calls":[call]}), Value::Null))?;
        }
        Ok((result, content, calls))
    } else {
        let (tail, _) = filter.push("", true)?;
        if !tail.is_empty() {
            send(chunk(app, r, json!({"content":tail}), Value::Null))?;
        }
        let mut full = TextStream::new(r.stops.clone())?;
        let (content, _) = full.push(&visible_text(&text), true)?;
        Ok((result, content, Vec::new()))
    }
}
async fn completions(
    State(app): State<App>,
    Json(body): Json<Value>,
) -> std::result::Result<Response, ApiError> {
    let request = prepare(&app, body).map_err(invalid)?;
    let permit = app.slot.clone().try_acquire_owned().map_err(|_| {
        ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Model is busy; one request is admitted at a time".into(),
            "rate_limit_error",
        )
    })?;
    let cancel = Arc::new(AtomicBool::new(false));
    *app.cancel.lock().map_err(internal)? = Some(cancel.clone());
    if request.stream {
        let (tx, rx) = mpsc::channel(16);
        let guard = CancelOnDrop(cancel.clone());
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let send = |value: Value| -> Result<()> {
                tx.blocking_send(Ok(Event::default().data(serde_json::to_string(&value)?)))
                    .map_err(|_| {
                        cancel.store(true, Ordering::Relaxed);
                        anyhow::anyhow!("Client disconnected")
                    })?;
                Ok(())
            };
            let result = (|| -> Result<()> {
                send(chunk(
                    &app,
                    &request,
                    json!({"role":"assistant","content":""}),
                    Value::Null,
                ))?;
                let (result, _, calls) = run(&app, &request, &cancel, &send)?;
                let reason = if calls.is_empty() {
                    result.finish_reason.as_str()
                } else {
                    "tool_calls"
                };
                send(chunk(&app, &request, json!({}), json!(reason)))?;
                if request.usage {
                    send(
                        json!({"id":request.id,"object":"chat.completion.chunk","created":request.created,"model":request.model,"choices":[],"usage":{"prompt_tokens":request.ids.len(),"completion_tokens":result.tokens.len(),"total_tokens":request.ids.len()+result.tokens.len()},"claw":{"generation_mode":result.mode,"statistics":result.statistics}}),
                    )?;
                }
                Ok(())
            })();
            if let Err(e) = result {
                let _ = send(json!({"error":{"message":e.to_string(),"type":"generation_error"}}));
            }
            let _ = tx.blocking_send(Ok(Event::default().data("[DONE]")));
            if let Ok(mut active) = app.cancel.lock() {
                *active = None;
            }
        });
        Ok(Sse::new(ResponseStream {
            inner: ReceiverStream::new(rx),
            guard,
        })
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
        .into_response())
    } else {
        let _guard = CancelOnDrop(cancel.clone());
        let response=tokio::task::spawn_blocking(move||->Result<Value>{
            let _permit=permit;let result=run(&app,&request,&cancel,|_|Ok(()));
            if let Ok(mut active)=app.cancel.lock(){*active=None;}
            let (result,content,calls)=result?;let reason=if calls.is_empty(){result.finish_reason.as_str()}else{"tool_calls"};let mut message=json!({"role":"assistant","content":content});if !calls.is_empty(){message["tool_calls"]=json!(calls);}
            Ok(json!({"id":request.id,"object":"chat.completion","created":request.created,"model":request.model,"choices":[{"index":0,"message":message,"finish_reason":reason}],"usage":{"prompt_tokens":request.ids.len(),"completion_tokens":result.tokens.len(),"total_tokens":request.ids.len()+result.tokens.len()},"claw":{"generation_mode":result.mode,"statistics":result.statistics}}))
        }).await.map_err(internal)?.map_err(internal)?;
        Ok(Json(response).into_response())
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let chat = Arc::new(Chat::load(&args.model_dir)?);
    let (engine, eos, mask) = {
        let assets = Assets::open(&args.model_dir)?;
        let eos = assets.config.eos_token_id;
        let mask = assets.config.mask_token_id;
        let engine: Box<dyn Decoder> = match args.backend.as_str() {
            "cpu" => Box::new(CpuDecoder::new(assets, args.context)?),
            "cuda" => {
                #[cfg(feature = "cuda")]
                {
                    Box::new(claw_lm::cuda::CudaDecoder::new(
                        &assets,
                        args.device.context("--device is required for CUDA")?,
                        args.context,
                    )?)
                }
                #[cfg(not(feature = "cuda"))]
                {
                    anyhow::bail!("Build server with --features cuda");
                }
            }
            _ => anyhow::bail!("Unknown backend"),
        };
        (engine, eos, mask)
    };
    let app = App {
        engine: Arc::new(Mutex::new(engine)),
        chat,
        slot: Arc::new(Semaphore::new(1)),
        cancel: Arc::new(Mutex::new(None)),
        alias: args.model.clone(),
        context: args.context,
        max_output: args.max_output,
        mode: serde_json::from_value(json!(args.mode))?,
        eos,
        mask,
        counter: Arc::new(AtomicU64::new(1)),
    };
    let models = app.clone();
    let health = app.clone();
    let shutdown = app.clone();
    let router=Router::new().route("/health",get(move||{let a=health.clone();async move{Json(json!({"status":"ok","model":a.alias,"generation_modes":["autoregressive","diffusion","self-speculation"]}))}}))
        .route("/v1/models",get(move||{let a=models.clone();async move{Json(model_listing(&a))}}))
        .route("/v1/chat/completions",post(completions)).layer(DefaultBodyLimit::max(4*1024*1024)).with_state(app);
    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    eprintln!(
        "claw-lm ready at http://{}; model={}",
        args.bind, args.model
    );
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            if let Ok(active) = shutdown.cancel.lock()
                && let Some(cancel) = active.as_ref()
            {
                cancel.store(true, Ordering::Relaxed);
            }
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request as HttpRequest,
    };
    use tower::ServiceExt;
    struct Fake {
        prefix: usize,
        forwards: Arc<AtomicU64>,
    }
    impl Decoder for Fake {
        fn vocab_size(&self) -> usize {
            8
        }
        fn context_limit(&self) -> usize {
            256
        }
        fn cache_len(&self) -> usize {
            self.prefix
        }
        fn reset(&mut self) -> Result<()> {
            self.prefix = 0;
            Ok(())
        }
        fn crop(&mut self, n: usize) -> Result<()> {
            ensure!(n <= self.prefix, "Invalid crop");
            self.prefix = n;
            Ok(())
        }
        fn forward(
            &mut self,
            ids: &[u32],
            _: claw_lm::generation::Attention,
            commit: bool,
            last: bool,
        ) -> Result<Vec<Vec<f32>>> {
            self.forwards.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(2));
            if commit {
                self.prefix += ids.len();
            }
            let mut row = vec![-10.; 8];
            row[2] = 10.;
            Ok(vec![row; if last { 1 } else { ids.len() }])
        }
    }
    fn app() -> App {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("tokenizer.json"),serde_json::to_vec(&json!({"version":"1.0","truncation":null,"padding":null,"added_tokens":[],"normalizer":null,"pre_tokenizer":{"type":"Whitespace"},"post_processor":null,"decoder":null,"model":{"type":"WordLevel","vocab":{"<unk>":0,"hello":1,"X":2},"unk_token":"<unk>"}})).unwrap()).unwrap();
        std::fs::write(
            dir.path().join("chat_template.jinja"),
            "{% for m in messages %}{{ m.content }}{% endfor %}",
        )
        .unwrap();
        App {
            engine: Arc::new(Mutex::new(Box::new(Fake {
                prefix: 0,
                forwards: Arc::new(AtomicU64::new(0)),
            }))),
            chat: Arc::new(Chat::load(dir.path()).unwrap()),
            slot: Arc::new(Semaphore::new(1)),
            cancel: Arc::new(Mutex::new(None)),
            alias: "test".into(),
            context: 256,
            max_output: 128,
            mode: Mode::Autoregressive,
            eos: 6,
            mask: 7,
            counter: Arc::new(AtomicU64::new(1)),
        }
    }
    fn router(a: App) -> Router {
        Router::new()
            .route("/v1/chat/completions", post(completions))
            .with_state(a)
    }
    fn body(stream: bool) -> Value {
        json!({"model":"test","messages":[{"role":"user","content":"hello"}],"max_tokens":3,"stream":stream,"stream_options":{"include_usage":true}})
    }
    fn request(value: Value) -> HttpRequest<Body> {
        HttpRequest::post("/v1/chat/completions")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&value).unwrap()))
            .unwrap()
    }
    #[tokio::test]
    async fn named_models_select_modes_and_echo_identity() {
        let a = app();
        let listing = model_listing(&a);
        let cards = listing["data"].as_array().unwrap();
        assert_eq!(cards.len(), 4);
        assert!(cards.iter().any(|c| c["id"] == "test"));
        for (suffix, mode) in [
            ("autoregressive", "autoregressive"),
            ("block-diffusion", "diffusion"),
            ("self-speculation", "self-speculation"),
        ] {
            let id = format!("test-{suffix}");
            assert!(
                cards
                    .iter()
                    .any(|c| c["id"] == id && c["context_window"] == 256)
            );
            for stream in [false, true] {
                let mut input = body(stream);
                input["model"] = json!(id);
                let response = router(a.clone()).oneshot(request(input)).await.unwrap();
                assert_eq!(response.status(), StatusCode::OK);
                let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
                if stream {
                    let events = String::from_utf8(bytes.to_vec()).unwrap();
                    assert!(events.contains("[DONE]"));
                    let mut reported_mode = false;
                    for line in events.lines().filter_map(|l| l.strip_prefix("data: ")) {
                        if line == "[DONE]" {
                            continue;
                        }
                        let event: Value = serde_json::from_str(line).unwrap();
                        assert_eq!(event["model"], id);
                        if event["claw"].is_object() {
                            assert_eq!(event["claw"]["generation_mode"], mode);
                            reported_mode = true;
                        }
                    }
                    assert!(reported_mode);
                } else {
                    let result: Value = serde_json::from_slice(&bytes).unwrap();
                    assert_eq!(result["model"], id);
                    assert_eq!(result["claw"]["generation_mode"], mode);
                    assert_eq!(result["choices"][0]["message"]["content"], "X X X");
                }
            }
        }
        let mut conflict = body(false);
        conflict["model"] = json!("test-self-speculation");
        conflict["generation_mode"] = json!("diffusion");
        let response = router(a).oneshot(request(conflict)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn context_overflow_is_recognizable_and_boundary_is_exact() {
        let mut a = app();
        a.context = 4;
        // One prompt token plus three requested output tokens exactly fits.
        let response = router(a.clone())
            .oneshot(request(body(false)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        for stream in [false, true] {
            let mut overflow = body(stream);
            overflow["max_tokens"] = json!(4);
            let response = router(a.clone()).oneshot(request(overflow)).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
            let error: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(error["error"]["code"], "context_length_exceeded");
            let message = error["error"]["message"].as_str().unwrap();
            assert!(message.contains("maximum context length is 4 tokens"));
            assert!(message.contains("1 prompt tokens"));
            assert!(message.contains("4 output tokens"));
            assert_eq!(a.slot.available_permits(), 1);
        }
    }
    #[tokio::test]
    async fn all_modes_through_http_and_sse() {
        for mode in ["autoregressive", "diffusion", "self-speculation"] {
            let a = app();
            let mut input = body(false);
            input["generation_mode"] = json!(mode);
            let response = router(a.clone())
                .oneshot(request(input.clone()))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
            let result: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(result["choices"][0]["message"]["content"], "X X X");
            assert_eq!(result["claw"]["generation_mode"], mode);
            input["stream"] = json!(true);
            let response = router(a).oneshot(request(input)).await.unwrap();
            let bytes = to_bytes(response.into_body(), 1_048_576).await.unwrap();
            let events = String::from_utf8(bytes.to_vec()).unwrap();
            assert!(events.contains("[DONE]"));
            let mut text = String::new();
            let mut usage = false;
            for line in events.lines().filter_map(|l| l.strip_prefix("data: ")) {
                if line == "[DONE]" {
                    continue;
                }
                let v: Value = serde_json::from_str(line).unwrap();
                if let Some(delta) = v["choices"][0]["delta"]["content"].as_str() {
                    text.push_str(delta);
                }
                if v["usage"].is_object() {
                    usage = true;
                }
            }
            assert_eq!(text, "X X X");
            assert!(usage);
        }
    }
    #[tokio::test]
    async fn text_blocks_are_supported_and_images_rejected() {
        let a = app();
        let mut b = body(false);
        b["messages"][0]["content"] = json!([{"type":"text","text":"hello"}]);
        let response = router(a.clone()).oneshot(request(b.clone())).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        b["messages"][0]["content"] = json!([{"type":"image_url","image_url":{"url":"synthetic"}}]);
        let response = router(a).oneshot(request(b)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    #[tokio::test]
    async fn invalid_options_and_bounded_admission() {
        let a = app();
        for (key, value) in [
            ("model", json!("unknown")),
            ("temperature", json!(0.5)),
            ("images", json!([])),
            ("generation_mode", json!("bogus")),
            ("max_tokens", json!(5000)),
            ("threshold", json!(-1)),
        ] {
            let mut b = body(false);
            b[key] = value;
            let response = router(a.clone()).oneshot(request(b)).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        let permit = a.slot.clone().try_acquire_owned().unwrap();
        let response = router(a.clone())
            .oneshot(request(body(false)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        drop(permit);
        assert_eq!(a.slot.available_permits(), 1);
    }
    #[tokio::test]
    async fn disconnected_stream_releases_admission() {
        let a = app();
        let mut b = body(true);
        b["max_tokens"] = json!(128);
        let response = router(a.clone()).oneshot(request(b)).await.unwrap();
        drop(response);
        for _ in 0..100 {
            if a.slot.available_permits() == 1 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("Cancelled generation retained admission");
    }
}
