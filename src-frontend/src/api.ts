import { emit } from 'tauri/api/event'
import type { Emission } from './models'

export const emitToBackend = (emission: Emission) =>
  emit('', JSON.stringify(emission))
