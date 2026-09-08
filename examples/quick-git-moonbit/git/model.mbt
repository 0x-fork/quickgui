///|
pub fn split(text : String, delimiter : String) -> Array[String] {
  text.split(delimiter).map(s => s.to_owned()).to_array()
}

///|
pub fn int(text : String) -> Int {
  @string.parse_int(text) catch {
    _ => 0
  }
}

///|
pub fn tail(text : String, start : Int) -> String {
  text.view(start_offset=start.min(text.length())).to_owned()
}

///|
pub fn base(path : String) -> String {
  split(path.replace_all(old="\\", new="/"), "/").last().unwrap_or(path)
}

///|
pub fn parent(path : String) -> String {
  let parts = split(path.replace_all(old="\\", new="/"), "/")
  if parts.length() <= 1 {
    "."
  } else {
    parts[:parts.length() - 1].to_owned().join("/")
  }
}

///|
pub fn fields(text : String, count : Int) -> Array[String] {
  let result : Array[String] = []
  let mut rest = text
  for n = 0; n < count; n = n + 1 {
    match rest.find(" ") {
      Some(i) => {
        result.push(rest.view(end_offset=i).to_owned())
        rest = tail(rest, i + 1)
      }
      None => return []
    }
  }
  result.push(rest)
  result
}

///|
pub(all) struct Change {
  path : String
  original : String
  index : String
  worktree : String
  conflict : Bool
  untracked : Bool
} derive(Eq, Debug)

///|
pub(all) struct Status {
  head : String
  branch : String
  upstream : String
  ahead : Int
  behind : Int
  entries : Array[Change]
} derive(Eq, Debug)

///|
pub fn empty_status() -> Status {
  { head: "", branch: "", upstream: "", ahead: 0, behind: 0, entries: [], }
}

///|
pub fn parse_status(text : String) -> Status {
  let records = split(text, "\u0000")
  let mut head = ""
  let mut branch = ""
  let mut upstream = ""
  let mut ahead = 0
  let mut behind = 0
  let entries : Array[Change] = []
  let mut i = 0
  while i < records.length() {
    let record = records[i]
    i += 1
    if record.has_prefix("# branch.oid ") {
      head = tail(record, 13)
      if head == "(initial)" {
        head = ""
      }
    } else if record.has_prefix("# branch.head ") {
      branch = tail(record, 14)
    } else if record.has_prefix("# branch.upstream ") {
      upstream = tail(record, 18)
    } else if record.has_prefix("# branch.ab ") {
      let parts = split(tail(record, 12), " ")
      if parts.length() == 2 {
        ahead = int(tail(parts[0], 1))
        behind = int(tail(parts[1], 1))
      }
    } else if record.has_prefix("? ") {
      entries.push({
        path: tail(record, 2),
        original: "",
        index: ".",
        worktree: "?",
        conflict: false,
        untracked: true,
      })
    } else {
      let rename = record.has_prefix("2 ")
      let conflict = record.has_prefix("u ")
      if !rename && !conflict && !record.has_prefix("1 ") {
        continue
      }
      let parts = fields(
        record,
        if rename {
          9
        } else if conflict {
          10
        } else {
          8
        },
      )
      let original = if rename && i < records.length() {
        let p = records[i]
        i += 1
        p
      } else {
        ""
      }
      if parts.length() < 9 || parts[1].length() != 2 {
        continue
      }
      entries.push({
        path: parts.last().unwrap(),
        original,
        index: parts[1].view(end_offset=1).to_owned(),
        worktree: tail(parts[1], 1),
        conflict,
        untracked: false,
      })
    }
  }
  { head, branch, upstream, ahead, behind, entries, }
}

///|
pub fn changes(status : Status, staged : Bool) -> Array[Change] {
  status.entries.filter(e => {
    if staged {
      e.index != "." && !e.untracked && !e.conflict
    } else {
      e.worktree != "." || e.conflict
    }
  })
}

///|
pub(all) struct Commit {
  sha : String
  short : String
  parents : Array[String]
  author : String
  email : String
  time : Int64
  refs : Array[String]
  subject : String
  body : String
} derive(Eq, Debug)

///|
pub let log_format : String = "%H%x1f%h%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s%x1f%b%x00"

///|
pub fn parse_log(text : String) -> Array[Commit] {
  let result : Array[Commit] = []
  for raw in split(text, "\u0000") {
    let f = split(raw.trim_start(chars="\n").to_owned(), "\u001f")
    if f.length() < 9 {
      continue
    }
    result.push({
      sha: f[0],
      short: f[1],
      parents: split(f[2], " ").filter(s => s != ""),
      author: f[3],
      email: f[4],
      time: @string.parse_int64(f[5]) catch {
        _ => 0
      },
      refs: split(f[6], ", ").filter(s => s != ""),
      subject: f[7],
      body: f[8].trim().to_owned(),
    })
  }
  result
}

///|
pub(all) struct Branch {
  name : String
  full : String
  sha : String
  upstream : String
  track : String
  current : Bool
  remote : Bool
} derive(Eq, Debug)

///|
pub let ref_format : String = "%(refname)%00%(objectname:short)%00%(upstream:short)%00%(upstream:track)%00%(HEAD)"

///|
pub fn parse_refs(text : String) -> Array[Branch] {
  let result : Array[Branch] = []
  for line in split(text, "\n") {
    let f = split(line, "\u0000")
    if f.length() < 5 || f[0].has_suffix("/HEAD") {
      continue
    }
    let remote = f[0].has_prefix("refs/remotes/")
    result.push({
      name: tail(
        f[0],
        if remote {
          13
        } else if f[0].has_prefix("refs/heads/") {
          11
        } else {
          10
        },
      ),
      full: f[0],
      sha: f[1],
      upstream: f[2],
      track: f[3],
      current: f[4] == "*",
      remote,
    })
  }
  result
}

///|
pub(all) struct Worktree {
  path : String
  branch : String
  head : String
  locked : Bool
  prunable : Bool
} derive(Eq, Debug)

///|
pub fn parse_worktrees(text : String) -> Array[Worktree] {
  let result : Array[Worktree] = []
  let mut path = ""
  let mut branch = ""
  let mut head = ""
  let mut locked = false
  let mut prunable = false
  for line in split(text, "\u0000") {
    if line == "" {
      if path != "" {
        result.push({ path, branch, head, locked, prunable, })
      }
      path = ""
      branch = ""
      head = ""
      locked = false
      prunable = false
    } else if line.has_prefix("worktree ") {
      path = tail(line, 9)
    } else if line.has_prefix("HEAD ") {
      head = tail(line, 5)
    } else if line.has_prefix("branch ") {
      branch = tail(line, 18)
    } else if line.has_prefix("locked") {
      locked = true
    } else if line.has_prefix("prunable") {
      prunable = true
    }
  }
  if path != "" {
    result.push({ path, branch, head, locked, prunable, })
  }
  result
}

///|
pub(all) struct Stash {
  name : String
  sha : String
  summary : String
  time : Int64
} derive(Eq, Debug)

///|
pub fn parse_stashes(text : String) -> Array[Stash] {
  let result : Array[Stash] = []
  for line in split(text, "\u0000") {
    let f = split(line.trim_start(chars="\n").to_owned(), "\u001f")
    if f.length() >= 4 {
      result.push({
        name: f[0],
        sha: f[1],
        time: @string.parse_int64(f[2]) catch {
          _ => 0
        },
        summary: f[3],
      })
    }
  }
  result
}

///|
pub(all) struct CommitFile {
  path : String
  original : String
  status : String
} derive(Eq, Debug)

///|
pub fn parse_commit_files(text : String) -> Array[CommitFile] {
  let parts = split(text, "\u0000")
  let result : Array[CommitFile] = []
  let mut i = 0
  while i + 1 < parts.length() {
    let status = parts[i].trim().to_owned()
    let path = parts[i + 1]
    i += 2
    if status.has_prefix("R") || status.has_prefix("C") {
      if i < parts.length() {
        result.push({
          path: parts[i],
          original: path,
          status: status.view(end_offset=1).to_owned(),
        })
        i += 1
      }
    } else if status != "" {
      result.push({ path, original: "", status, })
    }
  }
  result
}

///|
pub fn branch_problem(name : String) -> String {
  if name == "" {
    return "Enter a branch name"
  }
  if name.has_prefix("-") ||
    name.has_prefix(".") ||
    name.has_suffix(".") ||
    name.has_suffix("/") ||
    name.has_suffix(".lock") ||
    name == "@" {
    return "Invalid branch name"
  }
  for
    invalid in [
      " ", "\t", "\n", "\r", "..", "@{", "~", "^", ":", "?", "*", "[", "\\", "//",
      "/.",
    ] {
    if name.contains(invalid) {
      return "Invalid branch name"
    }
  }
  ""
}

///|
/// Explicit pathspecs keep unusual filenames literal without changing the
/// environment of Git's own subprocesses (notably stash's cleanup commands).
pub fn literal(path : String) -> String {
  ":(literal)" + path
}
