use super::*;

#[derive(Default)]
pub(super) struct Registry {
    pub(super) next_id: u32,
    pub(super) apps: HashMap<u32, NativeRuntime>,
}

thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry {
        next_id: 1,
        apps: HashMap::new(),
    });
}

pub(super) fn with_app_mut<T>(
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> Result<T> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let runtime = registry
            .apps
            .get_mut(&app)
            .ok_or_else(|| Error::from_reason(format!("unknown QuickGUI app {app}")))?;
        callback(runtime).map_err(Error::from_reason)
    })
}

#[napi]
pub fn create_app(options: Option<NativeAppOptions>) -> Result<u32> {
    REGISTRY.with(|registry| {
        let mut registry = registry.borrow_mut();
        let id = registry.next_id.max(1);
        registry.next_id = id
            .checked_add(1)
            .ok_or_else(|| Error::from_reason("QuickGUI app id space exhausted"))?;
        let runtime =
            NativeRuntime::new(options.unwrap_or_default()).map_err(Error::from_reason)?;
        registry.apps.insert(id, runtime);
        Ok(id)
    })
}

#[napi]
pub fn create_window(
    app: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_window(
            options.unwrap_or_default(),
            initial_batch.as_deref().unwrap_or_default(),
        )
    })
}

#[napi]
pub fn create_system_popover(
    app: u32,
    parent: u32,
    anchor: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_system_popover(
            parent,
            anchor,
            options.unwrap_or_default(),
            initial_batch.as_deref().unwrap_or_default(),
        )
    })
}

#[cfg(target_os = "macos")]
#[napi]
pub fn create_embedded_view(
    app: u32,
    parent: u32,
    match_horizontal: bool,
    match_vertical: bool,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    with_app_mut(app, |runtime| {
        runtime.create_embedded_view(
            parent,
            match_horizontal,
            match_vertical,
            options.unwrap_or_default(),
            initial_batch.as_deref().unwrap_or_default(),
        )
    })
}

#[napi]
pub fn apply_batch(app: u32, window: u32, batch: Buffer) -> Result<u32> {
    with_app_mut(app, |runtime| runtime.apply_batch(window, &batch))
}

#[napi]
pub fn close_window(app: u32, window: u32) -> Result<bool> {
    with_app_mut(app, |runtime| Ok(runtime.close_window(window)))
}

#[napi]
pub fn focus_node(app: u32, window: u32, node: u32) -> Result<bool> {
    with_app_mut(app, |runtime| runtime.focus_node(window, node))
}

#[napi]
pub fn show_alert_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeDialogOptions,
) -> Result<()> {
    with_app_mut(app, |runtime| {
        runtime.show_alert_dialog(window, request, options)
    })
}

#[napi]
pub fn show_open_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeOpenDialogOptions,
) -> Result<()> {
    with_app_mut(app, |runtime| {
        runtime.show_open_dialog(window, request, options)
    })
}

#[napi]
pub fn show_save_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeSaveDialogOptions,
) -> Result<()> {
    with_app_mut(app, |runtime| {
        runtime.show_save_dialog(window, request, options)
    })
}

#[napi]
pub fn start_app(app: u32) -> Result<()> {
    prepare_app(app)
}

#[napi]
pub fn prepare_app(app: u32) -> Result<()> {
    with_app_mut(app, NativeRuntime::prepare)
}

#[napi]
pub fn is_app_ready(app: u32) -> Result<bool> {
    with_app_mut(app, |runtime| Ok(runtime.is_ready()))
}

/// Return `-1` while running or the non-negative native exit code after termination.
#[napi]
pub fn pump_app(app: u32, timeout_ms: Option<f64>) -> Result<i32> {
    with_app_mut(app, |runtime| {
        let timeout = timeout_ms
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(16.0)
            .min(1_000.0);
        let status = runtime
            .runner
            .as_mut()
            .ok_or_else(|| "prepareApp must be called before pumpApp".to_owned())?
            .pump(Some(Duration::from_secs_f64(timeout / 1_000.0)))
            .map_err(|error| error.to_string())?;
        runtime.sync_closed_windows();
        match status {
            AppRunStatus::Continue => Ok(-1),
            AppRunStatus::Exited(code) => Ok(code.max(0)),
        }
    })
}

#[napi]
pub fn take_events(app: u32) -> Result<Vec<NativeEvent>> {
    with_app_mut(app, |runtime| Ok(runtime.drain_events()))
}

#[napi]
pub fn destroy_app(app: u32) -> Result<bool> {
    REGISTRY.with(|registry| Ok(registry.borrow_mut().apps.remove(&app).is_some()))
}

#[napi]
pub fn create_hosted_app(options: Option<NativeAppOptions>) -> Result<u32> {
    let app = HOST.allocate_app().map_err(Error::from_reason)?;
    HOST.set_app(app).map_err(Error::from_reason)?;
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateApp {
        app,
        options: options.unwrap_or_default(),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)?;
    Ok(app)
}

#[napi]
pub fn create_hosted_window(
    app: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateWindow {
        app,
        options: options.unwrap_or_default(),
        initial_batch: initial_batch.map_or_else(Vec::new, |batch| batch.to_vec()),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn create_hosted_system_popover(
    app: u32,
    parent: u32,
    anchor: u32,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateSystemPopover {
        app,
        parent,
        anchor,
        options: options.unwrap_or_default(),
        initial_batch: initial_batch.map_or_else(Vec::new, |batch| batch.to_vec()),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[cfg(target_os = "macos")]
#[napi]
pub fn create_hosted_embedded_view(
    app: u32,
    parent: u32,
    match_horizontal: bool,
    match_vertical: bool,
    options: Option<NativeWindowOptions>,
    initial_batch: Option<Buffer>,
) -> Result<u32> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CreateEmbeddedView {
        app,
        parent,
        match_horizontal,
        match_vertical,
        options: options.unwrap_or_default(),
        initial_batch: initial_batch.map_or_else(Vec::new, |batch| batch.to_vec()),
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn apply_hosted_batch(app: u32, window: u32, batch: Buffer) -> Result<u32> {
    if batch.len() > MAX_BATCH_BYTES {
        return Err(Error::from_reason(format!(
            "mutation batch exceeds {MAX_BATCH_BYTES} bytes"
        )));
    }
    HOST.enqueue(HostCommand::ApplyBatch {
        app,
        window,
        batch: batch.to_vec(),
    })
    .map_err(Error::from_reason)?;
    Ok(0)
}

#[napi]
pub fn close_hosted_window(app: u32, window: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::CloseWindow {
        app,
        window,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn focus_hosted_node(app: u32, window: u32, node: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::FocusNode {
        app,
        window,
        node,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn show_hosted_alert_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeDialogOptions,
) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::ShowAlertDialog {
        app,
        window,
        request,
        options,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn show_hosted_open_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeOpenDialogOptions,
) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::ShowOpenDialog {
        app,
        window,
        request,
        options,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn show_hosted_save_dialog(
    app: u32,
    window: Option<u32>,
    request: u32,
    options: NativeSaveDialogOptions,
) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::ShowSaveDialog {
        app,
        window,
        request,
        options,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn start_hosted_app(app: u32) -> Result<()> {
    prepare_hosted_app(app)
}

#[napi]
pub fn prepare_hosted_app(app: u32) -> Result<()> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::PrepareApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn is_hosted_app_ready(app: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::IsAppReady {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[napi]
pub fn destroy_hosted_app(app: u32) -> Result<bool> {
    let reply = Arc::new(SyncReply::new());
    HOST.enqueue(HostCommand::DestroyApp {
        app,
        reply: Arc::clone(&reply),
    })
    .map_err(Error::from_reason)?;
    reply.wait().map_err(Error::from_reason)
}

#[doc(hidden)]
pub struct WaitForHostedEvents {
    app: u32,
}

impl Task for WaitForHostedEvents {
    type Output = HostedAppUpdate;
    type JsValue = HostedAppUpdate;

    fn compute(&mut self) -> Result<Self::Output> {
        HOST.wait_for_update(self.app).map_err(Error::from_reason)
    }

    fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
        Ok(output)
    }
}

#[napi(ts_return_type = "Promise<HostedAppUpdate>")]
pub fn wait_for_hosted_events(app: u32) -> AsyncTask<WaitForHostedEvents> {
    AsyncTask::new(WaitForHostedEvents { app })
}

#[napi]
pub fn abort_app_host(message: String) {
    HOST.fail(message);
}

#[napi]
pub fn run_app_host(on_ready: Option<Function<'_, (), ()>>) -> Result<i32> {
    HOST.begin().map_err(Error::from_reason)?;
    let result = run_app_host_loop(on_ready.as_ref());
    if let Err(error) = &result {
        HOST.fail(error.clone());
    }
    HOST.finish();
    result.map_err(Error::from_reason)
}

pub(super) fn run_app_host_loop(
    on_ready: Option<&Function<'_, (), ()>>,
) -> std::result::Result<i32, String> {
    let mut active_app = None;
    let mut runtime: Option<NativeRuntime> = None;
    let mut ready_reported = false;

    loop {
        let running = runtime
            .as_ref()
            .is_some_and(|runtime| runtime.runner.is_some());
        let mut commands = if running {
            HOST.take_commands()?
        } else {
            HOST.wait_for_commands()?
        };

        while let Some(command) = commands.pop_front() {
            match command {
                HostCommand::CreateApp {
                    app,
                    options,
                    reply,
                } => {
                    let result = if runtime.is_some() {
                        Err("a QuickGUI native host can own only one app".to_owned())
                    } else {
                        match NativeRuntime::new(options) {
                            Ok(created) => {
                                active_app = Some(app);
                                runtime = Some(created);
                                Ok(())
                            }
                            Err(error) => Err(error),
                        }
                    };
                    reply.complete(result);
                }
                HostCommand::CreateWindow {
                    app,
                    options,
                    initial_batch,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.create_window(options, &initial_batch),
                    ));
                }
                HostCommand::CreateSystemPopover {
                    app,
                    parent,
                    anchor,
                    options,
                    initial_batch,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| {
                            runtime.create_system_popover(parent, anchor, options, &initial_batch)
                        },
                    ));
                }
                #[cfg(target_os = "macos")]
                HostCommand::CreateEmbeddedView {
                    app,
                    parent,
                    match_horizontal,
                    match_vertical,
                    options,
                    initial_batch,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| {
                            runtime.create_embedded_view(
                                parent,
                                match_horizontal,
                                match_vertical,
                                options,
                                &initial_batch,
                            )
                        },
                    ));
                }
                HostCommand::ApplyBatch { app, window, batch } => {
                    with_hosted_runtime(active_app, runtime.as_mut(), app, |runtime| {
                        runtime.apply_batch(window, &batch)
                    })?;
                }
                HostCommand::CloseWindow { app, window, reply } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| Ok(runtime.close_window(window)),
                    ));
                }
                HostCommand::FocusNode {
                    app,
                    window,
                    node,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.focus_node(window, node),
                    ));
                }
                HostCommand::ShowAlertDialog {
                    app,
                    window,
                    request,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.show_alert_dialog(window, request, options),
                    ));
                }
                HostCommand::ShowOpenDialog {
                    app,
                    window,
                    request,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.show_open_dialog(window, request, options),
                    ));
                }
                HostCommand::ShowSaveDialog {
                    app,
                    window,
                    request,
                    options,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.show_save_dialog(window, request, options),
                    ));
                }
                HostCommand::System {
                    app,
                    command,
                    reply,
                } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| runtime.execute_system_command(command),
                    ));
                }
                HostCommand::PrepareApp { app, reply } => {
                    let result = with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        NativeRuntime::prepare,
                    );
                    if result.is_ok() {
                        let waker = runtime
                            .as_ref()
                            .and_then(|runtime| runtime.runner.as_ref())
                            .expect("a started QuickGUI host must own an AppRunner")
                            .waker();
                        HOST.set_waker(waker);
                    }
                    reply.complete(result);
                }
                HostCommand::IsAppReady { app, reply } => {
                    reply.complete(with_hosted_runtime(
                        active_app,
                        runtime.as_mut(),
                        app,
                        |runtime| Ok(runtime.is_ready()),
                    ));
                }
                HostCommand::DestroyApp { app, reply } => {
                    let result = if active_app == Some(app) && runtime.is_some() {
                        HOST.publish_exit(0);
                        Ok(true)
                    } else {
                        Ok(false)
                    };
                    reply.complete(result);
                    return Ok(0);
                }
            }
        }

        let Some(runtime) = runtime.as_mut() else {
            continue;
        };
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());

        let Some(runner) = runtime.runner.as_mut() else {
            continue;
        };
        let status = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runner.pump(None)))
            .map_err(|payload| {
                format!(
                    "QuickGUI event loop panicked: {}",
                    panic_payload_message(payload.as_ref())
                )
            })?
            .map_err(|error| error.to_string())?;
        if !ready_reported {
            if let Some(on_ready) = on_ready {
                on_ready.call(()).map_err(|error| error.to_string())?;
            }
            ready_reported = true;
        }
        runtime.sync_closed_windows();
        HOST.publish_events(runtime.drain_events());
        if let AppRunStatus::Exited(code) = status {
            let code = code.max(0);
            HOST.publish_exit(code);
            return Ok(code);
        }
    }
}

pub(super) fn panic_payload_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .unwrap_or("unknown panic payload")
}

pub(super) fn with_hosted_runtime<T>(
    active_app: Option<u32>,
    runtime: Option<&mut NativeRuntime>,
    app: u32,
    callback: impl FnOnce(&mut NativeRuntime) -> std::result::Result<T, String>,
) -> std::result::Result<T, String> {
    if active_app != Some(app) {
        return Err(format!("unknown QuickGUI hosted app {app}"));
    }
    callback(runtime.ok_or_else(|| format!("unknown QuickGUI hosted app {app}"))?)
}

#[napi]
pub fn protocol_version() -> u32 {
    PROTOCOL_VERSION as u32
}

pub(super) fn finite_number(value: Option<f64>) -> Option<f32> {
    value
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(f32::MIN as f64, f32::MAX as f64) as f32)
}

pub(super) fn finite_dimension(value: Option<f64>, fallback: f32) -> f32 {
    finite_number(value)
        .filter(|value| *value > 0.0)
        .unwrap_or(fallback)
}
