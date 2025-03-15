import { DownloadTaskEvent } from './bindings.ts'

export type CurrentTabName = 'search' | 'favorite' | 'downloaded' | 'comic'

export type ProgressData = DownloadTaskEvent & { percentage: number; indicator: string }
