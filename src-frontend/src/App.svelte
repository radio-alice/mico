<script lang="ts">
  import { emit, listen } from 'tauri/api/event'
  import { writable } from 'svelte/store'
  import type { Writable } from 'svelte/store'
  import type {
    Emission,
    Model,
    Reception,
    Item,
    Channel,
    Input,
  } from './models'
  import {
    Event,
    externalLink,
    subscribe,
    arrayToIdMap,
    objectToIdTuple,
  } from './models'
  import { fromNullable, map, getOrElse } from 'fp-ts/lib/Option'
  import { pipe } from 'fp-ts/lib/function'

  const state: Writable<Model> = writable({
    items: new Map(),
    channels: new Map(),
  })
  const itemsToState = (data: Reception<Input<Item>[]>) =>
    state.update((s) => ({ ...s, items: arrayToIdMap(data.payload) }))
  const channelsToState = (data: Reception<Input<Channel>[]>) => {
    state.update((s) => ({ ...s, channels: arrayToIdMap(data.payload) }))
  }
  const newChannelToState = (data: Reception<Input<Channel>>) =>
    state.update((s) => ({
      ...s,
      channels: s.channels.set(...objectToIdTuple(data.payload)),
    }))
  const newItemsToState = (data: Reception<Input<Item>[]>) => {
    state.update((s) => ({
      ...s,
      items: new Map([...s.items, ...arrayToIdMap(data.payload)]),
    }))
  }
  const feedTitleFromId = (id: number): string =>
    pipe(
      fromNullable($state.channels.get(id)),
      map((channel) => channel.title),
      getOrElse(() => '')
    )
  const orderByDate = (itemA: [number, Item], itemB: [number, Item]) =>
    parseDate(itemB[1].date) - parseDate(itemA[1].date)

  // reorder date string so js can understand it
  const parseDate = (date: string) => {
    const mdy = date.split('-')
    return new Date([mdy[2], mdy[0], mdy[1]].join('-')).getTime()
  }
  listen('subscribed', console.log)
  listen(Event.AllChannels, channelsToState)
  listen(Event.AllItems, itemsToState)
  listen(Event.NewChannel, newChannelToState)
  listen(Event.NewItems, newItemsToState)
  listen('rust-error', console.error)

  const emitToBackend = (emission: Emission) =>
    emit('', JSON.stringify(emission))

  let newChannelUrl = ''
  const openLinksInBrowser = (event) => {
    if (
      event.target.tagName.toUpperCase() === 'A' &&
      event.target.href.startsWith('http')
    ) {
      event.preventDefault()
      emitToBackend(externalLink(event.target.href))
    }
  }
</script>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  ul {
    padding: var(--s1);
    width: max-content;
    align-items: stretch;
  }
  details {
    border: var(--s-5) solid var(--light3);
    padding: var(--s1);
  }
  .divider {
    margin: 0 var(--s0);
  }
</style>

<svelte:window on:click={openLinksInBrowser} />
<main>
  <input type="text" bind:value={newChannelUrl} placeholder="new feed url" />
  <button on:click={() => emitToBackend(subscribe(newChannelUrl))}>
    Subscribe
  </button>
  <ul class="stack">
    {#each Array.from($state.items).sort(orderByDate) as [id, item] (id)}
      <li>
        <details>
          <summary>
            <h3>{item.title}</h3>
            <h4>
              {feedTitleFromId(item.feed_id)}
              <span class="divider">⚉</span>
              <span class="date">{item.date}</span>
              <span class="divider">⚉</span>
              <span>
                <a href={item.url}>link</a>
              </span>
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
