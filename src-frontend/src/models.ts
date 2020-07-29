export enum Action {
  Subscribe = 'subscribe',
  ExternalLink = 'externalLink',
}
export interface Subscribe {
  cmd: Action.Subscribe
  url: string
}
export interface ExternalLink {
  cmd: Action.ExternalLink
  url: string
}
export const subscribe = (url: string) => ({ cmd: Action.Subscribe, url })
export const externalLink = (url: string) => ({ cmd: Action.ExternalLink, url })
export type Emission = Subscribe | ExternalLink
export interface Reception<T> {
  type: Event
  payload: T
}
export enum Event {
  AllChannels = 'allChannels',
  AllItems = 'allItems',
  NewChannel = 'newChannel',
  NewItems = 'newItems',
}
export interface Channel {
  url: string
  date: string
  title: string
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
