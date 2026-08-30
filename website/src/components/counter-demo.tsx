import { useEffect, useRef, useState } from 'react'
import { WindowFrame } from './window-frame'

const IDLE_DELAY_MS = 950

interface Damage {
  id: number
  width: number
  height: number
}

export function CounterDemo() {
  const [count, setCount] = useState(0)
  const [frames, setFrames] = useState(0)
  const [damage, setDamage] = useState<Damage | null>(null)
  const [sleeping, setSleeping] = useState(true)
  const countRef = useRef<HTMLDivElement | null>(null)
  const idleTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    return () => {
      if (idleTimer.current) clearTimeout(idleTimer.current)
    }
  }, [])

  function increment() {
    const rect = countRef.current?.getBoundingClientRect()
    setCount((value) => value + 1)
    setFrames((value) => value + 1)
    setDamage({
      id: Date.now(),
      width: Math.round((rect?.width ?? 96) + 24),
      height: Math.round((rect?.height ?? 32) + 12),
    })
    setSleeping(false)
    if (idleTimer.current) clearTimeout(idleTimer.current)
    idleTimer.current = setTimeout(() => setSleeping(true), IDLE_DELAY_MS)
  }

  return (
    <WindowFrame title="Counter — 480 × 320">
      <div className="flex h-64 flex-col items-center justify-center gap-4 bg-[#121214] text-[#f0f1f4] sm:h-72">
        <div className="relative">
          <div ref={countRef} className="text-2xl tabular-nums">
            Count: {count}
          </div>
          {damage && !sleeping ? (
            <div
              key={damage.id}
              aria-hidden
              className="damage-rect pointer-events-none absolute -inset-x-3 -inset-y-1.5 rounded-md border border-dashed border-peach/80"
            />
          ) : null}
        </div>
        <button
          type="button"
          onClick={increment}
          className="rounded-lg bg-[#2d69b4] px-4 py-2 text-sm transition-colors hover:bg-[#387acc] active:bg-[#255797]"
        >
          Increment
        </button>
      </div>
      <div className="flex items-center justify-between gap-4 border-t border-white/[0.06] bg-[#0d0d0f] px-3.5 py-2 font-mono text-[11px]">
        {sleeping ? (
          <span className="flex items-center gap-1.5 text-mint/80">
            <span className="size-1.5 rounded-full bg-mint shadow-[0_0_10px_2px_rgba(153,255,228,0.35)]" />
            sleeping · ControlFlow::Wait
          </span>
        ) : (
          <span className="flex items-center gap-1.5 text-peach">
            <span className="size-1.5 rounded-full bg-peach" />
            repaint · damage 1 rect · {damage?.width}×{damage?.height} px
          </span>
        )}
        <span className="text-white/35 tabular-nums">
          {sleeping ? '0 fps' : `frame #${frames}`}
        </span>
      </div>
    </WindowFrame>
  )
}
