import { create } from 'zustand'

export interface ReleaseInfo {
  tag_name: string
  html_url: string
  body: string
  published_at: string
}

interface UpdateStore {
  latestRelease: ReleaseInfo | null
  dismissedVersion: string | null
  checkCount: number
  lastChecked: number | null

  setLatestRelease: (release: ReleaseInfo) => void
  dismissVersion: (version: string) => void
  incrementCheckCount: () => void
  setLastChecked: (time: number) => void
}

export const useUpdateStore = create<UpdateStore>(set => ({
  latestRelease: null,
  dismissedVersion: null,
  checkCount: 0,
  lastChecked: null,

  setLatestRelease: release => set({ latestRelease: release }),
  dismissVersion: version => set({ dismissedVersion: version }),
  incrementCheckCount: () => set(state => ({ checkCount: state.checkCount + 1 })),
  setLastChecked: time => set({ lastChecked: time }),
}))
