<script lang="ts">
  import { pipe } from 'fp-ts/lib/function'
  import { fromNullable, map, getOrElse } from 'fp-ts/lib/Option'
  import { listen } from 'tauri/api/event'
  import { emitToBackend } from './api'
  import type { Reception, Item, Channel, Input } from './models'
  import { Event, externalLink } from './models'
  import store from './store'
  import toast from './toast'
  import Toast from './Toast.svelte'
  import OpenItem from './OpenItem.svelte'
  import Settings from './Settings.svelte'
  const {
    channelsToState,
    itemsToState,
    newChannelToState,
    newItemsToState,
    openItem,
    unsubscribeFromChannel,
  } = store

  const handleError = (err: Reception<string>) =>
    err.payload.startsWith('CouldntResolveHost')
      ? toast.trigger('ur offline! I think!', true)
      : toast.trigger(err.payload, true)
  const unsubscribe = (id: Reception<number>) => {
    unsubscribeFromChannel(id.payload)
    toast.trigger(`unsubscribed from ${feedTitleFromId(id.payload)}`, false)
  }
  listen(Event.Unsubscribe, unsubscribe)
  listen(Event.AllChannels, channelsToState)
  listen(Event.AllItems, itemsToState)
  listen(Event.NewChannel, (data: Reception<Input<Channel>>) => {
    newChannelToState(data)
    toast.trigger(`subscribed to channel ${data.payload.title}`, false)
  })
  listen(Event.NewItems, newItemsToState)
  listen('rust-error', handleError)

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
  div.row {
    width: 100%;
  }
  .items {
    align-items: stretch;
    overflow-y: scroll;
    max-height: 100vh;
  }
  li.item {
    flex-grow: 1;
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
  .open {
    background-color: var(--dark4);
    color: var(--light1);
  }
</style>

<svelte:window on:click={openLinksInBrowser} />
<main>
  <div class="row">
    <ul class="items">
      <li
        class="item"
        on:click={() => openItem(null)}
        class:open={$store.openItem === null}>
        <p class="item title">Settings</p>
      </li>
      {#each itemsList as [id, item] (id)}
        <li
          on:mousedown={() => openItem(id)}
          class="item stack"
          class:open={$store.openItem === id}>
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
      <Settings channels={$store.channels} />
    {/if}
  </div>
</main>
<Toast />
