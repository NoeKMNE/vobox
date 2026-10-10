interface RecognitionResult {
  isFinal: boolean
  0: { transcript: string }
}

interface RecognitionEvent {
  resultIndex: number
  results: RecognitionResult[]
}

type RecognitionInstance = {
  lang: string
  continuous: boolean
  interimResults: boolean
  onresult: ((e: RecognitionEvent) => void) | null
  onend: (() => void) | null
  onerror: ((e: { error?: string }) => void) | null
  start(): void
  stop(): void
  abort(): void
}

function recognitionCtor(): (new () => RecognitionInstance) | null {
  const w = window as unknown as {
    SpeechRecognition?: new () => RecognitionInstance
    webkitSpeechRecognition?: new () => RecognitionInstance
  }
  return w.SpeechRecognition || w.webkitSpeechRecognition || null
}

export function isSttSupported(): boolean {
  return recognitionCtor() !== null
}

let rec: RecognitionInstance | null = null

export function startDictation(opts: {
  onInterim: (text: string) => void
  onFinal: (text: string) => void
  onEnd: () => void
  onError: (msg: string) => void
}): boolean {
  const Ctor = recognitionCtor()
  if (!Ctor) return false
  const instance = new Ctor()
  rec = instance
  instance.lang = 'fr-FR'
  instance.continuous = false
  instance.interimResults = true
  let finalText = ''
  instance.onresult = (e) => {
    let interim = ''
    for (let i = e.resultIndex; i < e.results.length; i++) {
      const res = e.results[i]
      if (res.isFinal) finalText += res[0].transcript
      else interim += res[0].transcript
    }
    opts.onInterim(finalText + interim)
  }
  instance.onend = () => {
    if (finalText) opts.onFinal(finalText)
    opts.onEnd()
  }
  instance.onerror = (e) => {
    opts.onError(e?.error ?? 'erreur_audio')
  }
  try {
    instance.start()
    return true
  } catch {
    return false
  }
}

export function stopDictation() {
  rec?.stop()
}

function pickFrVoice(): SpeechSynthesisVoice | null {
  if (!('speechSynthesis' in window)) return null
  const voices = window.speechSynthesis.getVoices()
  return (
    voices.find((v) => v.lang.toLowerCase().startsWith('fr') && v.localService) ??
    voices.find((v) => v.lang.toLowerCase().startsWith('fr')) ??
    null
  )
}

export function isTtsSupported(): boolean {
  return 'speechSynthesis' in window
}

export function speak(text: string, onEnd?: () => void): boolean {
  if (!isTtsSupported()) return false
  try {
    window.speechSynthesis.cancel()
    const utterance = new SpeechSynthesisUtterance(text)
    utterance.lang = 'fr-FR'
    const voice = pickFrVoice()
    if (voice) utterance.voice = voice
    utterance.rate = 1.05
    utterance.onend = () => onEnd?.()
    window.speechSynthesis.speak(utterance)
    return true
  } catch {
    return false
  }
}

export function stopSpeaking() {
  if (isTtsSupported()) window.speechSynthesis.cancel()
}