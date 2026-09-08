///|
pub(all) struct Line {
  kind : String
  text : String
  old : Int
  new : Int
  no_newline : Bool
} derive(Eq, Debug)

///|
pub(all) struct Hunk {
  old_start : Int
  new_start : Int
  old_count : Int
  new_count : Int
  heading : String
  lines : Array[Line]
} derive(Eq, Debug)

///|
pub(all) struct DiffFile {
  path : String
  old_path : String
  new_path : String
  kind : String
  mode : String
  binary : Bool
  hunks : Array[Hunk]
} derive(Eq, Debug)

///|
pub(all) struct DiffRow {
  file : Int
  hunk : Int
  line : Int
  kind : String
  text : String
  old : Int
  new : Int
} derive(Eq, Debug)

///|
fn range(text : String) -> (Int, Int) {
  let parts = split(tail(text, 1), ",")
  (int(parts[0]), if parts.length() > 1 { int(parts[1]) } else { 1 })
}

///|
/// Decode Git's C-quoted paths, including octal UTF-8 and escaped controls.
pub fn unquote(path : String) -> String {
  if !path.has_prefix("\"") || !path.has_suffix("\"") {
    return path
  }
  let source = @utf8.encode(path)
  let out = @buffer.Buffer()
  let mut i = 1
  while i < source.length() - 1 {
    let c = source[i]
    i += 1
    if c != b'\\' {
      out.write_byte(c)
      continue
    }
    if i >= source.length() - 1 {
      break
    }
    let c = source[i]
    i += 1
    match c {
      b'n' => out.write_byte(b'\n')
      b'r' => out.write_byte(b'\r')
      b't' => out.write_byte(b'\t')
      b'0'..=b'7' => {
        let mut value = c.to_int() - 48
        for j = 0; j < 2 && i < source.length() - 1; j = j + 1 {
          let d = source[i]
          if d < b'0' || d > b'7' {
            break
          }
          value = value * 8 + d.to_int() - 48
          i += 1
        }
        out.write_byte(value.to_byte())
      }
      _ => out.write_byte(c)
    }
  }
  @utf8.decode_lossy(out.to_bytes())
}

///|
fn file_path(raw : String) -> String {
  let value = unquote(raw)
  if value == "/dev/null" {
    ""
  } else if value.has_prefix("a/") || value.has_prefix("b/") {
    tail(value, 2)
  } else {
    value
  }
}

///|
pub fn parse_diff(text : String) -> Array[DiffFile] {
  let files : Array[DiffFile] = []
  let mut current : DiffFile? = None
  let mut hunk : Hunk? = None
  let mut old = 0
  let mut new = 0
  for raw in split(text, "\n") {
    if raw.has_prefix("diff --git ") {
      if current is Some(file) {
        if hunk is Some(value) {
          file.hunks.push(value)
        }
        files.push(file)
      }
      hunk = None
      let pair = tail(raw, 11)
      let name = match pair.find(" b/") {
        Some(i) => tail(pair, i + 3)
        None => ""
      }
      current = Some({
        path: name,
        old_path: name,
        new_path: name,
        kind: "M",
        mode: "100644",
        binary: false,
        hunks: [],
      })
      continue
    }
    guard current is Some(file) else { continue }
    if raw.has_prefix("@@ ") {
      if hunk is Some(value) {
        file.hunks.push(value)
      }
      let parts = fields(raw + " ", 4)
      if parts.length() < 5 {
        hunk = None
        continue
      }
      let a = range(parts[1])
      let b = range(parts[2])
      old = a.0.max(1)
      new = b.0.max(1)
      hunk = Some({
        old_start: a.0,
        new_start: b.0,
        old_count: a.1,
        new_count: b.1,
        heading: parts[4].trim_end().to_owned(),
        lines: [],
      })
    } else if raw.has_prefix("@@ -") {
      ()
    } else if hunk is Some(value) {
      if raw.has_prefix("\\ No newline") {
        if !value.lines.is_empty() {
          let index = value.lines.length() - 1
          value.lines[index] = { ..value.lines[index], no_newline: true, }
        }
      } else if raw.has_prefix("+") {
        value.lines.push({
          kind: "+",
          text: tail(raw, 1),
          old: 0,
          new,
          no_newline: false,
        })
        new += 1
      } else if raw.has_prefix("-") {
        value.lines.push({
          kind: "-",
          text: tail(raw, 1),
          old,
          new: 0,
          no_newline: false,
        })
        old += 1
      } else if raw.has_prefix(" ") {
        value.lines.push({
          kind: " ",
          text: tail(raw, 1),
          old,
          new,
          no_newline: false,
        })
        old += 1
        new += 1
      }
    } else if raw.has_prefix("new file mode ") {
      current = Some({ ..file, kind: "A", mode: tail(raw, 14), old_path: "", })
    } else if raw.has_prefix("deleted file mode ") {
      current = Some({ ..file, kind: "D", mode: tail(raw, 18), new_path: "", })
    } else if raw.has_prefix("rename from ") {
      current = Some({ ..file, kind: "R", old_path: unquote(tail(raw, 12)), })
    } else if raw.has_prefix("rename to ") {
      let path = unquote(tail(raw, 10))
      current = Some({ ..file, kind: "R", new_path: path, path, })
    } else if raw.has_prefix("copy from ") {
      current = Some({ ..file, kind: "C", old_path: unquote(tail(raw, 10)), })
    } else if raw.has_prefix("copy to ") {
      let path = unquote(tail(raw, 8))
      current = Some({ ..file, kind: "C", new_path: path, path, })
    } else if raw.has_prefix("--- ") {
      let path = file_path(tail(raw, 4))
      current = Some({
        ..file,
        old_path: path,
        path: if file.new_path == "" {
          path
        } else {
          file.path
        },
      })
    } else if raw.has_prefix("+++ ") {
      let path = file_path(tail(raw, 4))
      current = Some({
        ..file,
        new_path: path,
        path: if path == "" {
          file.old_path
        } else {
          path
        },
      })
    } else if raw.has_prefix("Binary files ") ||
      raw.has_prefix("GIT binary patch") {
      current = Some({ ..file, binary: true, })
    }
  }
  if current is Some(file) {
    if hunk is Some(value) {
      file.hunks.push(value)
    }
    files.push(file)
  }
  files
}

///|
pub fn diff_rows(files : Array[DiffFile]) -> Array[DiffRow] {
  let rows : Array[DiffRow] = []
  for fi, file in files {
    rows.push({
      file: fi,
      hunk: -1,
      line: -1,
      kind: "file",
      text: file.path,
      old: 0,
      new: 0,
    })
    if file.binary {
      rows.push({
        file: fi,
        hunk: -1,
        line: -1,
        kind: "binary",
        text: "Binary file changed",
        old: 0,
        new: 0,
      })
    }
    for hi, hunk in file.hunks {
      rows.push({
        file: fi,
        hunk: hi,
        line: -1,
        kind: "hunk",
        text: "@@ -\{hunk.old_start},\{hunk.old_count} +\{hunk.new_start},\{hunk.new_count} @@ \{hunk.heading}",
        old: 0,
        new: 0,
      })
      for li, line in hunk.lines {
        rows.push({
          file: fi,
          hunk: hi,
          line: li,
          kind: line.kind,
          text: line.text,
          old: line.old,
          new: line.new,
        })
      }
    }
    if file.hunks.is_empty() && !file.binary {
      rows.push({
        file: fi,
        hunk: -1,
        line: -1,
        kind: "meta",
        text: "File metadata changed",
        old: 0,
        new: 0,
      })
    }
  }
  rows
}

///|
pub fn quote(path : String) -> String {
  if path.contains("\n") ||
    path.contains("\t") ||
    path.contains("\r") ||
    path.contains("\"") ||
    path.contains("\\") {
    "\"" +
    path
    .replace_all(old="\\", new="\\\\")
    .replace_all(old="\"", new="\\\"")
    .replace_all(old="\t", new="\\t")
    .replace_all(old="\n", new="\\n")
    .replace_all(old="\r", new="\\r") +
    "\""
  } else {
    path
  }
}

///|
/// Selected line indices are keyed by hunk. None selects the whole hunk. Partial
/// reverse patches retain unselected additions as context, matching git apply -R.
pub fn patch(
  file : DiffFile,
  selected : Map[Int, Array[Int]?],
  reverse : Bool,
) -> String {
  let body = StringBuilder()
  let mut delta = 0
  let mut whole = true
  for hi, hunk in file.hunks {
    guard selected.get(hi) is Some(selection) else {
      whole = false
      continue
    }
    let lines = StringBuilder()
    let mut old_count = 0
    let mut new_count = 0
    let mut changed = false
    for li, line in hunk.lines {
      let chosen = match selection {
        None => true
        Some(items) => items.contains(li)
      }
      let mut mark = " "
      if line.kind != " " {
        if chosen {
          mark = line.kind
          changed = true
        } else {
          whole = false
          if (line.kind == "+" && !reverse) || (line.kind == "-" && reverse) {
            continue
          }
        }
      }
      if mark != "+" {
        old_count += 1
      }
      if mark != "-" {
        new_count += 1
      }
      lines.write_string(mark + line.text + "\n")
      if line.no_newline {
        lines.write_string("\\ No newline at end of file\n")
      }
    }
    if !changed {
      continue
    }
    let old_start = if old_count == 0 { 0 } else { hunk.old_start.max(1) }
    let new_start = if new_count == 0 {
      0
    } else if reverse {
      hunk.new_start.max(1)
    } else {
      (hunk.old_start + delta).max(1)
    }
    body.write_string(
      "@@ -\{old_start},\{old_count} +\{new_start},\{new_count} @@ \{hunk.heading}\n",
    )
    body.write_string(lines.to_string())
    delta += new_count - old_count
  }
  if body.to_string().is_empty() {
    return ""
  }
  let old_name = if file.old_path == "" { file.path } else { file.old_path }
  let new_name = if file.new_path == "" { file.path } else { file.new_path }
  let a = quote("a/" + old_name)
  let b = quote("b/" + new_name)
  let header = if file.kind == "A" && (!reverse || whole) {
    "new file mode \{file.mode}\n--- /dev/null\n+++ \{b}\n"
  } else if file.kind == "D" && whole {
    "deleted file mode \{file.mode}\n--- \{a}\n+++ /dev/null\n"
  } else if file.kind == "R" || file.kind == "C" {
    let kind = if file.kind == "R" { "rename" } else { "copy" }
    "\{kind} from \{quote(old_name)}\n\{kind} to \{quote(new_name)}\n--- \{a}\n+++ \{b}\n"
  } else {
    "--- \{a}\n+++ \{b}\n"
  }
  "diff --git \{a} \{b}\n" + header + body.to_string()
}
