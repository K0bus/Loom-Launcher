import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useToastStore = defineStore('toast', () => {
  const visible = ref(false)
  const message = ref('')
  const type = ref<'success' | 'info' | 'error'>('success')
  let timer: any = null

  function show(msg: string, toastType: 'success' | 'info' | 'error' = 'success', duration = 3000) {
    message.value = msg
    type.value = toastType
    visible.value = true

    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      visible.value = false
    }, duration)
  }

  function hide() {
    visible.value = false
  }

  return {
    visible,
    message,
    type,
    show,
    hide
  }
})
