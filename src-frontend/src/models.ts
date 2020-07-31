export enum Action {
  Subscribe = 'subscribe',
  ExternalLink = 'externalLink',
  Unsubscribe = 'unsubscribe',
  Resubscribe = 'resubscribe',
}
export const unsubscribe = (id: Id) => ({ cmd: Action.Unsubscribe, id })
export const resubscribe = (id: Id) => ({ cmd: Action.Resubscribe, id })
export const subscribe = (url: string) => ({ cmd: Action.Subscribe, url })
export const externalLink = (url: string) => ({ cmd: Action.ExternalLink, url })
export interface Emission {
  cmd: Action
}
export interface Reception<T> {
  type: Event
  payload: T
}
export enum Event {
  AllChannels = 'allChannels',
  AllItems = 'allItems',
  NewChannel = 'newChannel',
  NewItems = 'newItems',
  Unsubscribe = 'unsubscribed',
}
export interface Channel {
  url: string
  date: string
  title: string
  subscribed: boolean
}
export interface Item {
  url?: string
  feed_id: number
  read: boolean
  date: string
  content: string
  title: string
}
export interface Model {
  items: Map<Id, Item>
  channels: Map<Id, Channel>
  openItem?: Id
}
export type Id = number
export type Input<T> = T & {
  id: Id
}
