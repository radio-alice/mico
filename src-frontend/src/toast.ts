import { writable } from 'svelte/store'

interface toastData {
  message: string
  error: boolean
  visible: boolean
}
function createToast() {
  const { subscribe, set, update } = writable({} as toastData)

  return {
    subscribe,
    trigger: (message, error) => {
      set({
        message,
        error,
        visible: true,
      })
      setTimeout(() => {
        update((toastVars) => ({ ...toastVars, visible: false }))
      }, 3500)
    },
    close: () => update((toastVars) => ({ ...toastVars, visible: false })),
  }
}

export default createToast()
