import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { initializeTransport } from '$lib/api'

// Initialize transport BEFORE mounting app
// This ensures Tauri IPC is ready when components make API calls
initializeTransport().then(() => {
  const app = mount(App, {
    target: document.getElementById('app')!,
  })

  // @ts-expect-error - exported for HMR
  window.__app = app
}).catch((err) => {
  console.error('Failed to initialize transport:', err)
  // Mount app anyway - will fall back to HTTP transport
  mount(App, {
    target: document.getElementById('app')!,
  })
})
