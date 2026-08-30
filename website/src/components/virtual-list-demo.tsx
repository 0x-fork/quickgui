import { useState } from 'react'
import { WindowFrame } from './window-frame'

const TOTAL_ROWS = 100_000
const ROW_HEIGHT = 34
const VIEWPORT_HEIGHT = 300
const OVERSCAN = 3

function paintTime(index: number): string {
  return (((index * 2654435761) % 90) / 100 + 0.08).toFixed(2)
}

export function VirtualListDemo() {
  const [scrollTop, setScrollTop] = useState(0)

  const start = Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN)
  const end = Math.min(
    TOTAL_ROWS,
    Math.ceil((scrollTop + VIEWPORT_HEIGHT) / ROW_HEIGHT) + OVERSCAN,
  )

  const rows = []
  for (let index = start; index < end; index++) {
    rows.push(
      <div
        key={index}
        style={{ top: index * ROW_HEIGHT, height: ROW_HEIGHT }}
        className="absolute inset-x-0 flex items-center justify-between border-b border-white/[0.04] px-4 font-mono text-xs"
      >
        <span className="text-white/70">
          Row {index.toLocaleString('en-US')}
        </span>
        <span className="text-white/30">paint {paintTime(index)} ms</span>
      </div>,
    )
  }

  return (
    <WindowFrame title="stress_scroll — 100,000 rows">
      <div
        className="slim-scroll overflow-y-auto bg-[#121214]"
        style={{ height: VIEWPORT_HEIGHT }}
        onScroll={(event) => setScrollTop(event.currentTarget.scrollTop)}
      >
        <div
          className="relative"
          style={{ height: TOTAL_ROWS * ROW_HEIGHT }}
        >
          {rows}
        </div>
      </div>
      <div className="flex items-center justify-between gap-4 border-t border-white/[0.06] bg-[#0d0d0f] px-3.5 py-2 font-mono text-[11px]">
        <span className="text-mint/80">
          mounted {end - start} / {TOTAL_ROWS.toLocaleString('en-US')} rows
        </span>
        <span className="text-white/35 tabular-nums">
          overscan {OVERSCAN} · anchor row{' '}
          {Math.min(TOTAL_ROWS, start + OVERSCAN).toLocaleString('en-US')}
        </span>
      </div>
    </WindowFrame>
  )
}
