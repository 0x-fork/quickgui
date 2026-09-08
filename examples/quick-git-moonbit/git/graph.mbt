///|
pub(all) struct Edge {
  lane : Int
  color : Int
} derive(Eq, Debug)

///|
pub(all) struct GraphRow {
  lane : Int
  color : Int
  width : Int
  incoming : Bool
  passing : Array[Edge]
  joins : Array[Edge]
  edges : Array[Edge]
} derive(Eq, Debug)

///|
pub fn graph(commits : Array[Commit]) -> Array[GraphRow] {
  let lanes : Array[(String, Int)?] = []
  let result : Array[GraphRow] = []
  let mut next_color = 0
  fn claim(sha : String, color : Int) -> Int {
    for i, lane in lanes {
      if lane is None {
        lanes[i] = Some((sha, color))
        return i
      }
    }
    lanes.push(Some((sha, color)))
    lanes.length() - 1
  }
  for commit in commits {
    let mut lane = -1
    for i, entry in lanes {
      if entry is Some((sha, _)) && sha == commit.sha {
        lane = i
        break
      }
    }
    let incoming = lane >= 0
    let color = if lane < 0 {
      let color = next_color
      next_color += 1
      lane = claim(commit.sha, color)
      color
    } else {
      lanes[lane].unwrap().1
    }
    let joins : Array[Edge] = []
    let passing : Array[Edge] = []
    for i, entry in lanes {
      if entry is Some((sha, color)) && i != lane {
        if sha == commit.sha {
          joins.push({ lane: i, color, })
          lanes[i] = None
        } else {
          passing.push({ lane: i, color, })
        }
      }
    }
    let before = lanes.length()
    let edges : Array[Edge] = []
    if commit.parents.is_empty() {
      lanes[lane] = None
    } else {
      lanes[lane] = Some((commit.parents[0], color))
      edges.push({ lane, color, })
    }
    for n = 1; n < commit.parents.length(); n = n + 1 {
      let parent = commit.parents[n]
      let mut existing = -1
      for i, entry in lanes {
        if entry is Some((sha, _)) && sha == parent {
          existing = i
          break
        }
      }
      if existing >= 0 {
        edges.push({ lane: existing, color: lanes[existing].unwrap().1, })
      } else {
        let c = next_color
        next_color += 1
        edges.push({ lane: claim(parent, c), color: c, })
      }
    }
    while !lanes.is_empty() && lanes.last().unwrap() is None {
      ignore(lanes.pop())
    }
    result.push({
      lane,
      color,
      width: before.max(lanes.length()).max(lane + 1),
      incoming,
      passing,
      joins,
      edges,
    })
  }
  result
}

///|
pub(all) struct GraphLayer {
  color : Int
  source : String
} derive(Eq, Debug)

///|
/// QuickGUI SVGs are tinted masks. Keep one mask per lane color so passing
/// branches and merge edges retain distinct colors, including across themes.
pub fn graph_layers(row : GraphRow, width : Int) -> Array[GraphLayer] {
  let bodies : Map[Int, Array[String]] = Map([])
  let colors : Array[Int] = []
  fn add(color : Int, source : String) -> Unit {
    if bodies.get(color) is Some(parts) {
      parts.push(source)
    } else {
      bodies[color] = [source]
      colors.push(color)
    }
  }
  fn x(lane : Int) -> Int {
    13 + lane * 14
  }
  fn path(color : Int, d : String) -> Unit {
    add(
      color,
      "<path fill=\"none\" stroke=\"#000\" stroke-width=\"2\" d=\"\{d}\"/>",
    )
  }
  let at = x(row.lane)
  for p in row.passing {
    path(p.color, "M\{x(p.lane)} 0V26")
  }
  if row.incoming {
    path(row.color, "M\{at} 0V13")
  }
  for j in row.joins {
    path(j.color, "M\{x(j.lane)} 0C\{x(j.lane)} 13 \{at} 0 \{at} 13")
  }
  for e in row.edges {
    path(e.color, "M\{at} 13C\{at} 26 \{x(e.lane)} 13 \{x(e.lane)} 26")
  }
  add(row.color, "<circle cx=\"\{at}\" cy=\"13\" r=\"4\" fill=\"#000\"/>")
  colors.map(color => {
    color,
    source: "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"\{width}\" height=\"26\" viewBox=\"0 0 \{width} 26\">" +
    bodies[color].join("") +
    "</svg>",
  })
}

///|
pub fn relative_time(seconds : Int64, now : Int64) -> String {
  let delta = (now - seconds).max(0)
  if delta < 60 {
    "just now"
  } else if delta < 3600 {
    "\{delta / 60}m ago"
  } else if delta < 86400 {
    "\{delta / 3600}h ago"
  } else if delta < 2592000 {
    "\{delta / 86400}d ago"
  } else if delta < 31536000 {
    "\{delta / 2592000}mo ago"
  } else {
    "\{delta / 31536000}y ago"
  }
}
