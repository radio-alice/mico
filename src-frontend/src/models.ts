export enum Action {
  GetChannels = 'getChannels',
  Subscribe = 'subscribe',
}
export interface Subscribe {
  cmd: Action.Subscribe
  url: string
}
export const subscribe = (url: string) => ({ cmd: Action.Subscribe, url })
export interface GetChannels {
  cmd: Action.GetChannels
}
export type Emission = Subscribe | GetChannels
export interface Reception<T> {
  type: Event
  payload: [T]
}
export enum Event {
  AllChannels = 'allChannels',
  AllItems = 'allItems',
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
}
type Id = number
type Input<T> = T & {
  id: Id
}
const objectToIdTuple = <T>(input: Input<T>): [Id, T] => [
  input.id,
  { ...input },
]
export const arrayToIdMap = <T>(inputs: Array<T>) =>
  new Map<Id, T>(inputs.map(objectToIdTuple))
