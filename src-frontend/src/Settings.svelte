<script lang="ts">
  import { emitToBackend } from './api'
  import type { Channel } from './models'
  import { subscribe, unsubscribe, resubscribe } from './models'
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
        {#if channel.subscribed}
          <button on:click={() => emitToBackend(unsubscribe(id))}>
            unsubscribe
          </button>
        {:else}
          <button on:click={() => emitToBackend(resubscribe(id))}>
            re-subscribe
          </button>
        {/if}
      </li>
    {/each}
  </ul>
</div>
