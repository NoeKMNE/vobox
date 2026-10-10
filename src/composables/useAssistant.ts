import { invoke } from '@tauri-apps/api/core'

export interface AiMessage {
  role: 'user' | 'assistant'
  content: string
}

export interface ProposedAction {
  tool: string
  args: Record<string, unknown>
  label: string
}

export interface AgentResponse {
  reply: string
  actions: ProposedAction[]
  used_tools: string[]
  model: string
}

export interface AiSettings {
  ready: boolean
  model: string
}

export const KNOWN_MODELS = ['deepseek-chat', 'deepseek-reasoner']

export function getAiSettings(token: string): Promise<AiSettings> {
  return invoke<AiSettings>('get_ai_settings', { token })
}

export function saveAiSettings(
  token: string,
  opts: { model: string }
): Promise<void> {
  return invoke('save_ai_settings', {
    token,
    model: opts.model
  })
}

export function assistantChat(token: string, messages: AiMessage[]): Promise<AgentResponse> {
  return invoke<AgentResponse>('assistant_chat', { token, messages })
}