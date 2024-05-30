export type Settings = {
  paths: string[]
}

export const useSettings = (): Settings => {
  const settings = document.querySelector('script#settings') as HTMLScriptElement
  const settingsJson = settings.innerHTML
  const settingsObj = JSON.parse(settingsJson) as Settings
  return settingsObj
}
