<script lang="ts">
  import { emit, listen } from 'tauri/api/event'
  import { writable } from 'svelte/store'
  import type { Writable } from 'svelte/store'
  import type { Emission, Model, Reception, Item, Channel } from './models'
  import { subscribe, Action, arrayToIdMap } from './models'
  import { fromNullable, map, getOrElse } from 'fp-ts/lib/Option'
  import { pipe } from 'fp-ts/lib/function'

  const state: Writable<Model> = writable({
    items: new Map(),
    channels: new Map(),
  })
  const itemsToState = (data: Reception<Item>) =>
    state.update((s) => ({ ...s, items: arrayToIdMap(data.payload) }))
  const channelsToState = (data: Reception<Channel>) => {
    state.update((s) => ({ ...s, channels: arrayToIdMap(data.payload) }))
  }
  const feedTitleFromId = (id: number): string =>
    pipe(
      fromNullable($state.channels.get(id)),
      map((channel) => channel.title),
      getOrElse(() => '')
    )

  listen('subscribed', console.log)
  listen('allChannels', channelsToState)
  listen('allItems', itemsToState)
  listen('rust-error', console.log)

  const emitToBackend = (emission: Emission) =>
    emit('', JSON.stringify(emission))

  let newChannelUrl = ''
</script>

<style>
  ul {
    margin: var(--s1);
    max-width: max-content;
  }
  summary::marker,
  summary::-webkit-details-marker {
    display: none;
  }
  details {
    border: var(--s-5) solid var(--light3);
    padding: var(--s1);
  }
</style>

<main>
  <input type="text" bind:value={newChannelUrl} placeholder="new feed url" />
  <button on:click={() => emitToBackend(subscribe(newChannelUrl))}>
    Subscribe
  </button>
  <ul class="stack">
    {#each Array.from($state.items) as [id, item] (id)}
      <li>
        <details>
          <summary>
            <h3>{item.title}</h3>
            <h4>
              {feedTitleFromId(item.feed_id)} ––
              <span class="date">{item.date}</span>
            </h4>
          </summary>
          <div>
            {@html item.content}
          </div>
        </details>
      </li>
    {/each}
  </ul>
</main>
