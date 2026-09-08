///|
pub fn field(value : Json, key : String) -> Json {
  if value is Object(fields) {
    fields.get(key).unwrap_or(Json::null())
  } else {
    Json::null()
  }
}

///|
pub fn string(value : Json) -> String {
  if value is String(s) {
    s
  } else {
    ""
  }
}

///|
pub fn number(value : Json) -> Int {
  if value is Number(n, ..) {
    n.to_int()
  } else {
    0
  }
}

///|
pub fn boolean(value : Json) -> Bool {
  value is True
}

///|
pub fn strings(value : Json) -> Array[String] {
  if value is Array(items) {
    items.map(string)
  } else {
    []
  }
}

///|
pub fn array(value : Json) -> Array[Json] {
  if value is Array(items) {
    items
  } else {
    []
  }
}

///|
pub fn service(
  operation : String,
  params : Json,
  done : (Result[Json, String]) -> Unit,
) -> Unit {
  @native.invoke("extension/quick-git-io/" + operation, params, done)
}

///|
pub(all) struct Output {
  stdout : String
  stderr : String
  code : Int
  cancelled : Bool
  truncated : Bool
} derive(Eq, Debug)

///|
priv struct Job {
  owner : Int
  options : Json
  out : @buffer.Buffer
  err : @buffer.Buffer
  mut done : (Result[Output, String]) -> Unit
  mut cancelled : Bool
  mut running : Bool
  mut cleanup : () -> Unit
}

///|
let jobs : Map[Int, Job] = Map([])

///|
let queue : Array[Int] = []

///|
let active : @ref.Ref[Int] = @ref.Ref(0)

///|
let sequence : @ref.Ref[Int] = @ref.Ref(1)

///|
let application_owner : @ref.Ref[Int] = @ref.Ref(0)

///|
let idle_callbacks : Array[(Int, () -> Unit)] = []

///|
/// An event-driven barrier for repository operations; no timer or polling loop.
pub fn after_jobs(callback : () -> Unit) -> Unit {
  if active.val == 0 && queue.is_empty() {
    callback()
  } else {
    idle_callbacks.push((@reactive.owner(), callback))
  }
}

///|
pub fn start() -> Unit {
  application_owner.val = @reactive.owner()
  @native.on_event(
    "extension-event",
    event => {
      let data = event.json()
      let id = number(field(data, "job"))
      if jobs.get(id) is Some(job) && !job.cancelled {
        let bytes = @base64.decode(string(field(data, "bytes"))) catch {
          _ => b""
        }
        let buffer = if string(field(data, "stream")) == "stderr" {
          job.err
        } else {
          job.out
        }
        if buffer.length() + bytes.length() <= 32 * 1024 * 1024 {
          buffer.write_bytes(bytes)
        }
      }
    },
    window=0,
  )
}

///|
fn pump() -> Unit {
  while active.val < 4 && !queue.is_empty() {
    let id = queue.remove(0)
    if jobs.get(id) is Some(job) {
      if job.cancelled {
        jobs.remove(id)
        continue
      }
      job.running = true
      active.val += 1
      @reactive.with_owner(application_owner.val, () => {
        service("run", job.options, result => {
          active.val -= 1
          jobs.remove(id)
          (job.cleanup)()
          let done = job.done
          job.done = _ => ()
          if !job.cancelled && @reactive.owner_alive(job.owner) {
            let output = match result {
              Err(error) => Err(error)
              Ok(value) =>
                if boolean(field(value, "timedOut")) {
                  Err("Command timed out")
                } else {
                  Ok({
                    stdout: @utf8.decode_lossy(job.out.to_bytes()),
                    stderr: @utf8.decode_lossy(job.err.to_bytes()),
                    code: number(field(value, "code")),
                    cancelled: boolean(field(value, "cancelled")),
                    truncated: boolean(field(value, "truncated")),
                  })
                }
            }
            @reactive.with_owner(job.owner, () => done(output))
          }
          job.out.reset()
          job.err.reset()
          pump()
        })
      })
    }
  }
  if active.val == 0 && queue.is_empty() && !idle_callbacks.is_empty() {
    let callbacks = idle_callbacks.copy()
    idle_callbacks.clear()
    for pair in callbacks {
      if @reactive.owner_alive(pair.0) {
        @reactive.with_owner(pair.0, pair.1)
      }
    }
  }
}

///|
pub fn process(
  program : String,
  args : Array[String],
  cwd : String,
  done : (Result[Output, String]) -> Unit,
  input? : String = "",
  input_file? : String = "",
  timeout? : Int = 60000,
) -> () -> Unit {
  if jobs.length() >= 128 {
    done(Err("Too many pending commands"))
    return () => ()
  }
  let id = sequence.val
  sequence.val += 1
  let job : Job = {
    owner: @reactive.owner(),
    out: @buffer.Buffer(),
    err: @buffer.Buffer(),
    done,
    cancelled: false,
    running: false,
    cleanup: () => (),
    options: {
      "id": id,
      "program": program,
      "args": args,
      "cwd": cwd,
      "input": input,
      "inputFile": input_file,
      "timeoutMs": timeout,
      "env": {
        "GIT_TERMINAL_PROMPT": "0",
        "GIT_PAGER": "cat",
        "GIT_EDITOR": "true",
        "LC_ALL": "C",
      },
    },
  }
  let cancel = () => {
    if !job.cancelled {
      job.cancelled = true
      job.done = _ => ()
      if job.running {
        service("cancel", { "id": id }, _ => ())
      } else {
        jobs.remove(id)
        if queue.search_by(queued => queued == id) is Some(index) {
          ignore(queue.remove(index))
        }
      }
    }
  }
  job.cleanup = @reactive.on_cleanup_cancelable(cancel)
  jobs[id] = job
  queue.push(id)
  pump()
  () => {
    cancel()
    (job.cleanup)()
  }
}

///|
pub fn cancel_pending() -> Unit {
  let owner = @reactive.owner()
  for id, job in jobs.copy() {
    if job.owner == owner {
      job.cancelled = true
      job.done = _ => ()
      (job.cleanup)()
      if job.running {
        service("cancel", { "id": id }, _ => ())
      } else {
        jobs.remove(id)
        if queue.search_by(queued => queued == id) is Some(index) {
          ignore(queue.remove(index))
        }
      }
    }
  }
}

///|
pub fn git(
  cwd : String,
  args : Array[String],
  done : (Result[String, String]) -> Unit,
  input? : String = "",
  input_file? : String = "",
  timeout? : Int = 60000,
  allow_one? : Bool = false,
) -> () -> Unit {
  if @utf8.encode(input).length() > 16000 {
    let stopped = @ref.Ref(false)
    let cancel : @ref.Ref[() -> Unit] = @ref.Ref(() => ())
    temporary(
      input,
      (path, _, release) => {
        if stopped.val {
          release()
          return
        }
        let stop = git(
          cwd,
          args,
          result => {
            release()
            done(result)
          },
          input_file=path,
          timeout~,
          allow_one~,
        )
        cancel.val = () => {
          stop()
          release()
        }
      },
      error => if !stopped.val { done(Err(error)) },
    )
    return () => {
      stopped.val = true
      (cancel.val)()
    }
  }
  process(
    "git",
    ["-c", "color.ui=false", "-c", "core.quotepath=false", ..args],
    cwd,
    result => {
      done(
        match result {
          Err(error) => Err(error)
          Ok(output) =>
            if output.truncated {
              Err("Git output exceeded 32 MiB; narrow the selection")
            } else if output.code != 0 && !(allow_one && output.code == 1) {
              Err(
                if output.stderr.trim().is_empty() {
                  "Git exited with status \{output.code}"
                } else {
                  output.stderr.trim().to_owned()
                },
              )
            } else {
              Ok(output.stdout)
            }
        },
      )
    },
    input~,
    input_file~,
    timeout~,
  )
}

///|
/// Stage a large input without exceeding the extension's 64 KiB message limit.
/// Temporary files retire on completion, errors, and owner disposal.
pub fn temporary(
  text : String,
  consume : (String, String, () -> Unit) -> Unit,
  failed : (String) -> Unit,
) -> Unit {
  let owner = @reactive.owner()
  @reactive.with_owner(application_owner.val, () => {
    service("temp", {}, result => {
      match result {
        Err(error) =>
          if @reactive.owner_alive(owner) {
            @reactive.with_owner(owner, () => failed(error))
          }
        Ok(value) => {
          let dir = string(value)
          let removed = @ref.Ref(false)
          let cancel_cleanup : @ref.Ref[() -> Unit] = @ref.Ref(() => ())
          let release = () => {
            if !removed.val {
              removed.val = true
              (cancel_cleanup.val)()
              service("remove-temp", { "path": dir }, _ => ())
            }
          }
          if !@reactive.owner_alive(owner) {
            release()
            return
          }
          cancel_cleanup.val = @reactive.with_owner(owner, () => {
            @reactive.on_cleanup_cancelable(release)
          })
          let path = dir + "/input.txt"
          write_text(path, text, result => {
            if !@reactive.owner_alive(owner) {
              release()
              return
            }
            @reactive.with_owner(owner, () => {
              match result {
                Err(error) => {
                  release()
                  failed(error)
                }
                Ok(_) => consume(path, dir, release)
              }
            })
          })
        }
      }
    })
  })
}

///|
pub fn read_text(
  path : String,
  done : (Result[String, String]) -> Unit,
  limit? : Int = 4 * 1024 * 1024,
) -> Unit {
  let buffer = @buffer.Buffer()
  fn next(offset : Int) -> Unit {
    service("read", { "path": path, "offset": offset }, result => {
      match result {
        Err(error) => done(Err(error))
        Ok(value) => {
          let bytes = @base64.decode(string(field(value, "bytes"))) catch {
            _ => b""
          }
          if offset + bytes.length() > limit {
            done(Err("File exceeds preview limit"))
            return
          }
          buffer.write_bytes(bytes)
          if bytes.length() == 32768 {
            next(offset + bytes.length())
          } else {
            done(Ok(@utf8.decode_lossy(buffer.to_bytes())))
          }
        }
      }
    })
  }
  next(0)
}

///|
pub fn write_text(
  path : String,
  text : String,
  done : (Result[Unit, String]) -> Unit,
) -> Unit {
  let bytes = @utf8.encode(text)
  fn next(offset : Int) -> Unit {
    let end = (offset + 32768).min(bytes.length())
    let chunk = @base64.encode(bytes[offset:end])
    service("write", { "path": path, "bytes": chunk, "append": offset > 0 }, result => {
      match result {
        Err(error) => done(Err(error))
        Ok(_) => if end == bytes.length() { done(Ok(())) } else { next(end) }
      }
    })
  }
  next(0)
}

///|
/// Settings may contain several commit drafts. Chunk large documents instead
/// of embedding them in a single extension message; replacement stays atomic.
pub fn save_state(
  path : String,
  state : Json,
  done : (Result[Json, String]) -> Unit,
) -> Unit {
  let text = state.stringify()
  let size = @utf8.encode(text).length()
  if size > 4 * 1024 * 1024 {
    done(Err("Settings exceed 4 MiB"))
    return
  }
  if size <= 16000 {
    service("save-state", { "path": path, "state": state }, done)
  } else {
    temporary(
      text,
      (input, _, release) => {
        service("save-state", { "path": path, "inputFile": input }, result => {
          release()
          done(result)
        })
      },
      error => done(Err(error)),
    )
  }
}

///|
pub fn load_state(path : String, done : (Result[Json, String]) -> Unit) -> Unit {
  if path == "" {
    done(Ok(Json::empty_object()))
    return
  }
  read_text(path, result => {
    match result {
      Err(error) => done(Err(error))
      Ok(text) => {
        let state = @json.parse(text) catch {
          _ => {
            done(Err("Invalid saved settings"))
            return
          }
        }
        done(Ok(state))
      }
    }
  })
}
