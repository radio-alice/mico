import type { Model, Reception, Input, Id, Item, Channel } from './models'
import type { Writable } from 'svelte/store'
import { writable } from 'svelte/store'

const createStore = () => {
  const { subscribe, set, update }: Writable<Model> = writable({
    items: new Map(),
    channels: new Map(),
  })

  return {
    subscribe,

    itemsToState: (Items: Reception<Input<Item>[]>) =>
      update((s) => ({ ...s, items: arrayToIdMap(Items.payload) })),

    channelsToState: (channels: Reception<Input<Channel>[]>) =>
      update((s) => ({ ...s, channels: arrayToIdMap(channels.payload) })),

    newChannelToState: (newChannel: Reception<Input<Channel>>) =>
      update((s) => ({
        ...s,
        channels: s.channels.set(...objectToIdTuple(newChannel.payload)),
      })),

    newItemsToState: (newItems: Reception<Input<Item>[]>) =>
      update((s) => ({
        ...s,
        items: new Map([...s.items, ...arrayToIdMap(newItems.payload)]),
      })),

    openItem: (itemId: Id) =>
      update((s) => ({
        ...s,
        openItem: itemId,
      })),
  }
}

const objectToIdTuple = <T>(input: Input<T>): [Id, T] => [
  input.id,
  { ...input },
]
const arrayToIdMap = <T>(inputs: Array<Input<T>>) =>
  new Map<Id, T>(inputs.map(objectToIdTuple))

export default createStore()
