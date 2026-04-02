import { fetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'

const GITHUB_REPO = 'Shapooo/weiback-rs'
const RELEASE_URL = `https://github.com/${GITHUB_REPO}/releases/latest`
export const PROJECT_URL = `https://github.com/${GITHUB_REPO}`

export interface ReleaseInfo {
  tag_name: string
  html_url: string
  body: string
  published_at: string
}

export const checkLatestRelease = async (): Promise<ReleaseInfo | null> => {
  try {
    const res = await fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`, {
      method: 'GET',
      headers: { Accept: 'application/vnd.github+json' },
    })
    if (!res.ok) return null
    const data = await res.json()
    return data as ReleaseInfo
  } catch {
    return null
  }
}

export const openReleasePage = () => openUrl(RELEASE_URL)
export const openProjectPage = () => openUrl(PROJECT_URL)
