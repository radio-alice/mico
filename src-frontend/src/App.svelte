<script lang="ts">
  import { emit, listen } from 'tauri/api/event'
  import { writable } from 'svelte/store'
  enum Action {
    GetChannels = 'getChannels',
    Subscribe = 'subscribe',
  }
  interface Subscribe {
    cmd: Action.Subscribe
    url: string
  }
  const subscribe = (url: string) => ({ cmd: Action.Subscribe, url })
  interface GetChannels {
    cmd: Action.GetChannels
  }
  type Emission = Subscribe | GetChannels
  const emitToBackend = (emission: Emission) =>
    emit('', JSON.stringify(emission))
  const log = (e) => backendInfo.set(JSON.stringify(e))
  listen('subscribed', log)
  listen('get-channels', log)
  listen('rust-error', log)
  let newChannelUrl = ''
  const backendInfo = writable('')
</script>

<main>
  <input type="text" bind:value={newChannelUrl} placeholder="new feed url" />
  <button on:click={() => emitToBackend(subscribe(newChannelUrl))}>
    Subscribe
  </button>
  <button on:click={() => emitToBackend({ cmd: Action.GetChannels })}>
    Get Feeds
  </button>
  <div>{$backendInfo}</div>
</main>
