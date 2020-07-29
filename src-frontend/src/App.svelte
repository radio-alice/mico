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
  import { Event, externalLink, subscribe } from './models'
  import { fromNullable, map, getOrElse } from 'fp-ts/lib/Option'
  import { pipe } from 'fp-ts/lib/function'
  import toast from './toast'
  import Toast from './Toast.svelte'
  import store from './store'
  import OpenItem from './OpenItem.svelte'
  const {
    channelsToState,
    itemsToState,
    newChannelToState,
    newItemsToState,
    openItem,
  } = store

  const handleError = (err: Reception<string>) =>
    err.payload.startsWith('CouldntResolveHost')
      ? toast.trigger('ur offline! I think!', true)
      : toast.trigger(err.payload, true)

  listen('subscribed', console.log)
  listen(Event.AllChannels, channelsToState)
  listen(Event.AllItems, itemsToState)
  listen(Event.NewChannel, newChannelToState)
  listen(Event.NewItems, newItemsToState)
  listen('rust-error', handleError)

  const emitToBackend = (emission: Emission) =>
    emit('', JSON.stringify(emission))

  const feedTitleFromId = (id: number): string =>
    pipe(
      fromNullable($store.channels.get(id)),
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
  $: itemsList = Array.from($store.items).sort(orderByDate)
  $: currentOpenItem = $store.items.get($store.openItem)
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
    position: absolute;
    width: 100%;
  }
  ul {
    flex-basis: 20rem;
    flex-grow: 1;
    align-items: stretch;
    overflow-y: scroll;
    max-height: 100vh;
  }
  li.item {
    min-height: max-content;
    border-bottom: var(--s-5) solid var(--light3);
    padding: var(--s0);
    cursor: pointer;
    font-size: var(--s-1);
  }
  .item.title {
    font-size: var(--s0);
  }
  .item.stack > * {
    --space: var(--s-6);
  }
  .item.divider {
    margin: 0 var(--s0);
  }
  .item.date {
    font-style: italic;
  }
  .placeholder {
    flex: 2;
    min-width: var(--measure);
  }
</style>

<svelte:window on:click={openLinksInBrowser} />
<main>
  <input type="text" bind:value={newChannelUrl} placeholder="new feed url" />
  <button on:click={() => emitToBackend(subscribe(newChannelUrl))}>
    Subscribe
  </button>
  <div class="row">
    <ul class="stack">
      {#each itemsList as [id, item] (id)}
        <li on:click={() => openItem(id)} class="item stack">
          <p class="item title">{item.title}</p>
          <p>{feedTitleFromId(item.feed_id)}</p>
          <p>
            <span class="item date">{item.date}</span>
            <span class="item divider">⚉</span>
            <span>
              <a href={item.url}>link</a>
            </span>
          </p>
        </li>
      {/each}
    </ul>
    {#if currentOpenItem}
      <OpenItem
        url={currentOpenItem.url}
        content={currentOpenItem.content}
        title={currentOpenItem.title} />
    {:else}
      <div class="placeholder" />
    {/if}
  </div>
</main>
<Toast />
