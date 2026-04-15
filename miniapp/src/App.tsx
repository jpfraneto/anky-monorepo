import { useState, useEffect, useRef, useCallback } from 'react'
import L from 'leaflet'
import 'leaflet/dist/leaflet.css'
import './app.css'

console.log('%c👽 anky', 'font-size: 24px; font-weight: bold; color: #b366ff')
console.log('%chello young adventurer. got any feedback? https://anky.app/feedback', 'color: #9e9590; font-size: 12px')
console.log('%c─────────────────────────────────', 'color: #333')

function log(area: string, ...args: any[]) {
  console.log(`%c[anky:${area}]`, 'color: #b366ff; font-weight: bold', ...args)
}

type MsgType = 'anky' | 'user' | 'system' | 'image' | 'mirror' | 'typing' | 'birth' | 'commands' | 'buttons' | 'qr' | 'map' | 'timepicker'

interface Message {
  id: string
  type: MsgType
  content: string
  mirror?: { imageUrl?: string; text?: string; gap?: string; kingdom?: string }
  birth?: { ankyId: string; imageUrl?: string; status: 'generating' | 'complete' }
  buttons?: { label: string; value: string }[]
  qr?: { svg: string; url: string }
  expanded?: boolean
}

interface AnkyData {
  id: string; title?: string; imageUrl?: string; reflection?: string; writing?: string
}

interface FarcasterUser {
  fid: number; username?: string; displayName?: string; pfpUrl?: string
}

const SESSION_DURATION = 480
const IDLE_TIMEOUT = 8
const IDLE_VISIBLE_AFTER = 3
const ANKY_MODE_THRESHOLD = 12 // seconds of continuous typing before anky mode kicks in

const KINGDOMS = [
  { name: 'primordia', chakra: 'root', color: '#cc2222' },
  { name: 'emblazion', chakra: 'sacral', color: '#e87020' },
  { name: 'chryseos', chakra: 'solar plexus', color: '#d4a017' },
  { name: 'eleasis', chakra: 'heart', color: '#2d9e2d' },
  { name: 'voxlumis', chakra: 'throat', color: '#3399ff' },
  { name: 'insightia', chakra: 'third eye', color: '#7744cc' },
  { name: 'claridium', chakra: 'crown', color: '#b388ff' },
  { name: 'poiesis', chakra: 'transcendent', color: '#e0d8c8' },
]
const CHAKRA_COLORS = KINGDOMS.map(k => k.color)

function uid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => {
    const r = (Math.random() * 16) | 0
    return (c === 'x' ? r : (r & 0x3) | 0x8).toString(16)
  })
}

function useViewportHeight() {
  useEffect(() => {
    function setH() {
      const h = window.visualViewport ? window.visualViewport.height : window.innerHeight
      document.documentElement.style.setProperty('--app-h', h + 'px')
    }
    setH()
    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', setH)
      window.visualViewport.addEventListener('scroll', setH)
      return () => {
        window.visualViewport!.removeEventListener('resize', setH)
        window.visualViewport!.removeEventListener('scroll', setH)
      }
    } else {
      window.addEventListener('resize', setH)
      return () => window.removeEventListener('resize', setH)
    }
  }, [])
}

export function App() {
  useViewportHeight()
  const [messages, setMessages] = useState<Message[]>([])
  const [view, setView] = useState<'chat' | 'profile' | 'convo'>('chat')
  const nowFlowRef = useRef<{ step: string; prompt: string; lat: number | null; lng: number | null; when: number | null } | null>(null)
  const [user, setUser] = useState<FarcasterUser | null>(null)
  const [pfp, setPfp] = useState('/static/icon-192.png')
  const [ankys, setAnkys] = useState<AnkyData[]>([])
  const [selectedAnky, setSelectedAnky] = useState<AnkyData | null>(null)
  const [profileBio, setProfileBio] = useState('')
  const [profileName, setProfileName] = useState('anon')
  const [ankyCount, setAnkyCount] = useState(0)
  const [walletAddress, setWalletAddress] = useState<string | null>(null)
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [text, setText] = useState('')

  // Mode: 'chat' = normal messaging, 'anky' = writing session active, 'paused' = idle timeout
  const [mode, setMode] = useState<'chat' | 'anky' | 'paused'>('chat')
  const [activeTime, setActiveTime] = useState(0)
  const [idleProgress, setIdleProgress] = useState(0)

  const chatRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLTextAreaElement>(null)
  const sdkRef = useRef<any>(null)
  const initializedRef = useRef(false)

  // timing refs
  const typingStartRef = useRef(0) // when did user start typing this message
  const lastInputRef = useRef(0)
  const tickRef = useRef<number | null>(null)
  const keystrokesRef = useRef<number[]>([])
  const lastKeystrokeRef = useRef(0)
  const pausedAtRef = useRef(0)
  const strictValueRef = useRef('')
  const modeRef = useRef(mode)
  modeRef.current = mode
  const textRef = useRef(text)
  textRef.current = text

  // ── Helpers ──

  const addMsg = useCallback((type: MsgType, content: string, extra?: Partial<Message>) => {
    const msg: Message = { id: uid(), type, content, ...extra }
    setMessages(prev => [...prev, msg])
    return msg.id
  }, [])

  const removeMsg = useCallback((id: string) => {
    setMessages(prev => prev.filter(m => m.id !== id))
  }, [])

  const updateMsg = useCallback((id: string, content: string) => {
    setMessages(prev => prev.map(m => m.id === id ? { ...m, content } : m))
  }, [])

  const wait = (ms: number) => new Promise(r => setTimeout(r, ms))

  function recordInputActivity() {
    const now = Date.now()
    lastInputRef.current = now
    if (!typingStartRef.current) {
      typingStartRef.current = now
      startTicking()
    }
    if (lastKeystrokeRef.current) keystrokesRef.current.push(now - lastKeystrokeRef.current)
    lastKeystrokeRef.current = now
  }

  useEffect(() => {
    if (chatRef.current) chatRef.current.scrollTop = chatRef.current.scrollHeight
  }, [messages])

  // ── Tick (runs during both chat and anky mode to detect transition) ──

  function startTicking() {
    if (tickRef.current) return
    tickRef.current = window.setInterval(tick, 100)
  }

  function stopTicking() {
    if (tickRef.current) { clearInterval(tickRef.current); tickRef.current = null }
  }

  function tick() {
    const now = Date.now()
    if (!typingStartRef.current) return

    const elapsed = (now - typingStartRef.current) / 1000

    // Chat mode → check if we should transition to anky mode
    if (modeRef.current === 'chat') {
      if (elapsed >= ANKY_MODE_THRESHOLD && textRef.current.length > 0) {
        log('mode', '🔮 anky mode activated — you\'ve been writing for ' + Math.round(elapsed) + 's. no more backspace. no more stopping. let it flow.')
        setMode('anky')
        modeRef.current = 'anky'
        strictValueRef.current = textRef.current
        inputRef.current?.focus()
      }
      return
    }

    // Anky mode
    if (modeRef.current === 'anky') {
      setActiveTime(elapsed)

      const idle = (now - lastInputRef.current) / 1000
      if (idle > IDLE_VISIBLE_AFTER && textRef.current.length > 0) {
        const p = Math.min((idle - IDLE_VISIBLE_AFTER) / (IDLE_TIMEOUT - IDLE_VISIBLE_AFTER), 1)
        setIdleProgress(p)
      } else {
        setIdleProgress(0)
      }

      if (idle >= IDLE_TIMEOUT && textRef.current.length > 0) {
        setMode('paused')
        modeRef.current = 'paused'
        pausedAtRef.current = elapsed
        stopTicking()
        setIdleProgress(1)
      }
    }
  }

  function resumeFromPause() {
    lastInputRef.current = Date.now()
    typingStartRef.current = Date.now() - pausedAtRef.current * 1000
    setMode('anky')
    modeRef.current = 'anky'
    setIdleProgress(0)
    startTicking()
    inputRef.current?.focus()
  }

  // ── Send a chat message (short text) ──

  function handleButtonClick(value: string) {
    // Remove the buttons message that was clicked
    setMessages(prev => prev.filter(m => m.type !== 'buttons'))
    if (value === '_map') {
      addMsg('user', 'pick on map')
      // Get current position first, then show map
      if (navigator.geolocation) {
        navigator.geolocation.getCurrentPosition(
          pos => addMsg('map', `${pos.coords.latitude},${pos.coords.longitude}`),
          () => addMsg('map', '0,0')
        )
      } else {
        addMsg('map', '0,0')
      }
      return
    }
    if (value === '_time') {
      addMsg('user', 'pick time')
      addMsg('timepicker', '')
      return
    }
    handleNowInput(value)
  }

  function handleNowInput(input: string) {
    const flow = nowFlowRef.current
    if (!flow) return

    if (flow.step === 'prompt') {
      flow.prompt = input
      flow.step = 'location'
      addMsg('anky', 'where?')
      addMsg('buttons', '', { buttons: [{ label: 'here', value: 'here' }, { label: 'pick on map', value: '_map' }, { label: 'skip', value: 'skip' }] })
      return
    }

    if (flow.step === 'location') {
      if (input === 'here') {
        addMsg('user', 'here')
        if (navigator.geolocation) {
          navigator.geolocation.getCurrentPosition(
            pos => {
              flow.lat = pos.coords.latitude
              flow.lng = pos.coords.longitude
              addMsg('anky', `${flow.lat!.toFixed(4)}, ${flow.lng!.toFixed(4)}`)
              flow.step = 'when'
              addMsg('anky', 'when?')
              addMsg('buttons', '', { buttons: [{ label: 'now', value: 'now' }, { label: '5 min', value: '5' }, { label: '10 min', value: '10' }, { label: '30 min', value: '30' }, { label: 'pick time', value: '_time' }] })
            },
            () => {
              addMsg('anky', "couldn't get your location. type a city or coords (e.g. 40.7128, -74.0060), or skip.")
              addMsg('buttons', '', { buttons: [{ label: 'skip', value: 'skip' }] })
            }
          )
        } else {
          addMsg('anky', "location not available here. type a city or coords, or skip.")
          addMsg('buttons', '', { buttons: [{ label: 'skip', value: 'skip' }] })
        }
        return
      }
      addMsg('user', input)
      // Try parse coords
      const parts = input.split(/[,\s]+/).map(Number)
      if (parts.length >= 2 && !isNaN(parts[0]) && !isNaN(parts[1])) {
        flow.lat = parts[0]; flow.lng = parts[1]
      }
      flow.step = 'when'
      addMsg('anky', 'when?')
      addMsg('buttons', '', { buttons: [{ label: 'now', value: 'now' }, { label: '5 min', value: '5' }, { label: '10 min', value: '10' }, { label: '30 min', value: '30' }, { label: 'pick time', value: '_time' }] })
      return
    }

    if (flow.step === 'when') {
      addMsg('user', input === 'now' ? 'now' : input + ' min')
      const mins = input === 'now' ? 0 : parseInt(input)
      flow.when = isNaN(mins) || mins <= 0 ? null : mins
      createNow()
      return
    }
  }

  async function createNow() {
    const flow = nowFlowRef.current
    if (!flow) return
    flow.step = 'creating'
    const typingId = addMsg('typing', '')

    const visitorId = localStorage.getItem('anky_visitor_id') || uid()
    localStorage.setItem('anky_visitor_id', visitorId)

    const body: any = { prompt: flow.prompt, mode: flow.when ? 'live' : 'sticker', creator_id: visitorId }
    if (flow.when) body.duration_seconds = flow.when * 60
    if (flow.lat !== null) { body.latitude = flow.lat; body.longitude = flow.lng }

    try {
      const r = await fetch('/api/v1/now', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) })
      const d = await r.json()
      removeMsg(typingId)
      if (d.error) { addMsg('anky', 'something went wrong: ' + d.error); nowFlowRef.current = null; return }

      if (flow.when) {
        await fetch(`/api/v1/now/${d.slug}/start`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ creator_id: visitorId })
        })
      }

      addMsg('anky', flow.when ? `your now is live. starts in ${flow.when} min.` : 'your now is live.')
      addMsg('qr', '', { qr: { svg: d.qr_svg, url: d.qr_url } })
      addMsg('buttons', '', { buttons: [
        { label: 'copy link', value: '_copy:' + d.qr_url },
        { label: 'download QR', value: '_dl:' + d.qr_svg + ':::' + d.slug }
      ]})
    } catch {
      removeMsg(typingId)
      addMsg('anky', "couldn't create the now. try again.")
    }
    nowFlowRef.current = null
  }

  const genFlowRef = useRef<boolean>(false)

  async function handleGenerate(prompt: string) {
    genFlowRef.current = false
    const typingId = addMsg('typing', '')
    try {
      const r = await fetch('/api/v1/generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ writing: prompt, model: 'flux', aspect_ratio: '9:16' })
      })
      const d = await r.json()
      if (d.error) { removeMsg(typingId); addMsg('anky', d.error); return }

      // Poll for completion
      const ankyId = d.anky_id
      const poll = async () => {
        for (let i = 0; i < 60; i++) {
          await new Promise(r => setTimeout(r, 3000))
          try {
            const pr = await fetch(`/api/v1/anky/${ankyId}`, { credentials: 'include' })
            const pd = await pr.json()
            if (pd.status !== 'generating') {
              removeMsg(typingId)
              if (pd.image_url) {
                addMsg('image', pd.image_url)
              } else {
                addMsg('anky', 'generation failed. try again.')
              }
              return
            }
          } catch { /* keep polling */ }
        }
        removeMsg(typingId)
        addMsg('anky', 'generation timed out.')
      }
      poll()
    } catch {
      removeMsg(typingId)
      addMsg('anky', "couldn't reach the server.")
    }
  }

  async function loadGallery() {
    const typingId = addMsg('typing', '')
    try {
      const r = await fetch('/api/v1/ankys')
      const d = await r.json()
      removeMsg(typingId)
      const ankys = (d.ankys || []).filter((a: any) => a.image_path).slice(0, 8)
      if (ankys.length === 0) {
        addMsg('anky', 'no ankys yet.')
        return
      }
      addMsg('anky', 'latest ankys:')
      setMessages(prev => [...prev, {
        id: uid(),
        type: 'system' as MsgType,
        content: '_gallery',
        buttons: ankys.map((a: any) => ({ label: a.title || 'untitled', value: a.image_path }))
      }])
    } catch {
      removeMsg(typingId)
      addMsg('anky', "couldn't load the gallery.")
    }
  }

  function runCommand(cmd: string) {
    addMsg('user', cmd)
    if (cmd === '/now') {
      nowFlowRef.current = { step: 'prompt', prompt: '', lat: null, lng: null, when: null }
      addMsg('anky', "what's the prompt?")
    } else if (cmd === '/generate') {
      genFlowRef.current = true
      addMsg('anky', 'write the prompt for generating an image.')
    } else if (cmd === '/gallery') {
      loadGallery()
    } else if (cmd === '/help') {
      addMsg('commands', '')
    } else if (cmd === '/profile') {
      setView('profile'); loadProfile()
    } else if (cmd === '/prompt') {
      const typingId = addMsg('typing', '')
      fetch('/api/miniapp/prompt' + (user?.fid ? '?fid=' + user.fid : ''))
        .then(r => r.json())
        .then(d => {
          removeMsg(typingId)
          addMsg('anky', d.prompt || 'just write. whatever comes to mind.')
        })
        .catch(() => {
          removeMsg(typingId)
          addMsg('anky', 'just write. whatever comes to mind.')
        })
    }
  }

  async function sendChatMessage() {
    const msg = textRef.current.trim()
    if (!msg) return

    // Intercept slash commands
    if (msg.startsWith('/')) {
      stopTicking()
      setText('')
      textRef.current = ''
      typingStartRef.current = 0
      setMode('chat')
      modeRef.current = 'chat'
      strictValueRef.current = ''
      runCommand(msg.toLowerCase())
      return
    }

    // Handle /generate conversational flow
    if (genFlowRef.current) {
      stopTicking()
      setText('')
      textRef.current = ''
      typingStartRef.current = 0
      setMode('chat')
      modeRef.current = 'chat'
      strictValueRef.current = ''
      addMsg('user', msg)
      handleGenerate(msg)
      return
    }

    // Handle /now conversational flow
    if (nowFlowRef.current) {
      stopTicking()
      setText('')
      textRef.current = ''
      typingStartRef.current = 0
      setMode('chat')
      modeRef.current = 'chat'
      strictValueRef.current = ''
      addMsg('user', msg)
      handleNowInput(msg)
      return
    }

    stopTicking()
    setText('')
    textRef.current = ''
    typingStartRef.current = 0
    setMode('chat')
    modeRef.current = 'chat'
    strictValueRef.current = ''
    setActiveTime(0)
    setIdleProgress(0)
    keystrokesRef.current = []
    lastKeystrokeRef.current = 0

    addMsg('user', msg)
    log('chat', 'sending message:', msg.slice(0, 80) + (msg.length > 80 ? '...' : ''))
    const typingId = addMsg('typing', '')

    try {
      const resp = await fetch('/write', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          text: msg,
          duration: 0,
          session_id: uid(),
          flow_score: 0,
        }),
      })
      const data = await resp.json()
      log('chat', 'response:', { is_anky: data.is_anky, model: data.model, provider: data.provider })
      removeMsg(typingId)
      addMsg('anky', data.response || data.error || '...')
    } catch (e) {
      log('chat', 'send failed:', e)
      removeMsg(typingId)
      addMsg('anky', "couldn't reach the server right now.")
    }
  }

  // ── Submit an anky (long writing session) ──

  async function submitAnky() {
    stopTicking()
    const content = textRef.current
    const duration = (Date.now() - typingStartRef.current) / 1000

    setText('')
    textRef.current = ''
    typingStartRef.current = 0
    setMode('chat')
    modeRef.current = 'chat'
    setActiveTime(0)
    setIdleProgress(0)
    strictValueRef.current = ''

    log('anky', '📝 submitting anky — duration: ' + Math.round(duration) + 's, words: ' + content.split(/\s+/).length + ', flow: ' + calcFlowScore())
    const preview = content.length > 300 ? content.slice(0, 300) + '...' : content
    addMsg('user', preview)
    const typingId = addMsg('typing', '')

    try {
      const resp = await fetch('/write', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          text: content, duration,
          keystroke_deltas: keystrokesRef.current.slice(0, 5000),
          session_id: uid(),
          flow_score: calcFlowScore(),
        }),
      })
      const data = await resp.json()

      if (data.is_anky && data.anky_id) {
        streamReflection(data.anky_id, typingId)
      } else {
        removeMsg(typingId)
        addMsg('anky', data.response || data.error || 'your writing has been received.')
      }
    } catch {
      removeMsg(typingId)
      addMsg('anky', "something went wrong. your writing happened though.")
    }

    keystrokesRef.current = []
    lastKeystrokeRef.current = 0
  }

  function calcFlowScore(): number {
    const deltas = keystrokesRef.current
    if (deltas.length < 5) return 0
    const mean = deltas.reduce((a, b) => a + b, 0) / deltas.length
    const variance = deltas.reduce((a, d) => a + (d - mean) ** 2, 0) / deltas.length
    const cv = mean > 0 ? Math.sqrt(variance) / mean : 1
    const stability = Math.max(0, 1 - cv) * 0.45
    const totalPause = deltas.filter(d => d > 2000).reduce((a, d) => a + d, 0) / 1000
    const at = (Date.now() - typingStartRef.current) / 1000
    const continuity = Math.max(0, 1 - totalPause / Math.max(at, 1)) * 0.30
    const speedScore = Math.max(0, 1 - Math.abs(mean - 200) / 200) * 0.25
    return Math.round((stability + continuity + speedScore) * 100)
  }

  function streamReflection(ankyId: string, typingId: string) {
    log('reflection', 'streaming reflection for anky ' + ankyId)
    const es = new EventSource('/api/stream-reflection/' + ankyId)
    let raw = ''
    let msgId: string | null = null

    const safety = setTimeout(() => {
      es.close(); removeMsg(typingId)
      if (!msgId) addMsg('anky', raw || 'anky is still reflecting.')
    }, 90000)

    es.onmessage = (e) => {
      if (e.data === 'keep-alive') return
      removeMsg(typingId)
      raw += e.data
      if (!msgId) { msgId = addMsg('anky', raw.trim()) }
      else { updateMsg(msgId, raw.trim()) }
    }

    es.addEventListener('done', () => {
      clearTimeout(safety); es.close()
      if (msgId) updateMsg(msgId, raw.trim())
      startBirthPoll(ankyId)
    })

    es.onerror = () => {
      clearTimeout(safety); es.close(); removeMsg(typingId)
      if (!raw) addMsg('anky', "connection dropped. your writing is saved.")
      startBirthPoll(ankyId)
    }
  }

  function startBirthPoll(ankyId: string) {
    const birthId = addMsg('birth', '', { birth: { ankyId, status: 'generating' } })
    log('birth', 'polling for anky image...', ankyId)

    const poll = setInterval(async () => {
      try {
        const r = await fetch(`/api/anky/${ankyId}/birth`, { credentials: 'include' })
        if (!r.ok) return
        const data = await r.json()
        if (data.status === 'complete' && data.imageUrl) {
          clearInterval(poll)
          log('birth', 'image ready!', data.imageUrl)
          setMessages(prev => prev.map(m =>
            m.id === birthId
              ? { ...m, birth: { ankyId, imageUrl: data.imageUrl, status: 'complete' as const } }
              : m
          ))
          loadProfile()
        }
      } catch {}
    }, 3000)

    // Stop polling after 3 minutes
    setTimeout(() => {
      clearInterval(poll)
      setMessages(prev => prev.map(m =>
        m.id === birthId && m.birth?.status === 'generating'
          ? { ...m, birth: { ankyId, status: 'complete' as const } }
          : m
      ))
      loadProfile()
    }, 180000)
  }

  // ── Input handling ──

  function handleKeyDown(e: React.KeyboardEvent) {
    const m = modeRef.current

    // Chat mode: Enter sends, everything else is normal
    if (m === 'chat') {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        sendChatMessage()
        return
      }
      return
    }

    // Anky mode: no backspace, no enter, no delete
    if (m === 'anky') {
      if (e.key === 'Backspace' || e.key === 'Delete' || e.key === 'Enter') {
        e.preventDefault()
        return
      }
      return
    }

    // Paused: Enter sends, typing resumes
    if (m === 'paused') {
      if (e.key === 'Enter') { e.preventDefault(); submitAnky(); return }
      if (e.key === 'Backspace' || e.key === 'Delete') { e.preventDefault(); return }
    }
  }

  function handleBeforeInput(e: React.FormEvent<HTMLTextAreaElement>) {
    const m = modeRef.current
    if (m === 'chat') return

    const nativeEvent = e.nativeEvent as InputEvent
    const inputType = nativeEvent.inputType || ''
    const el = e.currentTarget
    const atEnd = el.selectionStart === el.value.length && el.selectionEnd === el.value.length
    const blocks =
      inputType === 'insertLineBreak' ||
      inputType === 'insertParagraph' ||
      inputType === 'insertFromPaste' ||
      inputType === 'insertFromDrop' ||
      inputType === 'historyUndo' ||
      inputType === 'historyRedo' ||
      inputType.startsWith('delete') ||
      !inputType.startsWith('insert') ||
      !atEnd

    if (blocks) {
      e.preventDefault()
      requestAnimationFrame(() => {
        const end = el.value.length
        el.setSelectionRange(end, end)
      })
      return
    }

    if (m === 'paused') resumeFromPause()
  }

  function keepCaretAtEnd(e: React.SyntheticEvent<HTMLTextAreaElement>) {
    if (modeRef.current === 'chat') return
    const el = e.currentTarget
    requestAnimationFrame(() => {
      const end = el.value.length
      el.setSelectionRange(end, end)
    })
  }

  function handleInput(e: React.FormEvent<HTMLTextAreaElement>) {
    let val = e.currentTarget.value
    const m = modeRef.current

    if (m === 'chat') {
      setText(val)
      textRef.current = val
      if (!val.trim()) {
        stopTicking()
        typingStartRef.current = 0
        lastInputRef.current = 0
        setActiveTime(0)
        setIdleProgress(0)
        pausedAtRef.current = 0
        keystrokesRef.current = []
        lastKeystrokeRef.current = 0
        strictValueRef.current = ''
        return
      }

      recordInputActivity()
      strictValueRef.current = val
      return
    }

    if (val.includes('\n')) val = val.replace(/\n/g, ' ')

    if (val.length < strictValueRef.current.length || !val.startsWith(strictValueRef.current)) {
      val = strictValueRef.current
    }

    if (m === 'paused' && val.length > strictValueRef.current.length) {
      resumeFromPause()
    }

    if (val !== strictValueRef.current) {
      recordInputActivity()
      setIdleProgress(0)
      strictValueRef.current = val
    }

    setText(val)
    textRef.current = val
  }

  // ── Timer display ──
  const remaining = SESSION_DURATION - activeTime
  const timerText = remaining >= 0
    ? `${Math.floor(remaining / 60)}:${String(Math.floor(remaining % 60)).padStart(2, '0')}`
    : `+${Math.floor(Math.abs(remaining) / 60)}:${String(Math.floor(Math.abs(remaining) % 60)).padStart(2, '0')}`

  const chakraProgress = Math.min(activeTime / SESSION_DURATION, 1)
  const colorIdx = Math.min(Math.floor(chakraProgress * 8), 7)
  const stops = CHAKRA_COLORS.slice(0, colorIdx + 1)
  const chakraBg = stops.length === 1 ? stops[0] : `linear-gradient(to right, ${stops.join(', ')})`

  const isAnkyMode = mode === 'anky' || mode === 'paused'

  // ── Init ──

  useEffect(() => {
    if (initializedRef.current) return
    initializedRef.current = true

    async function init() {
      log('init', 'booting...')
      let fcUser: FarcasterUser | null = null

      try {
        log('init', 'loading farcaster sdk...')
        const mod = await import('https://esm.sh/@farcaster/miniapp-sdk')
        sdkRef.current = mod.sdk
        mod.sdk.actions.ready({ disableNativeGestures: true })
        const ctx = await mod.sdk.context
        log('init', 'farcaster context:', { fid: ctx?.user?.fid, username: ctx?.user?.username, displayName: ctx?.user?.displayName })
        if (ctx?.user?.fid) {
          fcUser = { fid: ctx.user.fid, username: ctx.user.username, displayName: ctx.user.displayName, pfpUrl: ctx.user.pfpUrl }
          setUser(fcUser)
          if (ctx.user.pfpUrl) setPfp(ctx.user.pfpUrl)
          try {
            log('auth', 'authenticating farcaster user fid=' + fcUser.fid)
            const r = await fetch('/auth/farcaster/verify', { method: 'POST', headers: { 'Content-Type': 'application/json' }, credentials: 'include', body: JSON.stringify({ fid: fcUser.fid, username: fcUser.username, pfp_url: fcUser.pfpUrl }) })
            const d = await r.json()
            log('auth', 'result:', d)
            if (d.ok) { setIsAuthenticated(true); if (d.wallet_address) setWalletAddress(d.wallet_address) }
          } catch (e) { log('auth', 'farcaster auth failed:', e) }
        }
        if (ctx?.client?.notificationDetails && fcUser) {
          log('init', 'saving notification token')
          fetch('/api/miniapp/notifications', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ fid: fcUser.fid, token: ctx.client.notificationDetails.token, url: ctx.client.notificationDetails.url }) }).catch(() => {})
        }
      } catch {
        log('init', 'not in farcaster — checking browser session...')
        try {
          const r = await fetch('/api/me', { credentials: 'include' })
          const me = await r.json()
          log('auth', 'browser session:', me.ok ? { user_id: me.user_id, username: me.username, wallet: me.wallet_address || me.solana_address } : 'not authenticated')
          if (me.ok) {
            setIsAuthenticated(true)
            setWalletAddress(me.wallet_address || me.solana_address || null)
            if (me.display_name) setProfileName(me.display_name)
            if (me.profile_image_url) setPfp(me.profile_image_url)
            if (me.username) { fcUser = { fid: 0, username: me.username, displayName: me.display_name, pfpUrl: me.profile_image_url }; setUser(fcUser) }
          }
        } catch (e) { log('auth', 'session check failed:', e) }
      }

      startChat(fcUser)
    }
    init()
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  // ── Chat flow ──

  async function startChat(fcUser: FarcasterUser | null) {
    let isOnboarded = false
    if (fcUser && fcUser.fid > 0) {
      try {
        const r = await fetch('/api/miniapp/onboarding?fid=' + fcUser.fid)
        const d = await r.json()
        if (d.onboarded) { isOnboarded = true; if (d.solana_address) setWalletAddress(d.solana_address) }
      } catch {}
    }

    if (isOnboarded && fcUser) {
      let prompt: string | null = null
      try { const r = await fetch('/api/miniapp/prompt?fid=' + fcUser.fid); const d = await r.json(); if (d.prompt) prompt = d.prompt } catch {}
      await wait(400)
      const name = fcUser.username || fcUser.displayName || 'you'
      addMsg('anky', prompt ? `@${name}. ${prompt}` : `hey @${name}. what's on your mind?`)
    } else {
      await wait(400)
      const name = fcUser?.username || fcUser?.displayName || null
      addMsg('anky', name ? `hi @${name}. what's on your mind?` : `hi. what's on your mind?`)

      if (fcUser && fcUser.fid > 0) {
        await wait(800)
        addMsg('typing', '')
        await generateMirror(fcUser)
      }
    }
  }

  async function generateMirror(fcUser: FarcasterUser) {
    const kingdomId = fcUser.fid % 8
    const kingdom = KINGDOMS[kingdomId]
    fetch('/api/miniapp/onboard', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ fid: fcUser.fid, kingdom_id: kingdomId }) }).catch(() => {})

    try {
      const resp = await fetch('/api/mirror?fid=' + fcUser.fid)
      const data = await resp.json()
      // remove typing indicator
      setMessages(prev => prev.filter(m => m.type !== 'typing'))

      if (data.error) return

      const imageUrl = data.anky_image_b64 ? `data:${data.anky_image_mime || 'image/png'};base64,${data.anky_image_b64}` : undefined
      addMsg('mirror', '', { mirror: { imageUrl, text: data.public_mirror, gap: data.gap, kingdom: `${kingdom.name} · ${kingdom.chakra}` } })
    } catch {
      setMessages(prev => prev.filter(m => m.type !== 'typing'))
    }
  }

  // ── Connect ──

  async function connectPhone() {
    log('connect', 'creating qr auth challenge...')
    setConnecting(true)
    try {
      const resp = await fetch('/api/auth/qr', { method: 'POST', credentials: 'include' })
      const data = await resp.json()
      log('connect', 'challenge created:', { id: data.id, deeplink: data.app_scheme_url })
      if (data.app_scheme_url) window.location.href = data.app_scheme_url
      if (data.id) {
        const poll = setInterval(async () => {
          try {
            const sr = await fetch('/api/auth/qr/' + data.id, { credentials: 'include' })
            const status = await sr.json()
            if (status.sealed && status.session_token) {
              clearInterval(poll); setIsAuthenticated(true)
              if (status.solana_address) setWalletAddress(status.solana_address)
              setConnecting(false); window.location.reload()
            }
          } catch {}
        }, 2000)
        setTimeout(() => { clearInterval(poll); setConnecting(false) }, 300000)
      }
    } catch { setConnecting(false) }
  }

  // ── Profile / Altar ──

  async function loadProfile() {
    try { const r = await fetch('/api/my-ankys', { credentials: 'include' }); if (r.ok) { const d = await r.json(); setAnkys(d || []); if (d?.length > 0 && d[0].imageUrl) setPfp(d[0].imageUrl) } } catch {}
    try {
      const r = await fetch('/api/me', { credentials: 'include' }); const p = await r.json()
      if (p?.ok) { setIsAuthenticated(true); if (p.display_name) setProfileName(p.display_name); if (p.bio) setProfileBio(p.bio); if (p.profile_image_url) setPfp(p.profile_image_url); if (p.wallet_address || p.solana_address) setWalletAddress(p.wallet_address || p.solana_address); setAnkyCount(Number(p.total_ankys || 0)) }
    } catch {}
  }


  // ── Render ──

  return (
    <div className={`app${isAnkyMode ? ' anky-mode' : ''}`}>
      <nav className="nav">
        <button className="nav-btn" onClick={() => window.open('/altar', '_blank')}>&#9764;</button>
        <span className="nav-title" onClick={() => { setView('chat'); runCommand('/now') }} style={{ cursor: 'pointer' }}>anky</span>
        <img className="nav-pfp" src={pfp} alt="you" onClick={() => { setView('profile'); loadProfile() }} />
      </nav>

      {/* Chat */}
      <div className="chat-wrap" style={{ display: view === 'chat' ? 'flex' : 'none' }}>
        <div className="chat-messages" ref={chatRef}>
          {messages.map(msg => <ChatMessage key={msg.id} msg={msg} setMessages={setMessages} onCommand={(v: string) => {
            if (v.startsWith('_btn:')) { handleButtonClick(v.slice(5)); return }
            runCommand(v)
          }} />)}
        </div>

        {/* Input — textarea */}
        <div className="input-bar">
          <textarea
            ref={inputRef}
            className="input-field"
            value={text}
            onKeyDown={handleKeyDown}
            onBeforeInput={handleBeforeInput}
            onInput={handleInput}
            onClick={keepCaretAtEnd}
            onFocus={keepCaretAtEnd}
            onPaste={isAnkyMode ? (e => e.preventDefault()) : undefined}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            placeholder="message anky..."
            rows={1}
          />
          {!isAnkyMode && mode === 'chat' && text.trim() ? (
            <button className="send-btn" onClick={sendChatMessage}>
              <svg viewBox="0 0 24 24" fill="currentColor"><path d="M2.01 21L23 12 2.01 3 2 10l15 2-15 2z"/></svg>
            </button>
          ) : null}
        </div>

        {/* Anky mode: progress bars + timer + send at bottom */}
        {isAnkyMode && (
          <div className="anky-bar">
            {idleProgress > 0 && <div className="idle-strip" style={{ width: `${idleProgress * 100}%` }} />}
            <div className="chakra-bar"><div className="chakra-fill" style={{ width: `${chakraProgress * 100}%`, background: chakraBg }} /></div>
            <span className={`input-timer${remaining < 0 ? ' past-zero' : ''}`}>{timerText}</span>
            {mode === 'paused' && (
              <button className="send-btn" onClick={submitAnky}>send</button>
            )}
          </div>
        )}
      </div>

      {/* Profile */}
      {view === 'profile' && (
        <div className="overlay">
          <div className="overlay-nav"><button className="overlay-back" onClick={() => setView('chat')}>&larr;</button><span className="overlay-title">you</span></div>
          <div className="overlay-body">
            <div className="profile-head">
              <img className="profile-pfp" src={pfp} alt="" />
              <div><div className="profile-name">{user?.displayName || profileName}</div><div className="profile-meta">{ankyCount} {ankyCount === 1 ? 'anky' : 'ankys'}</div></div>
            </div>
            <div className="identity-card">
              {walletAddress ? (
                <><div className="identity-row"><span className="identity-label">solana</span><span className="identity-addr">{walletAddress.slice(0, 4)}...{walletAddress.slice(-4)}</span></div><div className="identity-status connected"><span className="identity-dot connected" />connected</div></>
              ) : (
                <><div className="identity-status"><span className="identity-dot disconnected" />not connected</div><button className="connect-btn" onClick={connectPhone} disabled={connecting}>{connecting ? 'waiting for phone...' : 'connect with anky app'}</button></>
              )}
            </div>
            {profileBio && <div className="profile-bio">{profileBio}</div>}
            <div className="ankys-label">your ankys</div>
            <div className="ankys-grid">
              {ankys.map(a => (<div key={a.id} className="anky-tile" onClick={() => { setSelectedAnky(a); setView('convo') }}><img src={a.imageUrl || '/static/icon-192.png'} alt="" loading="lazy" /><div className="anky-tile-title">{a.title || 'untitled'}</div></div>))}
              {ankys.length === 0 && <div className="empty-state">no ankys yet</div>}
            </div>
          </div>
        </div>
      )}

      {/* Conversation */}
      {view === 'convo' && selectedAnky && (
        <div className="overlay">
          <div className="overlay-nav"><button className="overlay-back" onClick={() => setView('profile')}>&larr;</button><span className="overlay-title">{selectedAnky.title || 'untitled'}</span></div>
          <div className="overlay-body">
            <div className="convo-messages">
              {selectedAnky.writing && <div className="msg msg-user" style={{ maxHeight: 'none' }}>{selectedAnky.writing}</div>}
              {selectedAnky.imageUrl && <div className="msg-image"><img src={selectedAnky.imageUrl} alt="" /></div>}
              {selectedAnky.reflection?.split('\n').filter(l => l.trim()).map((line, i) => <div key={i} className="msg msg-anky">{line}</div>)}
            </div>
          </div>
        </div>
      )}

    </div>
  )
}

function ChatMessage({ msg, setMessages, onCommand }: { msg: Message; setMessages: React.Dispatch<React.SetStateAction<Message[]>>; onCommand?: (cmd: string) => void }) {
  if (msg.type === 'typing') return <div className="typing"><div className="typing-dot" /><div className="typing-dot" /><div className="typing-dot" /></div>

  if (msg.type === 'commands' && onCommand) {
    return (
      <div className="cmd-row">
        {['/now', '/prompt', '/generate', '/gallery', '/profile'].map(cmd => (
          <button key={cmd} className="cmd-btn" onClick={() => onCommand(cmd)}>{cmd}</button>
        ))}
      </div>
    )
  }

  if (msg.type === 'buttons' && msg.buttons && onCommand) {
    return (
      <div className="cmd-row">
        {msg.buttons.map((b, i) => (
          <button key={i} className="cmd-btn" onClick={() => {
            if (b.value.startsWith('_copy:')) {
              navigator.clipboard.writeText(b.value.slice(6))
              return
            }
            if (b.value.startsWith('_dl:')) {
              const parts = b.value.slice(4).split(':::')
              const blob = new Blob([parts[0]], { type: 'image/svg+xml' })
              const url = URL.createObjectURL(blob)
              const a = document.createElement('a')
              a.href = url; a.download = `anky-now-${parts[1] || 'qr'}.svg`; a.click()
              URL.revokeObjectURL(url)
              return
            }
            onCommand('_btn:' + b.value)
          }}>{b.label}</button>
        ))}
      </div>
    )
  }

  if (msg.type === 'qr' && msg.qr) {
    return (
      <div className="msg msg-anky" style={{ maxHeight: 'none', overflow: 'visible', textAlign: 'center' }}>
        <div className="now-qr-box" dangerouslySetInnerHTML={{ __html: msg.qr.svg }} />
        <div style={{ marginTop: 8, fontSize: 14 }}>
          <a href={msg.qr.url} target="_blank" rel="noopener" style={{ color: '#b366ff', wordBreak: 'break-all' as const }}>{msg.qr.url}</a>
        </div>
      </div>
    )
  }

  if (msg.type === 'system' && msg.content === '_gallery' && msg.buttons) {
    return (
      <div className="gallery-grid">
        {msg.buttons.map((a, i) => (
          <div key={i} className="gallery-tile" onClick={() => window.open(a.value, '_blank')}>
            <img src={a.value} alt={a.label} loading="lazy" />
            <div className="gallery-tile-title">{a.label}</div>
          </div>
        ))}
      </div>
    )
  }

  if (msg.type === 'map') {
    return <MapPicker initialCenter={msg.content} onConfirm={(lat, lng) => {
      setMessages(prev => prev.filter(m => m.id !== msg.id))
      if (onCommand) onCommand(`_btn:${lat},${lng}`)
    }} />
  }

  if (msg.type === 'timepicker') {
    return <TimePicker onConfirm={(mins) => {
      setMessages(prev => prev.filter(m => m.id !== msg.id))
      if (onCommand) onCommand(`_btn:${mins}`)
    }} />
  }

  if (msg.type === 'mirror' && msg.mirror) {
    return (
      <div className="mirror-card">
        {msg.mirror.imageUrl && <img src={msg.mirror.imageUrl} alt="your mirror" />}
        <div className="mirror-card-body">
          {msg.mirror.text && <div className="mirror-card-text">{msg.mirror.text}</div>}
          {msg.mirror.gap && <div className="mirror-card-gap">{msg.mirror.gap}</div>}
          {msg.mirror.kingdom && <div className="mirror-card-kingdom">{msg.mirror.kingdom}</div>}
        </div>
      </div>
    )
  }

  if (msg.type === 'birth' && msg.birth) {
    if (msg.birth.status === 'generating') {
      return (
        <div className="anky-birth">
          <div className="birth-orb" />
          <div className="birth-text">your anky is being born...</div>
        </div>
      )
    }
    if (msg.birth.imageUrl) {
      return (
        <div className="anky-birth complete">
          <img className="birth-image" src={msg.birth.imageUrl} alt="your anky" />
        </div>
      )
    }
    return null
  }

  if (msg.type === 'image') return <div className="msg-image"><img src={msg.content} alt="" /></div>

  return (
    <div
      className={`msg msg-${msg.type}${msg.type === 'user' && msg.expanded ? ' expanded' : ''}`}
      onClick={msg.type === 'user' ? () => setMessages(prev => prev.map(m => m.id === msg.id ? { ...m, expanded: !m.expanded } : m)) : undefined}
    >
      {msg.content}
    </div>
  )
}

function MapPicker({ initialCenter, onConfirm }: { initialCenter: string; onConfirm: (lat: number, lng: number) => void }) {
  const mapRef = useRef<HTMLDivElement>(null)
  const mapInstanceRef = useRef<L.Map | null>(null)
  const markerRef = useRef<L.Marker | null>(null)
  const [coords, setCoords] = useState<[number, number]>(() => {
    const parts = initialCenter.split(',').map(Number)
    return parts.length >= 2 && !isNaN(parts[0]) && !isNaN(parts[1]) ? [parts[0], parts[1]] : [0, 0]
  })

  useEffect(() => {
    if (!mapRef.current || mapInstanceRef.current) return
    const map = L.map(mapRef.current, { zoomControl: true, attributionControl: false }).setView(coords, 14)
    L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', { maxZoom: 19 }).addTo(map)

    // Custom small marker icon
    const icon = L.divIcon({ className: 'map-pin', html: '<div style="width:20px;height:20px;background:#b366ff;border:3px solid #fff;border-radius:50%;box-shadow:0 2px 8px rgba(0,0,0,0.4)"></div>', iconSize: [20, 20], iconAnchor: [10, 10] })
    const marker = L.marker(coords, { draggable: true, icon }).addTo(map)

    marker.on('dragend', () => {
      const pos = marker.getLatLng()
      setCoords([pos.lat, pos.lng])
    })

    map.on('click', (e: L.LeafletMouseEvent) => {
      marker.setLatLng(e.latlng)
      setCoords([e.latlng.lat, e.latlng.lng])
    })

    mapInstanceRef.current = map
    markerRef.current = marker

    // Fix tile rendering after mount
    setTimeout(() => map.invalidateSize(), 100)

    return () => { map.remove(); mapInstanceRef.current = null }
  }, []) // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="map-picker">
      <div ref={mapRef} style={{ width: '100%', height: 200, borderRadius: 12, overflow: 'hidden' }} />
      <div className="map-picker-footer">
        <span className="map-picker-coords">{coords[0].toFixed(5)}, {coords[1].toFixed(5)}</span>
        <button className="cmd-btn" onClick={() => onConfirm(coords[0], coords[1])}>confirm</button>
      </div>
    </div>
  )
}

function TimePicker({ onConfirm }: { onConfirm: (mins: number) => void }) {
  const [value, setValue] = useState('')
  const now = new Date()
  const minStr = new Date(now.getTime() + 60000).toISOString().slice(0, 16) // at least 1 min from now

  return (
    <div className="time-picker">
      <input
        type="datetime-local"
        className="time-picker-input"
        min={minStr}
        value={value}
        onChange={e => setValue(e.target.value)}
      />
      <button
        className="cmd-btn"
        disabled={!value}
        onClick={() => {
          const target = new Date(value).getTime()
          const mins = Math.max(1, Math.round((target - Date.now()) / 60000))
          onConfirm(mins)
        }}
      >
        confirm
      </button>
    </div>
  )
}
