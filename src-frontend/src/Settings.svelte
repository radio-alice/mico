<script lang="ts">
  import { emitToBackend } from './api'
  import type { Channel } from './models'
  import { subscribe } from './models'
  export let channels: Map<number, Channel>
  let newChannelUrl = ''
</script>

<div class="main-content">
  <ul class="stack">
    <li>
      <input
        type="text"
        bind:value={newChannelUrl}
        placeholder="new feed url" />
      <button on:click={() => emitToBackend(subscribe(newChannelUrl))}>
        Subscribe
      </button>
    </li>
    {#each Array.from(channels) as [id, channel] (id)}
      <li>
        {channel.title}
        <button>unsubscribe</button>
      </li>
    {/each}
  </ul>
</div>
